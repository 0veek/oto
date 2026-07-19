//! Microphone capture via cpal. Public API is consumed by the pipeline orchestrator.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use tauri::{AppHandle, Emitter};

use crate::audio::wav::write_wav_i16_mono;
use crate::error::{OtoError, OtoResult};
use crate::pipeline::events::PipelineEvent;

/// Live microphone capture session. Dropping or calling [`Self::stop`] ends the stream.
#[allow(dead_code)] // constructed by pipeline (Task 9+)
pub struct AudioRecorder {
    stream: Option<Stream>,
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
}

impl AudioRecorder {
    /// Open the default input device and start recording.
    ///
    /// Prefers mono when the device accepts a 1-channel stream; otherwise records
    /// multi-channel and downmixes to mono in the callback. Uses the device default
    /// sample rate (written into the WAV header; Whisper accepts common rates).
    pub fn start(app: AppHandle) -> OtoResult<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| OtoError::Message("no default input device".into()))?;

        let supported = device
            .default_input_config()
            .map_err(|e| OtoError::Message(format!("default input config: {e}")))?;

        let sample_format = supported.sample_format();
        let sample_rate = supported.sample_rate(); // cpal 0.18: SampleRate = u32
        let native_channels = supported.channels();

        let mut last_err = OtoError::Message("failed to open input stream".into());

        // Prefer mono; fall back to native channel count if mono fails to build.
        let attempts: Vec<(u16, StreamConfig)> = if native_channels == 1 {
            vec![(
                1,
                StreamConfig {
                    channels: 1,
                    sample_rate,
                    buffer_size: cpal::BufferSize::Default,
                },
            )]
        } else {
            vec![
                (
                    1,
                    StreamConfig {
                        channels: 1,
                        sample_rate,
                        buffer_size: cpal::BufferSize::Default,
                    },
                ),
                (
                    native_channels,
                    StreamConfig {
                        channels: native_channels,
                        sample_rate,
                        buffer_size: cpal::BufferSize::Default,
                    },
                ),
            ]
        };

        for (channels, config) in attempts {
            match Self::try_build_stream(&device, config, sample_format, channels, app.clone()) {
                Ok((stream, samples)) => {
                    stream
                        .play()
                        .map_err(|e| OtoError::Message(format!("stream play: {e}")))?;
                    return Ok(Self {
                        stream: Some(stream),
                        samples,
                        sample_rate,
                    });
                }
                Err(e) => last_err = e,
            }
        }

        Err(last_err)
    }

    fn try_build_stream(
        device: &cpal::Device,
        config: StreamConfig,
        sample_format: SampleFormat,
        channels: u16,
        app: AppHandle,
    ) -> OtoResult<(Stream, Arc<Mutex<Vec<i16>>>)> {
        let samples = Arc::new(Mutex::new(Vec::<i16>::new()));
        let err_fn = |err| eprintln!("audio input stream error: {err}");

        let stream = match sample_format {
            SampleFormat::F32 => {
                let samples_cb = Arc::clone(&samples);
                let app = app.clone();
                device
                    .build_input_stream(
                        config,
                        move |data: &[f32], _| {
                            process_f32(data, channels, &samples_cb, &app);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| OtoError::Message(format!("build input stream (f32): {e}")))?
            }
            SampleFormat::I16 => {
                let samples_cb = Arc::clone(&samples);
                let app = app.clone();
                device
                    .build_input_stream(
                        config,
                        move |data: &[i16], _| {
                            process_i16(data, channels, &samples_cb, &app);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| OtoError::Message(format!("build input stream (i16): {e}")))?
            }
            SampleFormat::U16 => {
                let samples_cb = Arc::clone(&samples);
                let app = app.clone();
                device
                    .build_input_stream(
                        config,
                        move |data: &[u16], _| {
                            process_u16(data, channels, &samples_cb, &app);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| OtoError::Message(format!("build input stream (u16): {e}")))?
            }
            other => {
                return Err(OtoError::Message(format!(
                    "unsupported sample format: {other:?}"
                )));
            }
        };

        Ok((stream, samples))
    }

    /// Stop capture, encode mono PCM as WAV, and return `(wav_bytes, sample_rate)`.
    pub fn stop(mut self) -> OtoResult<(Vec<u8>, u32)> {
        // Drop the stream to stop the callback before reading samples.
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
            drop(stream);
        }

        let mono = self
            .samples
            .lock()
            .map_err(|_| OtoError::Message("sample buffer poisoned".into()))?
            .clone();

        let wav = write_wav_i16_mono(&mono, self.sample_rate)?;
        Ok((wav, self.sample_rate))
    }
}

fn emit_level(app: &AppHandle, samples: &[i16]) {
    if samples.is_empty() {
        return;
    }
    let sum_sq: f64 = samples
        .iter()
        .map(|&s| {
            let f = s as f64;
            f * f
        })
        .sum();
    let rms = (sum_sq / samples.len() as f64).sqrt();
    let level = (rms / 32768.0).clamp(0.0, 1.0) as f32;
    let _ = app.emit("pipeline://event", PipelineEvent::Level { level });
}

/// Convert interleaved multi-channel frames to mono i16, append, emit level.
fn process_f32(data: &[f32], channels: u16, samples: &Arc<Mutex<Vec<i16>>>, app: &AppHandle) {
    let ch = channels.max(1) as usize;
    let mut chunk = Vec::with_capacity(data.len() / ch);
    if ch == 1 {
        for &s in data {
            chunk.push(f32_to_i16(s));
        }
    } else {
        for frame in data.chunks_exact(ch) {
            let sum: f32 = frame.iter().sum();
            chunk.push(f32_to_i16(sum / ch as f32));
        }
    }
    emit_level(app, &chunk);
    if let Ok(mut buf) = samples.lock() {
        buf.extend_from_slice(&chunk);
    }
}

fn process_i16(data: &[i16], channels: u16, samples: &Arc<Mutex<Vec<i16>>>, app: &AppHandle) {
    let ch = channels.max(1) as usize;
    let mut chunk = Vec::with_capacity(data.len() / ch);
    if ch == 1 {
        chunk.extend_from_slice(data);
    } else {
        for frame in data.chunks_exact(ch) {
            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
            chunk.push((sum / ch as i32) as i16);
        }
    }
    emit_level(app, &chunk);
    if let Ok(mut buf) = samples.lock() {
        buf.extend_from_slice(&chunk);
    }
}

fn process_u16(data: &[u16], channels: u16, samples: &Arc<Mutex<Vec<i16>>>, app: &AppHandle) {
    let ch = channels.max(1) as usize;
    let mut chunk = Vec::with_capacity(data.len() / ch);
    if ch == 1 {
        for &s in data {
            chunk.push(u16_to_i16(s));
        }
    } else {
        for frame in data.chunks_exact(ch) {
            let sum: i32 = frame.iter().map(|&s| u16_to_i16(s) as i32).sum();
            chunk.push((sum / ch as i32) as i16);
        }
    }
    emit_level(app, &chunk);
    if let Ok(mut buf) = samples.lock() {
        buf.extend_from_slice(&chunk);
    }
}

#[inline]
fn f32_to_i16(s: f32) -> i16 {
    let clamped = s.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

#[inline]
fn u16_to_i16(s: u16) -> i16 {
    // Center u16 around zero into i16 range.
    (s as i32 - 32768) as i16
}
