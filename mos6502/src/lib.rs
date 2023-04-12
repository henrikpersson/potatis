#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod address_mode;
pub mod cpu;
#[cfg(feature = "debugger")]
pub mod debugger;
mod instructions;
pub mod memory;
pub mod mos6502;

#[cfg(not(feature = "debugger"))]
pub mod debugger {
  #[derive(Default)]
  pub struct Debugger;
}
