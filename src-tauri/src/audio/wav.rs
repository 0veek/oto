use crate::error::OtoResult;
use hound::{WavSpec, WavWriter};
use std::io::Cursor;

#[allow(dead_code)] // used by AudioRecorder::stop and unit tests
pub fn write_wav_i16_mono(samples: &[i16], sample_rate: u32) -> OtoResult<Vec<u8>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)
            .map_err(|e| crate::error::OtoError::Message(e.to_string()))?;
        for &s in samples {
            writer
                .write_sample(s)
                .map_err(|e| crate::error::OtoError::Message(e.to_string()))?;
        }
        writer
            .finalize()
            .map_err(|e| crate::error::OtoError::Message(e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_has_riff_header() {
        let samples = vec![0i16; 1600];
        let bytes = write_wav_i16_mono(&samples, 16000).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
    }
}
