
use std::{fmt::Display, path::PathBuf};

use common::kilobytes;

use crate::error::PotatisError;

#[derive(Debug)]
#[repr(C)]
struct Header {
  magic: [u8; 4],
  prg_rom_size: u8,
  chr_rom_size: u8,
  flags6: u8,
  flags7: u8,
  flags8: u8,
  flags9: u8,
  flags10: u8,
  padding: [u8; 5]
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

pub struct Cartridge {
  mirroring: Mirroring,
  prg_rom: Vec<u8>,
  chr_rom: Vec<u8>,
  mapper: Mapper,
}

impl Cartridge {
  const MAGIC: [u8; 4] = [0x4e, 0x45, 0x53, 0x1a];

  pub fn blow_dust(path: PathBuf) -> Result<Cartridge, PotatisError> {
    let bin = std::fs::read(path)?;

    if bin[0..4] != Self::MAGIC {
      return Err(PotatisError::InvalidCartMagic);
    }
    
    // TODO: should probably use nom or something, or just read manually, it's 16bytes..
    let header: Header = unsafe { std::ptr::read(bin.as_ptr() as *const _) };

    let format = if (header.flags7 & 0x0c) == 0x08 { Format::Nes2 } else { Format::Ines };
    if format == Format::Nes2 {
      return Err(PotatisError::NotYetImplemented("NES 2.0".into()));
    }
    
    let mapper = Mapper::try_from(&header)?;

    if header.magic != Self::MAGIC {
      return Err(PotatisError::InvalidCartMagic);
    }

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

    let prg_size = (header.prg_rom_size as usize) * kilobytes::KB16; 
    let prg_start = 16usize; // sizeof header
    let prg_end = prg_start + prg_size;
    let prg_rom = bin[prg_start..prg_end].to_vec();

    let chr_rom = if header.chr_rom_size == 0 {
      vec![]
    } else {
      let chr_start = prg_end;
      let chr_size = (header.chr_rom_size as usize) * kilobytes::KB8;
      bin[chr_start..(chr_start + chr_size)].to_vec()
    };
    
    Ok(Cartridge {
      prg_rom,
      chr_rom,
      mirroring,
      mapper,
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

  #[cfg(test)]
  pub fn new_test(prg_rom: &[u8], chr_rom: &[u8]) -> Self {
    Cartridge {
      prg_rom: prg_rom.to_vec(),
      chr_rom: chr_rom.to_vec(),
      mirroring: Mirroring::Vertical,
      mapper: Mapper::Nrom,
    }
  }
}

impl Display for Cartridge {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Mapper: {:?}, Mirroring: {:?}, CHR ROM: {}", self.mapper, self.mirroring, self.chr_rom.len())
  }
}