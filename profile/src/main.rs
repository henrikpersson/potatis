#![cfg_attr(not(feature = "profile_cpu_std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;

use nes::cartridge::Cartridge;
use nes::nes::Nes;

#[cfg(feature = "profile_heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

struct FakeHost;

const EXPECTED_FRAME_SIZE: usize = 240 * 224 * 2;

impl nes::nes::HostPlatform for FakeHost {
  fn elapsed_millis(&self) -> usize {
    0
  }

  fn delay(&self, _: core::time::Duration) {}

  #[no_mangle]
  fn render(&mut self, f: &nes::frame::RenderFrame) {
    let buf: Vec<u8> = f.pixels_ntsc().collect();
    assert_eq!(buf.len(), EXPECTED_FRAME_SIZE);
  }

  fn poll_events(&mut self, _: &mut nes::joypad::Joypad) -> nes::nes::Shutdown {
    nes::nes::Shutdown::No
  }

  fn pixel_format(&self) -> nes::nes::HostPixelFormat {
    nes::nes::HostPixelFormat::Rgb565
  }
}

#[allow(unreachable_code)]
fn main() {
  #[cfg(debug_assertions)]
  panic!("run profile with --release");

  #[cfg(feature = "profile_heap")]
  let _profiler = dhat::Profiler::new_heap();

  let rom = include_bytes!(env!("PROF_ROM"));
  let cart = Cartridge::blow_dust_no_heap(rom).unwrap();
  let mut nes = Nes::insert(cart, FakeHost {});

  for _ in 0..10_000_000 {
    nes.tick();
  }
}
