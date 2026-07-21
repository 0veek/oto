use std::sync::Arc;

use crate::pipeline::Pipeline;

pub struct AppState {
    pub pipeline: Arc<Pipeline>,
}
