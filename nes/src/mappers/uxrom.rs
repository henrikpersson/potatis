use common::kilobytes;
use mos6502::memory::Bus;

use crate::cartridge::Cartridge;

use super::Mapper;

pub struct UxROM {
  cart: Cartridge,
  prg_rom_banks: Vec<Vec<u8>>,
  bank: u8
}

impl UxROM {
  pub fn new(cart: Cartridge) -> Self {
    let chunks = cart.prg().chunks_exact(kilobytes::KB16);
    assert!(chunks.remainder().is_empty());
    let prg_rom_banks: Vec<Vec<u8>> = chunks.map(|s| s.to_vec()).collect();
    
    Self { 
      cart,
      prg_rom_banks,
      bank: 0
    }
  }
}

impl Mapper for UxROM {
  fn mirroring(&self) -> crate::cartridge::Mirroring {
    self.cart.mirroring()
  }
}

impl Bus for UxROM {
  fn read8(&self, address: u16) -> u8 {
    let address = address as usize;
    match address {
      0x0000..=0x1fff => self.cart.chr()[address],
      0x8000..=0xbfff => self.prg_rom_banks[self.bank as usize][address - 0x8000],
      0xc000..=0xffff => self.prg_rom_banks.last().unwrap()[address - 0xc000],
      _ => 0
    }
  }

  fn write8(&mut self, val: u8, address: u16) {
    match address {
      0x0000..=0x1fff => self.cart.chr_mut()[address as usize] = val,
      0x8000..=0xffff => self.bank = val,
      _ => ()
    }
  }
}