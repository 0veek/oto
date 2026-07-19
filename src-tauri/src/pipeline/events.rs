use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Idle,
    Listening,
    Processing,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PipelineEvent {
    State {
        state: PipelineState,
        detail: Option<String>,
    },
    Level {
        /// 0.0 ..= 1.0 RMS-ish level for waveform
        level: f32,
    },
    Phase {
        /// "transcribing" | "polishing" | "injecting"
        phase: String,
    },
    Partial {
        /// Best transcription available so far. Local streaming can update this
        /// repeatedly; cloud providers emit it when their first text arrives.
        text: String,
    },
    Error {
        message: String,
    },
}

impl PipelineEvent {
    pub fn state(state: PipelineState, detail: impl Into<Option<String>>) -> Self {
        Self::State {
            state,
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_event_serializes_snake_case() {
        let ev = PipelineEvent::state(PipelineState::Listening, None);
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"state\""));
        assert!(json.contains("\"state\":\"listening\""));
    }
}
