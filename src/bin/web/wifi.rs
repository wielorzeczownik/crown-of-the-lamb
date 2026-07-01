use core::sync::atomic::Ordering::Relaxed;

use picoserve::{
  extract::Form,
  response::{IntoResponse, StatusCode},
};

#[derive(serde::Deserialize)]
pub struct WifiQuery {
  pub ssid: Option<alloc::string::String>,
}

pub async fn set(Form(query): Form<WifiQuery>) -> impl IntoResponse {
  let mut stored = cotl::storage::StoredConfig::load().await;
  if let Some(new_ssid) = query.ssid {
    stored.ssid = new_ssid.as_str().try_into().unwrap_or_default();
  }
  stored.save().await;
  cotl::control::RESTART_REQUESTED.store(true, Relaxed);
  (StatusCode::OK, "restarting")
}

pub async fn reset() -> impl IntoResponse {
  let mut stored = cotl::storage::StoredConfig::load().await;
  let defaults = cotl::storage::StoredConfig::default();
  stored.ssid = defaults.ssid;
  stored.save().await;
  cotl::control::RESTART_REQUESTED.store(true, Relaxed);
  (StatusCode::OK, "restarting")
}
