#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use rp2040_hal as hal;
use hal::{multicore::Stack, sio::SioFifo, pac, Sio, Watchdog};

mod clocks;
mod board;

use board::Board;
use critical_section::Mutex;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use embedded_alloc::Heap;
use nes::{cartridge::Cartridge, nes::{Nes, HostPixelFormat}, frame::PixelFormat};
use core::{alloc::Layout, cell::RefCell};
use embedded_graphics::{
  prelude::*,
  pixelcolor::Rgb565, image::{ImageRaw, Image},
};
use embedded_hal::digital::v2::OutputPin;
use hal::multicore::Multicore;
use hal::clocks::Clock;

#[link_section = ".boot_loader"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[global_allocator]
static HEAP: Heap = Heap::empty();

const FRAME_BUF_SIZE: usize = nes::frame::NTSC_WIDTH * nes::frame::NTSC_HEIGHT * nes::frame::PixelFormatRGB565::BYTES_PER_PIXEL;
static FRAME_BUF: Mutex<RefCell<[u8; FRAME_BUF_SIZE]>> = Mutex::new(RefCell::new([0; FRAME_BUF_SIZE]));

static mut CORE1_STACK: Stack<2048> = Stack::new();

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
  defmt::panic!("OOM");
}

struct EmbeddedHost {
  start: bool,
  core1: SioFifo,
}

impl EmbeddedHost {
  fn new(core1: SioFifo) -> Self {
    Self {
      start: false,
      core1
    }
  }
}

impl nes::nes::HostPlatform for EmbeddedHost {
  fn pixel_format(&self) -> HostPixelFormat {
    HostPixelFormat::Rgb565
  }

  fn render(&mut self, frame: &nes::frame::RenderFrame) {
    critical_section::with(|cs| {
      let mut framebuf = FRAME_BUF.borrow_ref_mut(cs);
      for (i, p) in frame.pixels_ntsc().enumerate() {
        framebuf[i] = p;
      }
    });
    
    self.core1.write(1);
  }

  fn poll_events(&mut self, joypad: &mut nes::joypad::Joypad) -> nes::nes::Shutdown {
    nes::nes::Shutdown::No
  }
}

fn core1_render(sys_freq: u32) -> ! {
  let mut pac = unsafe { pac::Peripherals::steal() };
  let core = unsafe { pac::CorePeripherals::steal() };

  let mut sio = Sio::new(pac.SIO);
  let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);

  let (mut board, pins) = Board::new(
    pac.IO_BANK0,
    pac.PADS_BANK0,
sio.gpio_bank0,
    pac.SPI0,
    &mut pac.RESETS,
    &mut delay,
  );

  board.screen.clear(Rgb565::BLACK).unwrap();

  let mut led_pin = pins.led.into_push_pull_output();
  let mut frame_n = 0;

  info!("core 1 loopin");
  loop {
    let _ = sio.fifo.read_blocking();

    critical_section::with(|cs| {
      let frame = FRAME_BUF.borrow(cs).borrow();

      if frame_n % 2 == 0 {
        led_pin.set_high().unwrap();
      } else {
        led_pin.set_low().unwrap();
      }

      let raw_image = ImageRaw::<Rgb565>::new(&frame[..], nes::frame::NTSC_WIDTH as u32);
      let image = Image::new(&raw_image, Point::zero());
      image.draw(&mut board.screen).unwrap();
    });
    
    frame_n += 1;
  }
}

#[hal::entry]
fn main() -> ! {
  {
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 140000;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
  }

  let mut pac = pac::Peripherals::take().unwrap();
  let mut watchdog = Watchdog::new(pac.WATCHDOG);
  let clocks = clocks::configure_overclock(
    pac.XOSC, 
    pac.CLOCKS, 
    pac.PLL_SYS, 
    pac.PLL_USB, 
    &mut pac.RESETS, 
    &mut watchdog
  );

  let mut sio = Sio::new(pac.SIO);

  let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
  let cores = mc.cores();
  let core1 = &mut cores[1];

  info!("booting core1");
  let sys_freq = clocks.system_clock.freq().to_Hz();
  core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || {
    core1_render(sys_freq);
  })
  .expect("core1 failed");
  
  let rom = include_bytes!(env!("ROM"));
  let cart = Cartridge::blow_dust_no_heap(rom).unwrap();
  let host = EmbeddedHost::new(sio.fifo);
  let mut nes = Nes::insert(cart, host);
  
  loop {
    nes.tick();
  }
}
