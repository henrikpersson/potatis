use common::kilobytes;
use mos6502::memory::Bus;

use crate::cartridge::Cartridge;

const BANK_SIZE: usize = kilobytes::KB8;

// AKA CNROM
pub(crate) struct Mapper3 {
  cart: Cartridge,
  selected_bank: usize,
  is_16kb: bool,

  // this mapper does not have a ram, but tests put status codes in mem ranges outside of the
  // documented memory map for this mapper. this ram is only used in tests. TODO make better
  ram_for_integration_test: [u8; kilobytes::KB32]
}

impl Mapper3 {
  pub fn new(cart: Cartridge) -> Self {
    let is_16kb = match cart.prg().len() {
      kilobytes::KB16 => true,
      kilobytes::KB32 => false,
      _ => panic!("invalid size for mapper 3 prg rom")
    };

    Self {
      cart,
      selected_bank: 0,
      is_16kb,
      ram_for_integration_test: [0; kilobytes::KB32]
    }
  }
}

impl Bus for Mapper3 {
  fn read8(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x1fff => self.cart.chr()[(self.selected_bank * BANK_SIZE) + address as usize],
      0x8000..=0xffff => {
        if self.is_16kb {
          self.cart.prg()[address as usize - 0x8000 - kilobytes::KB16] // see tests
        }
        else {
          self.cart.prg()[address as usize - 0x8000]
        }
      }
      _ => {
        self.ram_for_integration_test[address as usize]
      }
    }
  }

  fn write8(&mut self, val: u8, address: u16) {
    match address {
      0x0000..=0x1fff => self.cart.chr_mut()[(self.selected_bank * BANK_SIZE) + address as usize] = val,
      0x8000..=0xffff => {
        self.selected_bank = (val & 0b00000011) as usize;
        // println!("mapper 3 selected bank: {}", self.selected_bank);
      },
      _ => self.ram_for_integration_test[address as usize] = val
    }
  }
}

#[cfg(test)]
mod tests {
  use common::kilobytes;
  use mos6502::memory::Bus;

  use crate::cartridge::Cartridge;

  use super::Mapper3;

  #[test]
  fn test_vectors_at_end() {
    let mut kb32 = [0; kilobytes::KB32];
    kb32[kilobytes::KB32-2..kilobytes::KB32].copy_from_slice(&[0xde, 0xad]);
    let cart = Cartridge::new_test(&kb32, &[]);
    let mapper = Mapper3::new(cart);
    assert_eq!(mapper.read8(0xfffe), 0xde);
    assert_eq!(mapper.read8(0xffff), 0xad);

    let mut kb16 = [0; kilobytes::KB16];
    kb16[kilobytes::KB16-2..kilobytes::KB16].copy_from_slice(&[0xbe, 0xef]);
    let cart = Cartridge::new_test(&kb16, &[]);
    let mapper = Mapper3::new(cart);
    assert_eq!(mapper.read8(0xfffe), 0xbe);
    assert_eq!(mapper.read8(0xffff), 0xef);
  }
}