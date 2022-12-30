use core::panic;
use common::kilobytes;
use mos6502::memory::Bus;

use crate::cartridge::{Cartridge, Mirroring};

use super::Mapper;

#[derive(Debug, PartialEq, Eq)]
enum PrgBankMode {
  Switch32Kb,
  FixFirstLowerSwitchUpper,
  FixLastUpperSwitchLower,
}

impl From<u8> for PrgBankMode {
  fn from(i: u8) -> Self {
    match i {
      0 | 1 => PrgBankMode::Switch32Kb,
      2 => PrgBankMode::FixFirstLowerSwitchUpper,
      3 => PrgBankMode::FixLastUpperSwitchLower,
      _ => panic!()
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
enum ChrBankMode {
  Switch8Kb,
  SwitchTwo4KbBanks
}

pub struct MMC1 {
  cart: Cartridge,
  prg_ram: [u8; kilobytes::KB8],
  

  prg_rom_banks: Vec<Vec<u8>>,
  prg_rom_bank_mode: PrgBankMode,
  selected_prg_bank: u8,

  chr_rom_banks: Vec<Vec<u8>>,
  chr_rom_bank_mode: ChrBankMode,
  selected_chr_bank_0: u8,
  selected_chr_bank_1: u8,
  mirroring: Mirroring,

  num_shift_writes: u8,
  shift_register: u8,
}

impl MMC1 {
  pub fn new(cart: Cartridge) -> MMC1 {
    let chunks = cart.prg().chunks_exact(kilobytes::KB16);
    assert!(chunks.remainder().is_empty());
    let prg_rom_banks: Vec<Vec<u8>> = chunks.map(|s| s.to_vec()).collect();

    let chunks = cart.chr().chunks_exact(kilobytes::KB4);
    assert!(chunks.remainder().is_empty());
    let chr_rom_banks: Vec<Vec<u8>> = chunks.map(|s| s.to_vec()).collect();

    let mirroring = cart.mirroring();
    MMC1 {
      cart,
      prg_ram: [0; kilobytes::KB8],
      prg_rom_banks,
      // "seems to reliably power on in the last bank (by setting the "PRG ROM bank mode" to 3)"
      // https://www.nesdev.org/wiki/MMC1#Control_(internal,_$8000-$9FFF)
      prg_rom_bank_mode: PrgBankMode::FixLastUpperSwitchLower,
      chr_rom_banks,
      chr_rom_bank_mode: ChrBankMode::Switch8Kb,
      selected_chr_bank_0: 0,
      selected_chr_bank_1: 0,
      shift_register: 0,
      num_shift_writes: 0,
      selected_prg_bank: 0,
      mirroring
     }
  }

  fn reset_shift_register(&mut self) {
    self.num_shift_writes = 0;
    self.shift_register = 0;
  }

  fn write_to_shift_register(&mut self, val: u8, address: u16) {
    if common::bits::is_signed(val) {
      self.reset_shift_register();
      return;
    }

    let bit_to_write = val & 1;
    
    // shift in bit, lsb first. max width of shift reg is 5 bits, so we only shift to bit 4.
    self.shift_register = (self.shift_register >> 1) | (bit_to_write << 4);

    self.num_shift_writes += 1;

    if self.num_shift_writes == 5 {
      match address {
        0x8000..=0x9fff => { // Control
          self.update_control_register(self.shift_register);
        }
        0xa000..=0xbfff => { // CHR bank 0
          self.switch_lower_chr_bank(self.shift_register)
        }
        0xc000..=0xdfff => { // CHR bank 1
          self.switch_upper_chr_bank(self.shift_register)
        }
        0xe000..=0xffff => { // PRG bank
          self.selected_prg_bank = self.shift_register & 0b01111;
          // println!("selected_prg_bank: {}", self.selected_prg_bank);
        }
        _ => panic!("unknown register")
      }

      self.reset_shift_register()
    }
  }

  fn switch_lower_chr_bank(&mut self, selected_bank: u8) {
    // https://www.nesdev.org/wiki/MMC1#iNES_Mapper_001
    // println!("{} to CHR bank 0", selected_bank);
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => self.selected_chr_bank_0 = selected_bank >> 1,
      ChrBankMode::SwitchTwo4KbBanks => self.selected_chr_bank_0 = selected_bank,
    }
  }

  fn switch_upper_chr_bank(&mut self, selected_bank: u8) {
    // https://www.nesdev.org/wiki/MMC1#iNES_Mapper_001
    // println!("{} to CHR bank 1", selected_bank);
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => (), // (ignored in 8 KB mode)
      ChrBankMode::SwitchTwo4KbBanks => self.selected_chr_bank_1 = selected_bank,
    }
  }

  fn update_control_register(&mut self, val: u8) {
    self.mirroring = match val & 0b11 {
      0 => Mirroring::SingleScreenLower,
      1 => Mirroring::SingleScreenUpper,
      2 => Mirroring::Vertical,
      3 => Mirroring::Horizontal,
      _ => unreachable!()
    };

    // println!("setting mirroring to: {:?}", self.mirroring);

    let chr_rom_bank_mode = (val & 0b10000) >> 4;
    self.chr_rom_bank_mode = match chr_rom_bank_mode {
      0 => ChrBankMode::Switch8Kb,
      _ => ChrBankMode::SwitchTwo4KbBanks
    };
    // println!("setting chr rom bank mode: {:?}", self.chr_rom_bank_mode);

    let prg_rom_bank_mode = (val & 0b01100) >> 2;
    self.prg_rom_bank_mode = prg_rom_bank_mode.into();
    // println!("setting prg rom bank mode: {:?}", self.prg_rom_bank_mode);
  }

  fn lower_prg_bank(&self) -> &Vec<u8> {
    match self.prg_rom_bank_mode {
      PrgBankMode::Switch32Kb => &self.prg_rom_banks[self.selected_prg_bank as usize >> 1],
      PrgBankMode::FixFirstLowerSwitchUpper => &self.prg_rom_banks[0],
      PrgBankMode::FixLastUpperSwitchLower => &self.prg_rom_banks[self.selected_prg_bank as usize],
    }
  }

  fn upper_prg_bank(&self) -> &Vec<u8> {
    // &self.rom_banks[self.selected_rom as usize]
    match self.prg_rom_bank_mode {
      PrgBankMode::Switch32Kb => &self.prg_rom_banks[(self.selected_prg_bank as usize >> 1) + 1],
      PrgBankMode::FixFirstLowerSwitchUpper => &self.prg_rom_banks[self.selected_prg_bank as usize],
      PrgBankMode::FixLastUpperSwitchLower => self.prg_rom_banks.last().unwrap(),
    }
  }

  fn lower_chr_bank(&self) -> &Vec<u8> {
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => &self.chr_rom_banks[self.selected_chr_bank_0 as usize],
      ChrBankMode::SwitchTwo4KbBanks => &self.chr_rom_banks[self.selected_chr_bank_0 as usize],
    }
  }

