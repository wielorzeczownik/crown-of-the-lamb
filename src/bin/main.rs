#![no_std]
#![no_main]
#![recursion_limit = "512"]
#![allow(clippy::large_stack_frames)]

extern crate alloc;

mod dhcp;
mod dns;
mod reactor;
mod web;

use core::sync::atomic::Ordering::Relaxed;

use cotl::{
  constants::{GATEWAY_IP, SUSPICIOUS_GAZE_OFFSET},
  control,
  display::draw_eye,
  eye::{Expression, EyeState},
  storage,
};
use embassy_executor::Spawner;
use embassy_net::{Config as NetConfig, Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{draw_target::DrawTarget, pixelcolor::Rgb565};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
  analog::adc::{Adc, AdcConfig, Attenuation},
  clock::CpuClock,
  delay::Delay,
  gpio::{Level, Output, OutputConfig},
  interrupt::software::SoftwareInterruptControl,
  rng::Rng,
  spi::master::{Config as SpiConfig, Spi},
  time::Rate,
  timer::timg::TimerGroup,
};
use esp_println as _;
use esp_radio::wifi::{Config as WifiConfig, ControllerConfig, Interface, ap::AccessPointConfig};
use esp_storage::FlashStorage;
use mipidsi::{
  Builder,
  interface::SpiInterface,
  models::GC9A01,
  options::{ColorInversion, ColorOrder},
};
use static_cell::StaticCell;

// Display SPI clock the GC9A01 runs comfortably at 80 MHz
const DISPLAY_SPI_MHZ: u32 = 80;
// SPI scratch buffer for the display interface
const SPI_BUF_SIZE: usize = 512;
// Per-socket buffers for each web server task
const WEB_BUF_SIZE: usize = 2048;
// embassy-net socket slots, shared by the 2 web tasks + DHCP + DNS
const MAX_SOCKETS: usize = 4;
// ESP32 12-bit ADC normalisation (raw range 0..4095)
const ADC_MAX_VALUE: f32 = 4095.0;
// Animation frames (~1 ms each) between sound samples
const SOUND_SAMPLE_INTERVAL: u32 = 20;
// AP static-IP subnet prefix length
const AP_SUBNET_PREFIX: u8 = 24;
// Heap for esp-alloc
const HEAP_SIZE: usize = 72_768;
// Delay before software reset, so the HTTP response reaches the browser
const RESTART_DELAY_MS: u64 = 400;
// Main animation loop timing
const MICROS_PER_MS: f32 = 1000.0;
const MIN_FRAME_DT_MS: f32 = 1.0; // avoid a zero delta
const MAX_FRAME_DT_MS: f32 = 50.0; // skip huge jumps after stalls
const FRAME_DELAY_MS: u64 = 1; // ~1 kHz animation loop

static NET_RESOURCES: StaticCell<StackResources<MAX_SOCKETS>> = StaticCell::new();

static WEB_HTTP_BUF_0: StaticCell<[u8; WEB_BUF_SIZE]> = StaticCell::new();
static WEB_HTTP_BUF_1: StaticCell<[u8; WEB_BUF_SIZE]> = StaticCell::new();
static WEB_RX_BUF_0: StaticCell<[u8; WEB_BUF_SIZE]> = StaticCell::new();
static WEB_RX_BUF_1: StaticCell<[u8; WEB_BUF_SIZE]> = StaticCell::new();
static WEB_TX_BUF_0: StaticCell<[u8; WEB_BUF_SIZE]> = StaticCell::new();
static WEB_TX_BUF_1: StaticCell<[u8; WEB_BUF_SIZE]> = StaticCell::new();
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
  esp_println::println!("PANIC: {}", info);
  loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) -> ! {
  runner.run().await
}

#[embassy_executor::task]
async fn flash_save_task() -> ! {
  loop {
    storage::SAVE_SIGNAL.wait().await;
    // Wait 2 s and drain any follow-up signals
    Timer::after(Duration::from_secs(2)).await;
    storage::SAVE_SIGNAL.try_take();

    let mut config = storage::StoredConfig::load().await;
    cotl::config::capture_into(&mut config);
    config.save().await;
    defmt::debug!("config: saved to flash");
  }
}

