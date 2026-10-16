//! Sends frames to the GC9A01 over SPI DMA, only the rows that changed

use cotl::display::{DISPLAY_H, DISPLAY_W, Frame, ROW_BYTES, ROW_PIXELS};
use embassy_time::Timer;
use esp_hal::{
  Blocking,
  dma::DmaTxBuf,
  gpio::Output,
  spi::master::{Spi, SpiDma, SpiDmaTransfer},
};

// Rows per DMA chunk
const CHUNK_ROWS: usize = 16;
pub const CHUNK_BYTES: usize = CHUNK_ROWS * ROW_BYTES;
const _: () = assert!((DISPLAY_H as usize).is_multiple_of(CHUNK_ROWS));
// Fits one opcode or four argument bytes
pub const CMD_BYTES: usize = 4;

// MIPI DCS commands
const CASET: u8 = 0x2A;
const RASET: u8 = 0x2B;
const RAMWR: u8 = 0x2C;

type Transfer = SpiDmaTransfer<'static, Blocking, DmaTxBuf>;

pub struct Lcd {
  spi: Option<SpiDma<'static, Blocking>>,
  dc: Output<'static>,
  bufs: [Option<DmaTxBuf>; 2],
  cmd: Option<DmaTxBuf>,
  // Hash of each row as last sent
  row_hash: [u32; DISPLAY_H as usize],
  primed: bool,
  // CS (held low) and reset (held high) must stay driven
  _pins: [Output<'static>; 2],
}

/// CASET / RASET arguments, inclusive range
fn window(start: u16, end: u16) -> [u8; 4] {
  let [start_hi, start_lo] = start.to_be_bytes();
  let [end_hi, end_lo] = end.to_be_bytes();
  [start_hi, start_lo, end_hi, end_lo]
}

/// FNV-1a with the magic numbers everyone copies, so do we. Lives in IRAM
/// because it ran ~16% faster there. A collision leaves a row stale for a bit
#[esp_hal::ram]
fn hash_row(row: &[u32]) -> u32 {
  row.iter().fold(0x811C_9DC5, |hash, word| {
    (hash ^ word).wrapping_mul(0x0100_0193)
  })
}

fn start(spi: SpiDma<'static, Blocking>, buf: DmaTxBuf) -> Transfer {
  let len = buf.len();
  spi
    .write_buffer(len, buf)
    .map_err(|(err, ..)| err)
    .expect("lcd: DMA write failed")
}

/// The SPI interrupt ghosted us once, so now we just keep checking
async fn finish(transfer: Transfer) -> (SpiDma<'static, Blocking>, DmaTxBuf) {
  while !transfer.is_done() {
    Timer::after_micros(50).await;
  }
  transfer.wait()
}

impl Lcd {
  /// Call after mipidsi init, with CS already held low
  pub fn new(
    mut spi: Spi<'static, Blocking>,
    mut dc: Output<'static>,
    dma: esp_hal::peripherals::DMA_SPI2<'static>,
    [first, second, cmd]: [DmaTxBuf; 3],
    pins: [Output<'static>; 2],
  ) -> Self {
    // Full width once, rows are set per chunk
    dc.set_low();
    spi.write(&[CASET]).expect("lcd: command write failed");
    dc.set_high();
    spi
      .write(&window(0, DISPLAY_W as u16 - 1))
      .expect("lcd: command write failed");
    Self {
      spi: Some(spi.with_dma(dma)),
      dc,
      bufs: [Some(first), Some(second)],
      cmd: Some(cmd),
      row_hash: [0; DISPLAY_H as usize],
      primed: false,
      _pins: pins,
    }
  }

  /// Bus must be idle. A few bytes take under a microsecond, so just block
  fn command(&mut self, opcode: u8, args: &[u8]) {
    let mut spi = self.spi.take().expect("lcd: bus missing");
    let mut buf = self.cmd.take().expect("lcd: command buffer missing");
    self.dc.set_low();
    buf.fill(&[opcode]);
    (spi, buf) = start(spi, buf).wait();
    self.dc.set_high();
    if !args.is_empty() {
      buf.fill(args);
      (spi, buf) = start(spi, buf).wait();
    }
    self.spi = Some(spi);
    self.cmd = Some(buf);
  }

  pub async fn draw(&mut self, frame: &Frame) {
    let mut next = self.bufs[0].take().expect("lcd: buffer missing");
    let mut spare = self.bufs[1].take();
    let mut in_flight = None;

    for chunk_start in (0..DISPLAY_H).step_by(CHUNK_ROWS) {
      let data = &mut next.as_mut_slice()[..CHUNK_BYTES];
      // every bit pattern is a valid u16 / u32
      let (head, pixels, _) = unsafe { data.align_to_mut::<u16>() };
      assert!(head.is_empty(), "lcd: DMA buffer not aligned");
      let mut dirty: Option<(usize, usize)> = None;
      for (row, (idx, out)) in (chunk_start..).zip(pixels.chunks_exact_mut(ROW_PIXELS).enumerate())
      {
        frame.render_row(row, out);
        // as above, rows are 480 bytes so they stay aligned
        let (_, words, _) = unsafe { out.align_to::<u32>() };
        let hash = hash_row(words);
        let sent = &mut self.row_hash[row as usize];
        if !self.primed || *sent != hash {
          *sent = hash;
          dirty = Some((dirty.map_or(idx, |(first, _)| first), idx));
        }
      }
      let Some((first, last)) = dirty else {
        continue;
      };
      // DMA sends from the buffer start
      data.copy_within(first * ROW_BYTES..(last + 1) * ROW_BYTES, 0);
      next.set_length((last + 1 - first) * ROW_BYTES);

      // D/C can only flip once the last chunk is out
      if let Some(transfer) = in_flight.take() {
        let (spi, done) = finish(transfer).await;
        self.spi = Some(spi);
        spare = Some(done);
      }
      let top = chunk_start as u16;
      self.command(RASET, &window(top + first as u16, top + last as u16));
      self.command(RAMWR, &[]);
      in_flight = Some(start(self.spi.take().expect("lcd: bus missing"), next));
      next = spare.take().expect("lcd: buffer missing");
    }

    if let Some(transfer) = in_flight {
      let (spi, done) = finish(transfer).await;
      self.spi = Some(spi);
      spare = Some(done);
    }
    self.primed = true;
    self.bufs = [Some(next), spare];
  }
}
