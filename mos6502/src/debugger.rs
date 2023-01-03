use getch::Getch;
use std::{fmt::Write, collections::{VecDeque}, ops::RangeInclusive};
use crate::{cpu::{Cpu, Reg}, instructions::{Instruction, Opcode}};

const BACKTRACE_LIMIT: usize = 11;

pub struct Debugger {
  stdin: Getch,
  breakpoints: Vec<Breakpoint>,
  last_pc: Option<u16>,
  suspended: bool,
  verbose: bool,
  backtrace: VecDeque<BacktraceEntry>,
  watches: Vec<Watch>,
}

struct BacktraceEntry {
  inst: Instruction,
  pc: u16,
  opbyte: u8
}

#[derive(PartialEq, Eq)]
pub enum Breakpoint {
  Address(u16),
  Opcode(String),
  OpcodeSequence(Vec<&'static str>)
  // TODO add support to break on opcode WITH operands
}

enum Watch {
  Range { address: RangeInclusive<u16>, state: Option<Vec<u8>>, f: Box<dyn Fn(Vec<u8>)> },
  Address { address: u16, state: Option<u8>, f: Box<dyn Fn(u8)> },
  // TODO: Reg, Flag, PC watches
}

impl Debugger {
  pub fn default() -> Self {
    Self { 
      stdin: Getch::new(), 
      breakpoints: Vec::with_capacity(2),
      last_pc: None, 
      suspended: false,
      verbose: false,
      backtrace: VecDeque::with_capacity(BACKTRACE_LIMIT),
      watches: Vec::new(),
    }
  }

  pub fn verbose(&mut self, v: bool) {
    self.verbose = v;
  }

  pub fn add_breakpoint(&mut self, breakpoint: Breakpoint) {
    let mut breakpoint = breakpoint;
    if let Breakpoint::Opcode(opstr) = &breakpoint {
      breakpoint = Breakpoint::Opcode(opstr.to_uppercase());
    }
    self.breakpoints.push(breakpoint);
  }

  pub fn on_tick(&mut self, cpu: &Cpu, next_inst: &Instruction) {
    let pc = cpu.pc();
    let opbyte = cpu.bus().read8(pc);

    self.backtrace.push_back(BacktraceEntry { inst: next_inst.clone(), pc, opbyte });
    if self.backtrace.len() == BACKTRACE_LIMIT {
      self.backtrace.remove(0);
    }
    
    if self.suspended || self.verbose {
      Debugger::print_instruction(pc, opbyte, &next_inst);
    }

    self.check_watches(cpu);

    if self.suspended {
      self.user_input(cpu);
    }
    else if self.is_breakpoint(pc, next_inst.opcode()) {
      self.suspend(cpu, pc);
    }

    self.last_pc = Some(pc);
  }

  fn is_breakpoint(&self, pc: u16, opcode: Opcode) -> bool {
    for b in &self.breakpoints {
      match b {
        Breakpoint::Address(addr) => {
          if *addr == pc {
            return true
          }
        },
        Breakpoint::Opcode(opstr) => {
          if *opstr == opcode.to_string() {
            return true;
          }
        },
        Breakpoint::OpcodeSequence(seq) => {
          let history: Vec<String> = self.backtrace.iter()
            .rev()
            .take(seq.len())
            .map(|b| b.inst.opcode().to_string())
            .collect();
          let upper: Vec<String> = seq.iter()
            .rev()
            .map(|&s| s.to_uppercase())
            .collect();
          if history == upper {
            return true;
          }
        },
      }
    }
    false
  }

  pub fn watch_memory_range(&mut self, range: RangeInclusive<u16>, f: impl Fn(Vec<u8>) + 'static) {
    let watch = Watch::Range { address: range, state: None, f: Box::new(f) };
    self.watches.push(watch)
  }

  pub fn watch_memory(&mut self, address: u16, f: impl Fn(u8) + 'static) {
    let watch = Watch::Address { address, state: None, f: Box::new(f) };
    self.watches.push(watch)
  }