fn spawn_tasks(
  spawner: Spawner,
  stack: Stack<'static>,
  net_runner: Runner<'static, Interface<'static>>,
) {
  spawner.spawn(net_task(net_runner).unwrap());
  spawner.spawn(dhcp::dhcp_task(stack).unwrap());
  spawner.spawn(dns::dns_task(stack).unwrap());
  spawner.spawn(
    web::web_task(
      stack,
      WEB_HTTP_BUF_0.init([0u8; WEB_BUF_SIZE]),
      WEB_RX_BUF_0.init([0u8; WEB_BUF_SIZE]),
      WEB_TX_BUF_0.init([0u8; WEB_BUF_SIZE]),
    )
    .unwrap(),
  );
  spawner.spawn(
    web::web_task(
      stack,
      WEB_HTTP_BUF_1.init([0u8; WEB_BUF_SIZE]),
      WEB_RX_BUF_1.init([0u8; WEB_BUF_SIZE]),
      WEB_TX_BUF_1.init([0u8; WEB_BUF_SIZE]),
    )
    .unwrap(),
  );
  spawner.spawn(flash_save_task().unwrap());
}

static SPI_BUF: StaticCell<[u8; SPI_BUF_SIZE]> = StaticCell::new();

/// Brings up the SPI bus and the GC9A01 panel
fn setup_display(
  spi: esp_hal::peripherals::SPI2<'static>,
  sck: esp_hal::peripherals::GPIO18<'static>,
  mosi: esp_hal::peripherals::GPIO23<'static>,
  cs: esp_hal::peripherals::GPIO5<'static>,
  dc: esp_hal::peripherals::GPIO2<'static>,
  rst: esp_hal::peripherals::GPIO4<'static>,
) -> impl DrawTarget<Color = Rgb565> {
  let spi_bus = Spi::new(
    spi,
    SpiConfig::default().with_frequency(Rate::from_mhz(DISPLAY_SPI_MHZ)),
  )
  .expect("SPI init failed")
  .with_sck(sck)
  .with_mosi(mosi);

  let pin_cfg = OutputConfig::default();
  let chip_select = Output::new(cs, Level::High, pin_cfg);
  let data_cmd = Output::new(dc, Level::Low, pin_cfg);
  let display_rst = Output::new(rst, Level::High, pin_cfg);

  let spi_device = ExclusiveDevice::new_no_delay(spi_bus, chip_select).unwrap();

  let iface_buf = SPI_BUF.init([0u8; SPI_BUF_SIZE]);

  let display_iface = SpiInterface::new(spi_device, data_cmd, iface_buf);
  let mut delay = Delay::new();
  Builder::new(GC9A01, display_iface)
    .color_order(ColorOrder::Bgr)
    .invert_colors(ColorInversion::Inverted)
    .reset_pin(display_rst)
    .init(&mut delay)
    .expect("display init failed")
}

/// Builds a 64-bit seed from the hardware RNG
fn random_seed(rng: Rng) -> u64 {
  (u64::from(rng.random()) << 32) | u64::from(rng.random())
}

/// Starts the WiFi access point and the embassy-net stack
fn setup_network(
  wifi: esp_hal::peripherals::WIFI<'static>,
  ssid: &str,
) -> (
  esp_radio::wifi::WifiController<'static>,
  Stack<'static>,
  Runner<'static, Interface<'static>>,
) {
  let wifi_config = ControllerConfig::default().with_initial_config(WifiConfig::AccessPoint(
    AccessPointConfig::default().with_ssid(ssid),
  ));
  let (wifi_ctrl, interfaces) = esp_radio::wifi::new(wifi, wifi_config).unwrap();

  let net_resources = NET_RESOURCES.init(StackResources::new());
  let net_seed = random_seed(Rng::new());
  let (stack, net_runner) = embassy_net::new(
    interfaces.access_point,
    NetConfig::ipv4_static(StaticConfigV4 {
      address: Ipv4Cidr::new(GATEWAY_IP, AP_SUBNET_PREFIX),
      gateway: Some(GATEWAY_IP),
      dns_servers: heapless::Vec::new(),
    }),
    net_resources,
    net_seed,
  );
  (wifi_ctrl, stack, net_runner)
}