  fn upper_chr_bank(&self) -> &Vec<u8> {
    // &self.rom_banks[self.selected_rom as usize]
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => &self.chr_rom_banks[self.selected_chr_bank_0 as usize + 1],
      ChrBankMode::SwitchTwo4KbBanks => &self.chr_rom_banks[self.selected_chr_bank_1 as usize],
    }
  }

  fn write_chr_ram(&mut self, val: u8, address: u16) {
    if !self.cart.chr_ram_mode() {
      panic!("This cart is not configured for CHR RAM! Legit write?")
    }

    // TODO: don't to this every write op
    let mut ram: Vec<&mut u8> = self.chr_rom_banks.iter_mut().flatten().collect();
    *ram[address as usize] = val;
  }
}

impl Mapper for MMC1 {
  fn mirroring(&self) -> crate::cartridge::Mirroring {
    self.mirroring
  }
}

impl Bus for MMC1 {
  
  fn read8(&self, address: u16) -> u8 {
    // println!("Read: {:#06x}", address);
    match address {
      // PPU
      0x0000..=0x0fff => self.lower_chr_bank()[address as usize],
      0x1000..=0x1fff => self.upper_chr_bank()[address as usize - 0x1000],

      // CPU
      0x6000..=0x7fff => self.prg_ram[address as usize - 0x6000],
      0x8000..=0xbfff => self.lower_prg_bank()[address as usize - 0x8000],
      0xc000..=0xffff => self.upper_prg_bank()[address as usize - 0xc000],
      _ => panic!("unknown mmc1 memory range") // TODO: In most mappers, banks past the end of PRG or CHR ROM show up as mirrors of earlier banks.
    }
  }

  fn write8(&mut self, val: u8, address: u16) {
    // println!("Write: {:#06x} {:#04x}", address, val);
    match address {
      // PPU
      0x0000..=0x1fff => self.write_chr_ram(val, address),
      // 0x1000..=0x1fff => self.upper_chr_bank().borrow_mut()[address as usize - 0x1000] = val,

      // CPU
      0x6000..=0x7fff => self.prg_ram[address as usize - 0x6000] = val,
      0x8000..=0xffff => self.write_to_shift_register(val, address),
      _ => panic!("writing to rom")
    }
  }
}