  fn check_watches(&mut self, cpu: &Cpu) {
    for watch in self.watches.iter_mut() {
      match watch {
        Watch::Range { address, state, f } => {
          // let start = *address.start() as usize;
          let current_state: Vec<u8> = cpu.bus().read_range(address.clone());
          if state.as_ref() != Some(&current_state) {
            *state = Some(current_state.clone());
            f(current_state);
          }
        }
        Watch::Address { address, state, f } => {
          let current_state = cpu.bus().read8(*address);
          if *state != Some(current_state) {
            *state = Some(current_state);
            f(current_state);
          }
        }
      }
    }
  }

  pub fn dump_backtrace(&mut self) {
    println!("...");
    for entry in self.backtrace.iter() {
      Debugger::print_instruction(entry.pc, entry.opbyte, &entry.inst);
    }
  }

  pub fn enable(&mut self) { // TODO: better API
    self.dump_backtrace();
    self.suspended = true;
  }

  pub fn last_opcode(&self) -> u8 {
    self.backtrace.back().unwrap().opbyte
  }

  fn suspend(&mut self, cpu: &Cpu, address: u16) {
    self.suspended = true;
    if !self.verbose {
      // Print some instructions if we hit a break and we're not verbose already.
      self.dump_backtrace();
    }
    println!("break at {:#06x}. step: <space>, cpu: <enter>, stack: <s>, continue: <c>, mute & continue: <m>", address);
    self.user_input(cpu);
  }

  fn user_input(&mut self, cpu: &Cpu) {
    let ch = self.stdin.getch().unwrap();
    match ch {
      0x20 => (), // Space, step
      0x0a => { // Enter
        println!("{:?}", cpu);
        println!("{}", cpu);
        self.user_input(cpu);
      }
      b'c' => {
        println!("continuing...");
        self.suspended = false;
      }
      b'm' => {
        // TODO: Only mute current suspended address, not everything.
        println!("continuing...");
        self.suspended = false;
        self.breakpoints.clear()
      }
      b's' => {
        self.dump_stack(cpu);
        self.user_input(cpu);
      }
      _ => {
        println!("Unknown debugger command: {}", ch);
        self.user_input(cpu);
      }
    }
  }

  fn dump_stack(&self, cpu: &Cpu) {
    for a in Cpu::STACK_TOP..=Cpu::STACK_BOTTOM {
      print!("{:#06x}: {:#04x}", a, cpu.bus().read8(a as u16));
      if a as u8 == cpu[Reg::SP] {
        print!(" <----");
      }
      println!();
    }
  }

  fn print_instruction(pc: u16, opbyte: u8, inst: &Instruction) {
    let mut pc_str = String::new();
    write!(&mut pc_str, "{:#06x}", pc).unwrap();
  
    let mut opbyte_str = String::new();
    write!(&mut opbyte_str, "{:#04x}", opbyte).unwrap();
  
    // TODO move to Into<String> for Instruction?
    let mut operands_str = String::new();
    for operand in inst.operands() {
      write!(&mut operands_str, "{:#04x} ", operand).unwrap();
    }
  
    let mut mnemonic_str = String::new();
    write!(&mut mnemonic_str, "{:?} {:?} {}", inst.opcode(), inst.mode(), operands_str).unwrap();
  
    // let ascii_operand = if inst.mode() == AddressMode::Imm {
    //   String::from_utf8(vec![inst.operands()[0]]).unwrap_or_default()
    // } else {
    //   String::new()
    // };
  
    // if ascii_operand.is_empty() {
      println!("{:<10} {} {:<10} {}", pc_str, opbyte_str, operands_str, mnemonic_str);
    // } else {
    //   println!("{:<10} {} {:<10} {} (b'{}')", pc_str, opbyte_str, operands_str, mnemonic_str, ascii_operand);
    // }
  }
}