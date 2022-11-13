
use std::{fmt::Display, path::PathBuf};
use common::kilobytes;
use crate::error::PotatisError;

pub const MAGIC: [u8; 4] = [0x4e, 0x45, 0x53, 0x1a];
pub const HEADER_SIZE: usize = 16;
const PRG_ROM_BLOCK_SIZE: usize = kilobytes::KB16;
const CHR_ROM_BLOCK_SIZE: usize = kilobytes::KB8;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Header {
  magic: [u8; 4],
  prg_rom_blocks: u8,
  chr_rom_blocks: u8,
  flags6: u8,
  flags7: u8,
  flags8: u8,
  flags9: u8,
  flags10: u8,
  padding: [u8; 5]
}

impl Header {
  pub fn parse(bin: &[u8]) -> Result<Header, PotatisError> {
    if bin.len() < HEADER_SIZE {
      return Err(PotatisError::InvalidCartridge("strange size"));
    }

    let magic = &bin[0..4];
    if magic != MAGIC {
      println!("{:?}", magic);
      return Err(PotatisError::InvalidCartridge("magic"));
    }

    Ok(Header {
      magic: magic.try_into().map_err(|_| PotatisError::InvalidCartridge("magic 2"))?,
      prg_rom_blocks: bin[4],
      chr_rom_blocks: bin[5],
      flags6: bin[6],
      flags7: bin[7],
      flags8: bin[8],
      flags9: bin[9],
      flags10: bin[10],
      padding: bin[11..16].try_into().map_err(|_| PotatisError::InvalidCartridge("padding"))?,
    })
  }

  pub fn total_size_excluding_header(&self) -> usize {
    (self.prg_rom_blocks as usize * PRG_ROM_BLOCK_SIZE) + (self.chr_rom_blocks as usize * CHR_ROM_BLOCK_SIZE)
  }
}

#[derive(Debug, PartialEq, Eq)]
enum Format { Nes2, Ines }

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mapper {
  Nrom = 0,
  Mmc1 = 1,
  Mapper3 = 3,
}

impl TryFrom<&Header> for Mapper {
  type Error = PotatisError;

