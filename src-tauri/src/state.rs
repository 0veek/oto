use std::sync::Arc;

use tokio::sync::Mutex;

use crate::pipeline::Pipeline;

pub struct AppState {
    pub cancel_flag: Arc<Mutex<bool>>,
    pub pipeline: Arc<Pipeline>,
}
