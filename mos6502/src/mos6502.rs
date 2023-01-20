use alloc::boxed::Box;

use crate::{cpu::Cpu, memory::Bus, debugger::Debugger};

pub struct Mos6502 {
  cpu: Cpu,
  #[allow(dead_code)]
  debugger: Debugger,
  total_cycles: usize,
  total_ticks: usize,
}

impl Mos6502 {
  pub fn new(cpu: Cpu) -> Self {
    let debugger = Debugger::default();
    Self { cpu, debugger, total_cycles: 0, total_ticks: 0 }
  }

  pub fn cpu(&self) -> &Cpu {
    &self.cpu
  }

  pub fn cpu_mut(&mut self) -> &mut Cpu {
    &mut self.cpu
  }

  pub fn bus(&self) -> &Box<dyn Bus> {
    self.cpu.bus()
  }

  #[cfg(feature = "debugger")]
  pub fn debugger(&mut self) -> &mut Debugger {
    &mut self.debugger
  }

  pub fn ticks(&self) -> usize {
    self.total_ticks
  }

  pub fn cycles(&self) -> usize {
    self.total_cycles
  }

  pub fn inc_cycles(&mut self, c: usize) {
    self.total_cycles += c;
  }

  // The clock ticks Hzhzhzhz
  pub fn tick(&mut self) -> usize {
    let inst = self.cpu.fetch_next_instruction();

    #[cfg(feature = "debugger")]
    self.debugger.on_tick(&self.cpu, &inst);

    let cycles = self.cpu.execute(&inst);

    self.total_cycles += cycles;
    self.total_ticks += 1;
    cycles
  }
}
