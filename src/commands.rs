use tauri::{AppHandle, command, Runtime};

use crate::models::*;
use crate::Result;
use crate::CacheExt;

#[command]
pub(crate) async fn set<R: Runtime>(
  app: AppHandle<R>,
  request: SetRequest,
) -> Result<()> {
  app.cache().set(&request.key, &request.value, request.ttl).await
}

#[command]
pub(crate) async fn get<R: Runtime>(
  app: AppHandle<R>,
  request: GetRequest,
) -> Result<GetResponse> {
  let value = app.cache().get(&request.key).await?;
  let exists = value.is_some();
  Ok(GetResponse { value, exists })
}

#[command]
pub(crate) async fn has_key<R: Runtime>(
  app: AppHandle<R>,
  request: HasRequest,
) -> Result<HasResponse> {
  let exists = app.cache().has(&request.key).await?;
  Ok(HasResponse { exists })
}

#[command]
pub(crate) async fn remove<R: Runtime>(
  app: AppHandle<R>,
  request: RemoveRequest,
) -> Result<()> {
  app.cache().remove(&request.key).await
}

#[command]
pub(crate) async fn clear<R: Runtime>(
  app: AppHandle<R>,
) -> Result<()> {
  app.cache().clear().await
}

#[command]
pub(crate) async fn keys<R: Runtime>(
  app: AppHandle<R>,
) -> Result<KeysResponse> {
  let keys = app.cache().keys().await?;
  Ok(KeysResponse { keys })
}