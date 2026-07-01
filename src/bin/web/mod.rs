mod config;
mod expression;
mod ota;
mod portal;
mod wifi;

// Portal UI
const PORTAL_HTML: &str = include_str!("../../../portal/dist/index.html");
use embassy_net::Stack;
use embassy_time::Duration;
use picoserve::routing::{get, post};

pub(super) struct StaticContent {
  pub body: &'static str,
  pub content_type: &'static str,
}

impl picoserve::response::Content for StaticContent {
  fn content_type(&self) -> &'static str {
    self.content_type
  }
  fn content_length(&self) -> usize {
    self.body.len()
  }
  async fn write_content<W: embedded_io_async::Write>(self, mut writer: W) -> Result<(), W::Error> {
    writer.write_all(self.body.as_bytes()).await
  }
}

/// Capacity of the buffer backing a JSON API response
pub(super) const JSON_RESPONSE_CAP: usize = 384;

pub(super) struct JsonBytes {
  pub data: [u8; JSON_RESPONSE_CAP],
  pub len: usize,
}

impl picoserve::response::Content for JsonBytes {
  fn content_type(&self) -> &'static str {
    "application/json"
  }
  fn content_length(&self) -> usize {
    self.len
  }
  async fn write_content<W: embedded_io_async::Write>(self, mut writer: W) -> Result<(), W::Error> {
    writer.write_all(&self.data[..self.len]).await
  }
}

// Catchall: unknown paths serve the config page
struct PortalFallback;

impl picoserve::routing::PathRouterService<(), ()> for PortalFallback {
  async fn call_path_router_service<
    R: picoserve::io::Read,
    W: picoserve::response::ResponseWriter<Error = R::Error>,
  >(
    &self,
    _state: &(),
    _path_parameters: (),
    _path: picoserve::request::Path<'_>,
    request: picoserve::request::Request<'_, R>,
    response_writer: W,
  ) -> Result<picoserve::ResponseSent, W::Error> {
    use picoserve::response::IntoResponse as _;
    picoserve::response::Response::ok(StaticContent {
      body: PORTAL_HTML,
      content_type: "text/html; charset=utf-8",
    })
    .write_to(request.body_connection.finalize().await?, response_writer)
    .await
  }
}

#[allow(clippy::large_stack_frames, clippy::large_futures)]
#[embassy_executor::task(pool_size = 2)]
pub async fn web_task(
  stack: Stack<'static>,
  http_buf: &'static mut [u8; 2048],
  rx_buf: &'static mut [u8; 2048],
  tx_buf: &'static mut [u8; 2048],
) -> ! {
  let app = picoserve::Router::from_service(PortalFallback)
        // Captive portal detection: different OSes probe different well-known URLs
        .route("/generate_204",              get(portal::generate_204))
        .route("/gen_204",                   get(portal::generate_204))
        .route("/ncsi.txt",                  get(portal::ncsi_txt))
        .route("/connecttest.txt",           get(portal::ncsi_txt))
        .route("/hotspot-detect.html",       get(portal::hotspot_html))
        .route("/library/test/success.html", get(portal::hotspot_html))
        // Config API
        .route("/api/config",                get(config::get).post(config::set))
        .route("/api/config/reset",          post(config::reset))
        .route("/api/expression",            post(expression::set))
        .route("/api/wifi",                  post(wifi::set))
        .route("/api/wifi/reset",            post(wifi::reset))
        .route("/api/version",               get(ota::version))
        .route_service("/api/ota",           ota::OtaService);

  let server_config = picoserve::Config::new(picoserve::Timeouts {
    start_read_request: Duration::from_secs(5),
    persistent_start_read_request: Duration::from_secs(5),
    read_request: Duration::from_secs(5),
    write: Duration::from_secs(10),
  })
  .keep_connection_alive();

  loop {
    picoserve::Server::new(&app, &server_config, &mut http_buf[..])
      .listen_and_serve(0u32, stack, 80, &mut rx_buf[..], &mut tx_buf[..])
      .await;
  }
}
