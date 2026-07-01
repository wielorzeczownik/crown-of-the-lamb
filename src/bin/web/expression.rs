use core::sync::atomic::Ordering::Relaxed;

use cotl::eye::Expression;
use picoserve::{
  extract::Form,
  response::{IntoResponse, NoContent, StatusCode},
};

#[derive(serde::Deserialize)]
pub struct ExpressionQuery {
  /// Web expression code
  pub mode: i32,
}

pub async fn set(Form(query): Form<ExpressionQuery>) -> impl IntoResponse {
  if Expression::from_web_code(query.mode).is_some() {
    cotl::control::EXPRESSION.store(query.mode, Relaxed);
    (StatusCode::NO_CONTENT, NoContent)
  } else {
    (StatusCode::BAD_REQUEST, NoContent)
  }
}
