#[macro_use]
extern crate lazy_static;

pub use mos6502;

mod mappers;
mod nesbus;
mod ppu;
mod fonts;

pub mod frame;
pub mod cartridge;
pub mod nes;
pub mod joypad;

pub mod error {
  #[derive(Debug)]
  pub enum PotatisError {
    IO(std::io::Error),
    InvalidCartridge(&'static str),
    NotYetImplemented(String),
  }

  impl std::error::Error for PotatisError {}

  impl std::fmt::Display for PotatisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{:?}", self)
    }
  }

  impl From<std::io::Error> for PotatisError {
    fn from(e: std::io::Error) -> Self {
      PotatisError::IO(e)
    }
  }
}