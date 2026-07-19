use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub cancel_flag: Arc<Mutex<bool>>,
}
