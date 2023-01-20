use core::panic;

use common::kilobytes;
use mos6502::memory::Bus;

use crate::cartridge::{Cartridge, Rom};

use super::Mapper;

pub struct NROM<R : Rom> {
  cart: Cartridge<R>,
  is_16kb: bool
}

impl<R : Rom> Mapper for NROM<R> {}

impl<R : Rom> NROM<R> {
  pub fn new(cart: Cartridge<R>) -> Self {
    let is_16kb = match cart.prg().len() {
      kilobytes::KB16 => true,
      kilobytes::KB32 => false,
      _ => panic!("invalid size for NROM prg rom")
    };
    Self { 
      cart, 
      is_16kb 
    }
  }
}

impl<R : Rom> Bus for NROM<R> {
  fn read8(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x1fff => self.cart.chr()[address as usize], // PPU
      // TODO: Mirrored, Write protectable w external switch
      // 0x6000..=0x7fff => self.cart.prg_ram()[address as usize - 0x6000],
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