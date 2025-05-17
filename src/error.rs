use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[cfg(mobile)]
  #[error(transparent)]
  PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
  #[error("Cache key not found: {0}")]
  KeyNotFound(String),
  #[error("Cache entry expired")]
  Expired,
  #[error("Cache error: {0}")]
  Other(String),
  #[error("Failed to serialize or deserialize data: {0}")]
  SerdeError(String),
  #[error("Failed to initialize cache: {0}")]
  InitError(String),
}

impl Serialize for Error {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.to_string().as_ref())
  }
}

impl From<serde_json::Error> for Error {
  fn from(error: serde_json::Error) -> Self {
    Error::SerdeError(error.to_string())
  }
}