#[allow(
  clippy::large_stack_frames,
  reason = "init buffers are fine on the stack in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
  let hal_config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
  let peripherals = esp_hal::init(hal_config);

  esp_alloc::heap_allocator!(size: HEAP_SIZE);

  defmt::info!("boot: firmware v{}", env!("CARGO_PKG_VERSION"));

  // Must come before any StoredConfig::load/save
  storage::init(FlashStorage::new(peripherals.FLASH)).await;

  let stored = storage::StoredConfig::load().await;
  cotl::config::apply_stored(&stored);
  defmt::info!("config: loaded ssid={}", stored.ssid.as_str());

  let timg0 = TimerGroup::new(peripherals.TIMG0);
  let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
  esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

  // Display
  let mut display = setup_display(
    peripherals.SPI2,
    peripherals.GPIO18,
    peripherals.GPIO23,
    peripherals.GPIO5,
    peripherals.GPIO2,
    peripherals.GPIO4,
  );
  let _backlight = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());

  // ADC
  let mut adc1_config = AdcConfig::new();
  let mut mic_left = adc1_config.enable_pin(peripherals.GPIO34, Attenuation::_11dB);
  let mut mic_right = adc1_config.enable_pin(peripherals.GPIO35, Attenuation::_11dB);
  let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);

  // WiFi access point + network stack
  let (_wifi_ctrl, stack, net_runner) = setup_network(peripherals.WIFI, stored.ssid.as_str());
  defmt::info!("wifi: AP up ssid={}", stored.ssid.as_str());

  // Spawn background tasks
  spawn_tasks(spawner, stack, net_runner);
  defmt::info!("boot: ready, entering main loop");

  // Animation state. Each PRNG is seeded from the hardware RNG
  let rng = Rng::new();
  let mut eye = EyeState::new(random_seed(rng));
  let mut reactor = reactor::SoundReactor::new(random_seed(rng));

  let mut frame: u32 = 0;
  let mut last_frame_instant = Instant::now();

  loop {
    // Web UI expression request
    if let Some(expr) = Expression::from_web_code(control::EXPRESSION.swap(0, Relaxed)) {
      defmt::info!("expr: web request {}", expr);
      let param = if expr == Expression::Suspicious {
        SUSPICIOUS_GAZE_OFFSET
      } else {
        0.0
      };
      eye.trigger_expr(expr, param);
    }

    if control::RESTART_REQUESTED.load(Relaxed) {
      defmt::warn!("system: restart requested");
      // Short delay lets the HTTP response reach the browser before reset
      Timer::after(Duration::from_millis(RESTART_DELAY_MS)).await;
      storage::software_reset();
    }

    let now = Instant::now();
    // Clamp to skip huge jumps after stalls
    let dt_ms = (now.duration_since(last_frame_instant).as_micros() as f32 / MICROS_PER_MS)
      .clamp(MIN_FRAME_DT_MS, MAX_FRAME_DT_MS);
    last_frame_instant = now;

    // Advance sound timers and react to any fresh direction change
    reactor.update(&mut eye, now);

    let gesture_active = reactor.is_gesture_active(now);

    if frame.is_multiple_of(SOUND_SAMPLE_INTERVAL) {
      reactor.sample(|| {
        let left = nb::block!(adc1.read_oneshot(&mut mic_left))
          .inspect_err(|()| defmt::warn!("adc: left read failed"))
          .ok()
          .map(|raw| f32::from(raw) / ADC_MAX_VALUE);
        let right = nb::block!(adc1.read_oneshot(&mut mic_right))
          .inspect_err(|()| defmt::warn!("adc: right read failed"))
          .ok()
          .map(|raw| f32::from(raw) / ADC_MAX_VALUE);
        (left, right)
      });
    }

    eye.sound_x = reactor.track_eye_x(gesture_active);
    eye.extra_close = reactor.track_squint(gesture_active);

    eye.update(dt_ms);
    draw_eye(&mut display, &eye);

    Timer::after(Duration::from_millis(FRAME_DELAY_MS)).await;
    frame = frame.wrapping_add(1);
  }
}
