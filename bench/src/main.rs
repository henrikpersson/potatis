#![no_std]

use nes::{cartridge::{Cartridge}, nes::Nes};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

struct FakeHost;

impl nes::nes::HostSystem for FakeHost {
  fn elapsed_millis(&self) -> usize {
     0
  }

  fn delay(&self, _: core::time::Duration) {
    
  }

  fn render(&mut self, _: &nes::frame::RenderFrame) {
    
  }

  fn poll_events(&mut self, _: &mut nes::joypad::Joypad) -> nes::nes::Shutdown {
    nes::nes::Shutdown::No
  }

  fn display_region(&self) -> nes::nes::HostDisplayRegion {
    nes::nes::HostDisplayRegion::Ntsc
  }

  fn pixel_format(&self) -> nes::nes::HostPixelFormat {
    nes::nes::HostPixelFormat::Rgb565
  }
}

#[allow(unreachable_code)]
fn main() {
  #[cfg(debug_assertions)]
  panic!("run bench with --release");

  let _profiler = dhat::Profiler::new_heap();

  // let rom = include_bytes!("../../test-roms/nestest/nestest.nes");
  let rom = include_bytes!("../../legal-roms/drmario.nes");
  // let rom = include_bytes!("../../legal-roms/smb.nes");
  // let rom = EmbeddedRom { full: rom };
  let cart = Cartridge::blow_dust_no_heap(rom).unwrap();
  let mut nes = Nes::insert(cart, FakeHost{});

  for _ in 0..100000000 {
    nes.tick();
  }
}
