#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod address_mode;
mod instructions;
pub mod cpu;
#[cfg(feature = "debugger")]
pub mod debugger;
pub mod mos6502;
pub mod memory;

#[cfg(not(feature = "debugger"))]
pub mod debugger {
  #[derive(Default)]
  pub struct Debugger;
}