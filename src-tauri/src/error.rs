use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    NotFound { path: String },
    ProcessFailed { reason: String },
    Config { field: String, reason: String },
    Io { reason: String },
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Io { reason: s }
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Io { reason: s.to_string() }
    }
}
