use core::panic;

use common::kilobytes;
use mos6502::memory::Bus;

use crate::cartridge::Cartridge;

use super::Mapper;

pub struct NROM {
  cart: Cartridge,
  prg_ram: [u8; kilobytes::KB8],
  is_16kb: bool
}

impl NROM {
  pub fn new(cart: Cartridge) -> Self {
    let is_16kb = match cart.prg().len() {
      kilobytes::KB16 => true,
      kilobytes::KB32 => false,
      _ => panic!("invalid size for NROM prg rom")
    };
    Self { 
      cart, 
      prg_ram: [0; kilobytes::KB8], 
      is_16kb 
    }
  }
}

impl Mapper for NROM {
  fn mirroring(&self) -> crate::cartridge::Mirroring {
    self.cart.mirroring()
  }
}

impl Bus for NROM {
  fn read8(&self, address: u16) -> u8 {
    // println!("NROM reading addr: {:#06x}", address);
    match address {
      0x0000..=0x1fff => self.cart.chr()[address as usize], // PPU
      // TODO: Mirrored, Write protectable w external switch
      0x6000..=0x7fff => self.prg_ram[address as usize - 0x6000],
      0x8000..=0xbfff => self.cart.prg()[address as usize - 0x8000],
      0xc000..=0xffff => {
        if self.is_16kb {
          // Mirror
          self.cart.prg()[address as usize - 0xc000]
        }
        else {
          // last 16kb of rom
          self.cart.prg()[kilobytes::KB16 + (address as usize - 0xc000)]
        }
      }
      _ => 0//panic!("unknown NROM memory range: {:#06x}", address)
    }
  }

  fn write8(&mut self, _: u8, _: u16) {
    
  }
}