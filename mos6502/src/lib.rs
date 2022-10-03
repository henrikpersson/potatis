mod address_mode;
mod instructions;
pub mod cpu;
#[cfg(not(target_arch = "wasm32"))]
pub mod debugger;
pub mod mos6502;
pub mod memory;


#[cfg(target_arch = "wasm32")]
pub mod debugger {
  use crate::{cpu::Cpu, instructions::Instruction};

  #[derive(Default)]
  pub struct Debugger;

  impl Debugger {
    pub fn on_tick(&mut self, _: &Cpu, _: &Instruction) {}
  } 
}