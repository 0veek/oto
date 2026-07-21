use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::error::{OtoError, OtoResult};

use super::traits::{SpeechToText, TranscriptionContext};

pub struct LocalWhisperClient {
    pub model_path: String,
}

type CachedModel = Option<(String, Arc<WhisperContext>)>;

static MODEL_CACHE: OnceLock<Mutex<CachedModel>> = OnceLock::new();

fn context_for_model(path: &str) -> OtoResult<Arc<WhisperContext>> {
    let cache = MODEL_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache
        .lock()
        .map_err(|_| OtoError::Message("local Whisper model cache poisoned".into()))?;
    if let Some((cached_path, context)) = guard.as_ref() {
        if cached_path == path {
            return Ok(Arc::clone(context));
        }
    }
    let context = Arc::new(
        WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|error| OtoError::Message(format!("load Whisper model: {error}")))?,
    );
    *guard = Some((path.to_string(), Arc::clone(&context)));
    Ok(context)
}

impl LocalWhisperClient {
    pub fn new(model_path: String) -> OtoResult<Self> {
        if model_path.trim().is_empty() {
            return Err(OtoError::Message(
                "Choose a local Whisper ggml model file in Models".into(),
            ));
        }
        if !std::path::Path::new(&model_path).is_file() {
            return Err(OtoError::Message(format!(
                "Local Whisper model not found: {model_path}"
            )));
        }
        Ok(Self { model_path })
    }
}

fn decode_wav(audio_wav: &[u8]) -> OtoResult<Vec<f32>> {
    let mut reader = hound::WavReader::new(Cursor::new(audio_wav))
        .map_err(|error| OtoError::Message(format!("read WAV: {error}")))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map(|value| value.clamp(-1.0, 1.0)))
            .collect::<Result<_, _>>()
            .map_err(|error| OtoError::Message(format!("decode WAV samples: {error}")))?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
            .collect::<Result<_, _>>()
            .map_err(|error| OtoError::Message(format!("decode WAV samples: {error}")))?,
        hound::SampleFormat::Int => {
            let scale = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<Result<_, _>>()
                .map_err(|error| OtoError::Message(format!("decode WAV samples: {error}")))?
        }
    };
    let mono: Vec<f32> = if channels == 1 {
        interleaved
    } else {
        interleaved
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };
    if spec.sample_rate == 16_000 {
        return Ok(mono);
    }
    if mono.is_empty() {
        return Ok(mono);
    }
    let output_len = (mono.len() as u64 * 16_000 / spec.sample_rate as u64) as usize;
    let ratio = spec.sample_rate as f64 / 16_000.0;
    let last = mono.len() - 1;
    let mut resampled = Vec::with_capacity(output_len);
    for output_index in 0..output_len {
        let source = output_index as f64 * ratio;
        // Clamp both indices: floating-point rounding can push `source` to
        // `mono.len()` at the final sample, which would panic on mono[left].
        let left = (source.floor() as usize).min(last);
        let right = (left + 1).min(last);
        let fraction = (source - left as f64) as f32;
        resampled.push(mono[left] * (1.0 - fraction) + mono[right] * fraction);
    }
    Ok(resampled)
}

#[async_trait]
impl SpeechToText for LocalWhisperClient {
    async fn transcribe(&self, audio_wav: &[u8], ctx: &TranscriptionContext) -> OtoResult<String> {
        let path = self.model_path.clone();
        let audio = audio_wav.to_vec();
        let context = ctx.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let pcm = decode_wav(&audio)?;
            if pcm.is_empty() {
                return Err(OtoError::Message("No audio samples captured".into()));
            }
            let whisper = context_for_model(&path)?;
            let mut state = whisper
                .create_state()
                .map_err(|error| OtoError::Message(format!("create Whisper state: {error}")))?;
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_n_threads(
                std::thread::available_parallelism()
                    .map(|count| count.get().min(8) as i32)
                    .unwrap_or(4),
            );
            params.set_translate(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            if let Some(language) = context.language.as_deref() {
                params.set_language(Some(language));
            } else {
                params.set_detect_language(true);
            }
            if let Some(prompt) = context.vocabulary_prompt.as_deref() {
                params.set_initial_prompt(prompt);
            }
            state
                .full(params, &pcm)
                .map_err(|error| OtoError::Message(format!("local transcription: {error}")))?;
            let segment_count = state.full_n_segments();
            let mut text = String::new();
            for index in 0..segment_count {
                let segment = state
                    .get_segment(index)
                    .ok_or_else(|| OtoError::Message("Whisper segment disappeared".into()))?;
                text.push_str(&segment.to_str_lossy().map_err(|error| {
                    OtoError::Message(format!("read Whisper segment: {error}"))
                })?);
            }
            Ok(text.trim().to_string())
        })
        .await
        .map_err(|error| OtoError::Message(format!("local Whisper task failed: {error}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_is_rejected_before_transcription() {
        assert!(LocalWhisperClient::new("/definitely/missing/model.bin".into()).is_err());
    }
}
