use picoserve::response::{IntoResponse, NoContent, StatusCode};

pub async fn generate_204() -> impl IntoResponse {
  (StatusCode::NO_CONTENT, NoContent)
}
pub async fn ncsi_txt() -> impl IntoResponse {
  "Microsoft NCSI"
}
pub async fn hotspot_html() -> impl IntoResponse {
  "<HTML><HEAD><TITLE>Success</TITLE></HEAD><BODY>Success</BODY></HTML>"
}
