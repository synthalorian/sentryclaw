pub mod config;
pub mod dashboard;
pub mod db;
pub mod diff_filter;
pub mod github;
pub mod gitlab;
pub mod inline_comments;
pub mod llm;
pub mod review;

use std::sync::Arc;
use config::AppConfig;
use db::Database;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub database: Arc<Database>,
}
