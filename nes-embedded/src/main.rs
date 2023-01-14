#![no_std]
#![no_main]

use pimoroni_pico_explorer::entry;
use nes::{cartridge::Cartridge, nes::Nes};
use rp_pico::entry;

// PicoHost?
struct EmbeddedHost {

}

impl nes::nes::HostSystem for EmbeddedHost {
  fn render(&mut self, frame: &nes::frame::RenderFrame) {
    todo!()
  }

  fn poll_events(&mut self, joypad: &mut nes::joypad::Joypad) -> nes::nes::Shutdown {
    todo!()
  }
}

#[entry]
fn main() -> ! {
  let rom = include_bytes!("../../legal-roms/smb.nes");
  let cart = Cartridge::load(rom).unwrap();
  let nes = Nes::insert(cart, EmbeddedHost {});

  loop {
    nes.tick()
  }
}