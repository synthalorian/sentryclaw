pub mod config;
pub mod diff_filter;
pub mod github;
pub mod gitlab;
pub mod inline_comments;
pub mod llm;
pub mod review;

use std::sync::Arc;
use config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
}
