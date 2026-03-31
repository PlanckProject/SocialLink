use std::sync::Arc;

use crate::config::Config;
use crate::services::AppServices;

/// Shared application state passed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub services: Arc<AppServices>,
    pub config: Arc<Config>,
}
