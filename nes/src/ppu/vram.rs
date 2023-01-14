use core::cell::RefCell;
use alloc::rc::Rc;
use common::kilobytes;

use crate::{cartridge::Mirroring, mappers::Mapper};

pub(crate) struct Vram {
  nametables: [[u8; kilobytes::KB1]; 2], // AKA CIRAM
  mapper: Rc<RefCell<dyn Mapper>>,
}

impl Vram {
  pub fn new(mapper: Rc<RefCell<dyn Mapper>>) -> Self {
    Self { nametables: [[0; kilobytes::KB1]; 2], mapper }
  }

  pub fn mode(&self) -> Mirroring {
    self.mapper.borrow().mirroring()
  }

  pub fn read(&self, address: u16) -> u8 {
    assert!((0x2000..=0x2fff).contains(&address), "Invalid vram read: {:#06x}", address);

    let virtual_index = Self::get_virtual_nametable_index(address);
    let index = Self::map(&self.mode(), virtual_index);
    let offset = address & 0x3ff; // Only lower 9 bits, higher are indexing
    self.nametables[index][offset as usize]
  }

  pub fn write(&mut self, val: u8, address: u16) {
    assert!((0x2000..=0x2fff).contains(&address), "Invalid vram write: {:#06x}", address);

    let virtual_index = Self::get_virtual_nametable_index(address);
    let index = Self::map(&self.mode(), virtual_index);
    let offset = address & 0x3ff; // Only lower 9 bits, higher are indexing
    self.nametables[index][offset as usize] = val;
  }

  pub fn read_indexed(&self, virtual_index: u16, offset: u16) -> u8 {
    let index = Self::map(&self.mode(), virtual_index);
    self.nametables[index][offset as usize]
  }

  fn get_virtual_nametable_index(address: u16) -> u16 {
    // (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    // Start == 0x2000, bit 11 & 10 selects the nametable index.
    (address >> 10) & 0b11
  }

  // TODO: This mapping could probably be setup in the constructor already, 
  // instead of calculated for EVERY read/write..
  fn map(mode: &Mirroring, virtual_index: u16) -> usize {
    assert!(virtual_index <= 3);

    // https://www.nesdev.org/wiki/PPU_nametables
    // https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    let physical_index = match (mode, virtual_index) {
      (Mirroring::Vertical, 0) => 0,
      (Mirroring::Vertical, 1) => 1,
      (Mirroring::Vertical, 2) => 0,
      (Mirroring::Vertical, 3) => 1,
      (Mirroring::Horizontal, 0) => 0,
      (Mirroring::Horizontal, 1) => 0,
      (Mirroring::Horizontal, 2) => 1,
      (Mirroring::Horizontal, 3) => 1,
      (Mirroring::SingleScreenLower, _) => 0,
      (Mirroring::SingleScreenUpper, _) => 1,
      _ => panic!("nametable mirroring? {} {:?}", virtual_index, mode),
    };

    physical_index
  }
}

#[cfg(test)]
mod tests {
  use crate::{ppu::vram::Vram};

  #[test]
  fn get_virtual_nametable_index() {
    assert_eq!(Vram::get_virtual_nametable_index(0x2000), 0);
    assert_eq!(Vram::get_virtual_nametable_index(0x2400), 1);
    assert_eq!(Vram::get_virtual_nametable_index(0x2800), 2);
    assert_eq!(Vram::get_virtual_nametable_index(0x2c00), 3);
    assert_eq!(Vram::get_virtual_nametable_index(0x24ff), 1);
  }
}