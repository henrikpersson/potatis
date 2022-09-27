use core::panic;
use common::kilobytes;
use mos6502::memory::Bus;

use crate::cartridge::Cartridge;

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
  prg_ram: [u8; kilobytes::KB8],

  prg_rom_banks: Vec<Vec<u8>>,
  prg_rom_bank_mode: PrgBankMode,
  selected_prg_bank: u8,

  // chr_rom: Vec<Vec<u8>>,
  chr_rom_banks: Vec<Vec<u8>>,
  chr_rom_bank_mode: ChrBankMode,
  selected_chr_bank: u8,
  chr_ram_mode: bool,

  num_shift_writes: u8,
  shift_register: u8,
}

impl MMC1 {
  pub fn new(cart: Cartridge) -> MMC1 {
    let chunks = cart.prg().chunks_exact(kilobytes::KB16);
    assert!(chunks.remainder().len() == 0);
    let prg_rom_banks = chunks.map(|s| s.to_vec()).collect();

    let chr_ram_mode: bool;
    let chr_rom_banks = if cart.chr().len() == 0 {
      // CHR RAM
      chr_ram_mode = true;
      vec![vec![0; kilobytes::KB4]; 2]
    } else {
      chr_ram_mode = false;
      let chunks = cart.chr().chunks_exact(kilobytes::KB4);
      assert!(chunks.remainder().len() == 0);
      chunks.map(|s| s.to_vec()).collect()
    };

    MMC1 { 
      prg_ram: [0; kilobytes::KB8],
      prg_rom_banks: prg_rom_banks, 
      prg_rom_bank_mode: PrgBankMode::FixFirstLowerSwitchUpper, // TODO don't understand the default yet..
      chr_rom_banks: chr_rom_banks,
      chr_rom_bank_mode: ChrBankMode::Switch8Kb, // TODO: default?
      chr_ram_mode,
      selected_chr_bank: 0,
      shift_register: 0,
      num_shift_writes: 0,
      selected_prg_bank: 0,
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
          // println!("selected_rom: {}", self.selected_prg_bank);
        }
        _ => panic!("unknown register")
      }

      self.reset_shift_register()
    }
  }

  fn switch_lower_chr_bank(&mut self, selected_bank: u8) {
    // https://www.nesdev.org/wiki/MMC1#iNES_Mapper_001
    // println!("{:05b} to CHR bank 0", selected_bank);
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => {
        if selected_bank != 0 {
          todo!("handle bank switch")
        }
        self.selected_chr_bank = selected_bank;
      },
      ChrBankMode::SwitchTwo4KbBanks => todo!(),
    }
  }

  fn switch_upper_chr_bank(&mut self, _selected_bank: u8) {
    // https://www.nesdev.org/wiki/MMC1#iNES_Mapper_001
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => (), // (ignored in 8 KB mode)
      ChrBankMode::SwitchTwo4KbBanks => todo!(),
    }
  }

  fn update_control_register(&mut self, val: u8) {
    // 01010
    // println!("{:05b} control", val);
    let _mirroring = val & 0b00011; // TODO, gfx stuff
    let chr_rom_bank_mode = (val & 0b10000) >> 4;
    self.chr_rom_bank_mode = match chr_rom_bank_mode {
      0 => ChrBankMode::Switch8Kb,
      _ => ChrBankMode::SwitchTwo4KbBanks
    };

    if self.chr_rom_bank_mode == ChrBankMode::SwitchTwo4KbBanks {
      todo!("implement this bank mode")
    }

    let prg_rom_bank_mode = (val & 0b01100) >> 2;
    self.prg_rom_bank_mode = prg_rom_bank_mode.into();
    // println!("setting prg rom bank mode: {:?}", self.prg_rom_bank_mode);
  }

  fn lower_prg_bank(&self) -> &Vec<u8> {
    match self.prg_rom_bank_mode {
      PrgBankMode::Switch32Kb => todo!(),
      PrgBankMode::FixFirstLowerSwitchUpper => &self.prg_rom_banks[0],
      PrgBankMode::FixLastUpperSwitchLower => todo!(),
    }
  }

  fn upper_prg_bank(&self) -> &Vec<u8> {
    // &self.rom_banks[self.selected_rom as usize]
    match self.prg_rom_bank_mode {
      PrgBankMode::Switch32Kb => todo!(),
      PrgBankMode::FixFirstLowerSwitchUpper => &self.prg_rom_banks[self.selected_prg_bank as usize],
      PrgBankMode::FixLastUpperSwitchLower => todo!(),
    }
  }

  fn lower_chr_bank(&self) -> &Vec<u8> {
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => &self.chr_rom_banks[0],
      ChrBankMode::SwitchTwo4KbBanks => todo!(),
    }
  }

  fn upper_chr_bank(&self) -> &Vec<u8> {
    // &self.rom_banks[self.selected_rom as usize]
    match self.chr_rom_bank_mode {
      ChrBankMode::Switch8Kb => &self.chr_rom_banks[1],
      ChrBankMode::SwitchTwo4KbBanks => todo!(),
    }
  }

  fn write_chr_ram(&mut self, val: u8, address: u16) {
    if !self.chr_ram_mode {
      panic!("writing to CHR without CHR RAM enabled.. is this correct?")
    }

    // TODO: don't to this every write op
    let mut ram: Vec<&mut u8> = self.chr_rom_banks.iter_mut().flatten().collect();
    *ram[address as usize] = val;
  }
}

impl Bus for MMC1 {
  fn read8(&self, address: u16) -> u8 {
    // TODO: In most mappers, banks past the end of PRG or CHR ROM show up as mirrors of earlier banks.
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