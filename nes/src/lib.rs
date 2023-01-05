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
pub mod display;

pub mod trace {
  #[derive(Debug)]
  pub enum Tag { PpuTiming, Cpu }

  impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{:?}", self)
    }
  }

  #[macro_export]
  macro_rules! trace {
    ($enum:ident::$variant:ident, $($t:tt)*) => {{
      if let Ok(env) = std::env::var("TRACE_TAG") {
        let st: String = $crate::trace::Tag::$variant.to_string();
        if st == env {
          eprintln!($($t)*);
        }
      }
    }};
  }
}

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