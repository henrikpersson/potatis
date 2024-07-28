#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

pub use mos6502;

mod fonts;
mod mappers;
mod nesbus;
mod ppu;

pub mod cartridge;
pub mod frame;
pub mod joypad;
pub mod nes;

pub mod trace {
  #[derive(Debug)]
  pub enum Tag {
    PpuTiming,
    Cpu,
  }

  impl core::fmt::Display for Tag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
      write!(f, "{:?}", self)
    }
  }

  #[macro_export]
  macro_rules! trace {
    ($enum:ident::$variant:ident, $($t:tt)*) => {{
      #[cfg(feature = "std")]
      if let Ok(env) = std::env::var("TRACE_TAG") {
        let st: String = $crate::trace::Tag::$variant.to_string();
        if st == env {
          eprintln!($($t)*);
        }
      }
    }};
  }
}