  fn try_from(header: &Header) -> Result<Self, Self::Error> {
    let id: u8 = (header.flags7 & 0xf0) | header.flags6 >> 4;
    match id {
      0 => Ok(Mapper::Nrom),
      1 => Ok(Mapper::Mmc1),
      3 => Ok(Mapper::Mapper3),
      _ => Err(PotatisError::NotYetImplemented(format!("Mapper {}", id)))
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mirroring {
  Horizontal,
  Vertical,
  FourScreen
}

#[derive(PartialEq, Eq)]
pub struct Cartridge {
  mirroring: Mirroring,
  prg_rom: Vec<u8>,
  chr_rom: Vec<u8>,
  mapper: Mapper,
  uses_chr_ram: bool,
}

impl Cartridge {
  pub fn blow_dust(path: PathBuf) -> Result<Cartridge, PotatisError> {
    let bin = std::fs::read(path)?;
    Self::load(&bin)
  }

  pub fn load(bin: &[u8]) -> Result<Cartridge, PotatisError> {
    if bin.len() < HEADER_SIZE + PRG_ROM_BLOCK_SIZE || bin[0..4] != MAGIC {
      return Err(PotatisError::InvalidCartridge("strange size"));
    }
    
    let header = Header::parse(bin)?;
    if header.magic != MAGIC {
      return Err(PotatisError::InvalidCartridge("magic"));
    }

    let format = if (header.flags7 & 0x0c) == 0x08 { Format::Nes2 } else { Format::Ines };
    if format == Format::Nes2 {
      return Err(PotatisError::NotYetImplemented("NES 2.0".into()));
    }
    
    let mapper = Mapper::try_from(&header)?;

    let skip_trainer = header.flags6 & 0b100 != 0;
    if skip_trainer || (header.flags6 & (1 << 3)) != 0 {
      return Err(PotatisError::NotYetImplemented("Trainer".into()));
    }

    if header.flags6 & 0b10 != 0 {
      return Err(PotatisError::NotYetImplemented("PRG RAM".into()));
    }

    if header.flags6 & 0b1000 != 0 {
      return Err(PotatisError::NotYetImplemented("cartidge fiddles w VRAM address space..".into()));
    }

    let mut mirroring = match header.flags6 & 1 {
      1 => Mirroring::Vertical,
      _ => Mirroring::Horizontal
    };

    if header.flags6 & 0b1000 != 0 {
      mirroring = Mirroring::FourScreen
    }

    let prg_size = (header.prg_rom_blocks as usize) * PRG_ROM_BLOCK_SIZE; 
    let prg_start = HEADER_SIZE;
    let prg_end = prg_start + prg_size;
    let prg_rom = bin[prg_start..prg_end].to_vec();

    let mut uses_chr_ram = false;
    let chr_rom = if header.chr_rom_blocks == 0 {
      uses_chr_ram = true;
      vec![0; CHR_ROM_BLOCK_SIZE]
    } else {
      let chr_start = prg_end;
      let chr_size = (header.chr_rom_blocks as usize) * CHR_ROM_BLOCK_SIZE;
      bin[chr_start..(chr_start + chr_size)].to_vec()
    };
    
    Ok(Cartridge {
      prg_rom,
      chr_rom,
      mirroring,
      mapper,
      uses_chr_ram
    })
  }

  pub fn mirroring(&self) -> Mirroring {
    self.mirroring
  }

  pub fn prg(&self) -> &[u8] {
    &self.prg_rom
  }

  pub fn chr(&self) -> &[u8] {
    &self.chr_rom
  }

  pub fn chr_mut(&mut self) -> &mut [u8] {
    &mut self.chr_rom
  }

  pub fn mapper(&self) -> Mapper {
    self.mapper
  }

  pub fn chr_ram_mode(&self) -> bool {
    self.uses_chr_ram
  }

  #[cfg(test)]
  pub fn new_test(prg_rom: &[u8], chr_rom: &[u8]) -> Self {
    Cartridge {
      prg_rom: prg_rom.to_vec(),
      chr_rom: chr_rom.to_vec(),
      mirroring: Mirroring::Vertical,
      mapper: Mapper::Nrom,
      uses_chr_ram: false,
    }
  }
}

impl Display for Cartridge {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let chr_ram_or_rom = if self.chr_ram_mode() { " RAM" } else { "" };
    write!(f, 
      "Mapper: {:?}, Mirroring: {:?}, CHR{}: {}x{}K, PRG: {}x{}K", 
      self.mapper, self.mirroring, chr_ram_or_rom,
      self.chr().len() / CHR_ROM_BLOCK_SIZE, CHR_ROM_BLOCK_SIZE / 1000,
      self.prg().len() / PRG_ROM_BLOCK_SIZE, PRG_ROM_BLOCK_SIZE / 1000
    )
  }
}

impl std::fmt::Debug for Cartridge {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Cartridge")
      .field("mirroring", &self.mirroring)
      .field("prg_rom", &"...")
      .field("chr_rom", &"...")
      .field("mapper", &self.mapper)
      .field("uses_chr_ram", &self.uses_chr_ram)
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::Cartridge;

  fn assert_cart(r: &[u8], s: &str) {
    assert_eq!(Cartridge::load(r).unwrap().to_string(), s);
  }

  #[test]
  fn cart_invalid_len() {
    assert!(Cartridge::load(&[b'N', b'E', b'S']).is_err())
  }

  #[test]
  fn cart_valid_nrom() {
    assert_cart(
    include_bytes!("../../test-roms/nestest/nestest.nes"),
    "Mapper: Nrom, Mirroring: Horizontal, CHR: 1x8K, PRG: 1x16K"
  ) ;
  }

  #[test]
  fn cart_valid_mmc1() {
    assert_cart(
      include_bytes!("../../test-roms/nes-test-roms/instr_test-v5/official_only.nes"),
      "Mapper: Mmc1, Mirroring: Vertical, CHR RAM: 1x8K, PRG: 16x16K"
    );
  }

  #[test]
  fn cart_valid_nrom_chr_ram() {
    assert_cart(
      include_bytes!("../../test-roms/nes-test-roms/blargg_ppu_tests_2005.09.15b/vram_access.nes"),
      "Mapper: Nrom, Mirroring: Horizontal, CHR RAM: 1x8K, PRG: 1x16K"
    );
  }
}