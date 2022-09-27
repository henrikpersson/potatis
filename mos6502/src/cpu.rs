use std::ops::{Index, IndexMut};
use crate::address_mode::AddressMode;
use crate::memory::Bus;
use crate::instructions::{Instruction, Opcode};

#[derive(Debug, Clone, Copy)]
pub enum Reg {
  AC = 0,
  X = 1,
  Y = 2,
  SP = 3
}

#[derive(Debug, Clone, Copy)]
pub enum Flag {
  BUNUSEDMASK = 0b00110000,

  N = 7, // Negative
  V = 6, // Overflow

  // These are only set when SR is pushed to stack by software. Should never be accessed by the CPU.
  UNUSED = 5,
  B = 4, // Break

  D = 3, // Decimal (use BCD for arithmetics)
  I = 2, // Interrupt (IRQ disable)
  Z = 1, // Zero
  C = 0, // Carry
}

pub struct Cpu {
  pc: u16,
  flags: [u8; 8], // All flags fit in one byte. But this is more readable. Or is it???
  regs: [u8; 4],
  mem: Box<dyn Bus>,
  extra_cycles: usize,
}

impl Cpu {
  // LIFO, top-down, 8 bit range, 0x0100 - 0x01FF
  pub const STACK_TOP: usize = 0x0100;
  pub const STACK_BOTTOM: usize = 0x01ff;

  const NMI_VECTOR: u16 = 0xfffa;
  const RESET_VECTOR: u16 = 0xfffc;
  const IRQ_VECTOR: u16 = 0xfffe;

  pub fn new(mem: Box<dyn Bus>) -> Self {
    Self { 
      pc: 0,
      flags: [0; 8],
      regs: [0; 4],
      mem,
      extra_cycles: 0,
    }
  }

  pub fn fetch_next_instruction(&mut self) -> Instruction {
    let pc = self.pc();

    let opbyte = self.mem.read8(pc);
    let operand1 = self.mem.read8(pc + 1);
    let operand2 = self.mem.read8(pc + 2);

    // Decode
    Instruction::disassemble(opbyte, operand1, operand2)
  }

  pub fn execute(&mut self, inst: &Instruction) -> usize {
    let operands = &inst.operands();
    let pc_before_exec = self.pc();

    // println!("exec: {:?}", inst.opcode());
    // println!("nmi: {:#06x}", mem.read16(Self::NMI_VECTOR));
    // println!("reset: {:#06x}", mem.read16(Self::RESET_VECTOR));
    // println!("irq/brk: {:#06x}", mem.read16(Self::IRQ_VECTOR));

    self.extra_cycles = 0;

    match inst.opcode() {
      Opcode::JAM => panic!("jammed"), //println!("WARN!!!! JAMMED"), // TODO, Illegal opcode.. might be used somewhere as a nice HLT?
      Opcode::NOP => {
        // cycles on absX nops
        if inst.mode() == AddressMode::AbsX {
          let _ = inst.resolve_operand_value(self);
        }
      },
      Opcode::DEX => self.dec_reg(Reg::X),
      Opcode::DEY => self.dec_reg(Reg::Y),
      Opcode::INX => self.inc_reg(Reg::X),
      Opcode::INY => self.inc_reg(Reg::Y),
      Opcode::DEC => {
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = val.wrapping_sub(1);
        self.flags_set_neg_zero(res);
        self.mem.write8(res, address);
      }
      Opcode::INC => {
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = val.wrapping_add(1);
        self.flags_set_neg_zero(res);
        self.mem.write8(res, address);
      }
      Opcode::DCP => {
        // DEC oper
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = val.wrapping_sub(1);
        self.mem.write8(res, address);

        // CMP oper
        self.cmp(Reg::AC, res);
      }
      Opcode::CLC => self[Flag::C] = 0,
      Opcode::CLD => self[Flag::D] = 0,
      Opcode::CLI => self[Flag::I] = 0,
      Opcode::CLV => self[Flag::V] = 0,
      Opcode::LDX => {
        let res = inst.resolve_operand_value(self);
        self[Reg::X] = res;
        self.flags_set_neg_zero(res)
      }
      Opcode::LAX => {
        let res = inst.resolve_operand_value(self);
        self[Reg::AC] = res;
        self[Reg::X] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::SAX => {
        let address = inst.resolve_operand_address(self);
        let res = self[Reg::AC] & self[Reg::X];
        self.mem.write8(res, address);
      }
      Opcode::TAX => self.mv_with_neg_zero(Reg::AC, Reg::X),
      Opcode::TAY => self.mv_with_neg_zero(Reg::AC, Reg::Y),
      Opcode::TSX => self.mv_with_neg_zero(Reg::SP, Reg::X),
      Opcode::TXA => self.mv_with_neg_zero(Reg::X, Reg::AC),
      Opcode::TXS => self[Reg::SP] = self[Reg::X],
      Opcode::TYA => self.mv_with_neg_zero(Reg::Y, Reg::AC),
      Opcode::SEC => self[Flag::C] = 1,
      Opcode::SED => self[Flag::D] = 1,
      Opcode::SEI => self[Flag::I] = 1,
      Opcode::LDA => {
        let res = inst.resolve_operand_value(self);
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res)
      }
      Opcode::LDY => {
        let res = inst.resolve_operand_value(self);
        self[Reg::Y] = res;
        self.flags_set_neg_zero(res)
      }
      Opcode::STA => {
        let address = inst.resolve_operand_address(self);
        self.mem.write8(self[Reg::AC], address);
      }
      Opcode::STX => {
        let address = inst.resolve_operand_address(self);
        self.mem.write8(self[Reg::X], address);
      }
      Opcode::STY => {
        let address = inst.resolve_operand_address(self);
        self.mem.write8(self[Reg::Y], address);
      }
      Opcode::JMP => {
        let target = inst.resolve_operand_address(self);
        self.set_pc(target);
      }
      Opcode::JSR => {
        self.push_word(self.pc() + 2);
        let target = inst.resolve_operand_address(self);
        self.set_pc(target);
      }
      Opcode::RTS => {
        let ret = self.pop_word();
        self.set_pc(ret + 1); // pull PC, PC+1 -> PC
      }
      Opcode::BRK => {
        self.push_word(self.pc() + 2);

        let mut res = self.flags_as_byte();
        res |= Flag::BUNUSEDMASK as u8; // break and 5 should always be set to 1 on stack
        self.push(res);
        self[Flag::I] = 1;

        // Jump to IRQ vector, TODO cycles
        self.set_pc(self.read16(Self::IRQ_VECTOR));
      }
      Opcode::RTI => {
        let flags = self.pop();
        self.set_flags_ignore_5_4(flags);
        let ret = self.pop_word();
        self.set_pc(ret);
      }
      Opcode::BNE => {
        self.branch_if(operands[0], |cpu| cpu[Flag::Z] == 0);
      }
      Opcode::BEQ => {
        self.branch_if(operands[0], |cpu| cpu[Flag::Z] == 1);
      }
      Opcode::BPL => {
        self.branch_if(operands[0], |cpu| cpu[Flag::N] == 0);
      }
      Opcode::BMI => {
        self.branch_if(operands[0], |cpu| cpu[Flag::N] == 1);
      }
      Opcode::BCC => {
        self.branch_if(operands[0], |cpu| cpu[Flag::C] == 0);
      }
      Opcode::BCS => {
        self.branch_if(operands[0], |cpu| cpu[Flag::C] == 1);
      }
      Opcode::BVC => {
        self.branch_if(operands[0], |cpu| cpu[Flag::V] == 0);
      }
      Opcode::BVS => {
        self.branch_if(operands[0], |cpu| cpu[Flag::V] == 1);
      }
      Opcode::CPY => {
        let val = inst.resolve_operand_value(self);
        self.cmp(Reg::Y, val);
      }
      Opcode::CPX => {
        let val = inst.resolve_operand_value(self);
        self.cmp(Reg::X, val);
      }
      Opcode::CMP => {
        let val = inst.resolve_operand_value(self);
        self.cmp(Reg::AC, val);
      }
      Opcode::SRE => {
        // LSR oper 
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = self.shift_right(val);
        self.mem.write8(res, address);
        
        // EOR oper
        let res = self[Reg::AC] ^ res;
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::LSR => {
        match inst.mode() {
          AddressMode::Impl => {
            let res = self.shift_right(self[Reg::AC]);
            self[Reg::AC] = res;
          }
          _ => {
            let (val, address) = inst.resolve_operand_value_and_address(self);
            let res = self.shift_right(val);
            self.mem.write8(res, address);
          }
        };
      }
      Opcode::SLO => {
        // ASL oper
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = self.shift_left(val);
        self.mem.write8(res, address);

        // ORA oper
        let res = self[Reg::AC] | res;
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::RLA => {
        // ROL oper
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = self.rotate_left(val);
        self.mem.write8(res, address);

        // AND oper
        let res = self[Reg::AC] & res;
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::RRA => {
        // ROR oper
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = self.rotate_right(val);
        self.mem.write8(res, address);

        // ADC oper
        self[Reg::AC] = self.add_with_carry(self[Reg::AC], res);
      }
      Opcode::ASL => {
        match inst.mode() {
          AddressMode::Impl => {
            let res = self.shift_left(self[Reg::AC]);
            self[Reg::AC] = res;
          }
          _ => {
            let (val, address) = inst.resolve_operand_value_and_address(self);
            let res = self.shift_left(val);
            self.mem.write8(res, address);
          }
        };
      }
      Opcode::ROL => {
        match inst.mode() {
          AddressMode::Impl => {
            let res = self.rotate_left(self[Reg::AC]);
            self[Reg::AC] = res;
          }
          _ => {
            let (val, address) = inst.resolve_operand_value_and_address(self);
            let res = self.rotate_left(val);
            self.mem.write8(res, address);
          }
        };
      }
      Opcode::ROR => {
        match inst.mode() {
          AddressMode::Impl => {
            let res = self.rotate_right(self[Reg::AC]);
            self[Reg::AC] = res;
          }
          _ => {
            let (val, address) = inst.resolve_operand_value_and_address(self);
            let res = self.rotate_right(val);
            self.mem.write8(res, address);
          }
        };
      }
      Opcode::ADC => {
        let val = inst.resolve_operand_value(self);
        self[Reg::AC] = self.add_with_carry(self[Reg::AC], val);
      }
      Opcode::SBC | Opcode::USBC => {
        let val = inst.resolve_operand_value(self);
        self[Reg::AC] = self.sub_with_borrow(self[Reg::AC], val);
      }
      Opcode::ISC => {
        // cycles! should to as many cycles as ISC + INC + SBC LOOL
        // INC oper
        let (val, address) = inst.resolve_operand_value_and_address(self);
        let res = val.wrapping_add(1);
        self.mem.write8(res, address);

        // SBC oper
        self[Reg::AC] = self.sub_with_borrow(self[Reg::AC], res);
        // self.add_extra_cycles(10);
      }
      Opcode::EOR => {
        let val = inst.resolve_operand_value(self);
        let res = self[Reg::AC] ^ val;
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::ORA => {
        let val = inst.resolve_operand_value(self);
        let res = self[Reg::AC] | val;
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::AND => {
        let val = inst.resolve_operand_value( self);
        let res = self[Reg::AC] & val;
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::PHA => {
        self.push(self[Reg::AC]);
      }
      Opcode::PLA => {
        let res = self.pop();
        self[Reg::AC] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::PHX => self.push(self[Reg::X]),
      Opcode::PHY => self.push(self[Reg::Y]),
      Opcode::PLX => {
        let res = self.pop();
        self[Reg::X] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::PLY => {
        let res = self.pop();
        self[Reg::Y] = res;
        self.flags_set_neg_zero(res);
      }
      Opcode::PLP => {
        let res = self.pop();
        self.set_flags_ignore_5_4(res);
      }
      Opcode::PHP => {
        let mut res = self.flags_as_byte();
        res |= Flag::BUNUSEDMASK as u8; // break and 5 should always be set to 1 on stack
        self.push(res);
      }
      Opcode::BIT => {
        let val = inst.resolve_operand_value(self);
        let res = self[Reg::AC] & val;
        self[Flag::Z] = if res == 0 { 1 } else { 0 };
        self[Flag::N] = if (val & (1 << 7)) == 0 { 0 } else { 1 };
        self[Flag::V] = if (val & (1 << 6)) == 0 { 0 } else { 1 };
      }
      Opcode::ANC | Opcode::ANC2 => {
        let val = inst.resolve_operand_value(self);
        let res = self[Reg::AC] & val;
        self[Flag::C] = common::bits::is_signed(res) as u8;
        self.flags_set_neg_zero(res);
      }
      Opcode::ALR => {
        let val = inst.resolve_operand_value(self);
        let res = self[Reg::AC] & val;
        self[Flag::C] = res & 1u8;
        self.flags_set_neg_zero(res);
      }
    };

    // No jmp; advance.
    if pc_before_exec == self.pc() {
      self.inc_pc(inst.size());
    }

    inst.cycles() + self.extra_cycles
  }

  pub fn add_extra_cycles(&mut self, cycles: usize) {
    self.extra_cycles += cycles;
  }

  pub fn bus(&self) -> &Box<dyn Bus> {
    &self.mem
  }

  pub fn pc(&self) -> u16 {
    self.pc
  }

  pub fn set_pc(&mut self, pc: u16) {
    self.pc = pc
  }

  pub fn inc_pc(&mut self, inc: u8) {
    self.pc += inc as u16
  }

  pub fn reset(&mut self) {
    // TODO: Cycles
    self[Reg::AC] = 0;
    self[Reg::X] = 0;
    self[Reg::Y] = 0;
    self[Reg::SP] = 0xfd;

    self.flags = [0; 8];
    self[Flag::UNUSED] = 1;
    self[Flag::I] = 1;

    let start = self.read16(Self::RESET_VECTOR);
    println!("------------> RESET VECTOR: {:#06x}", start);
    self.set_pc(start);
  }

  pub fn interrupt_nmi(&mut self) {
    // TODO: Cycles
    self.push_word(self.pc());

    let mut stackflags = self.flags_as_byte();
    stackflags &= 0b11101111; // B should be off
    stackflags |= 0b00100000; // unused should be on
    self.push(stackflags);
    self[Flag::I] = 1;

    // Jump to NMI vector, TODO cycles
    let vector = self.read16(Self::NMI_VECTOR);
    // println!("NMI interrupt -> {:#06x}", vector);
    self.add_extra_cycles(2);
    self.set_pc(vector);
  }

  fn push_word(&mut self, val: u16) {
    let ret_high = (val >> 8) as u8;
    let ret_low = (val & 0x00ff) as u8;
    self.push(ret_high);
    self.push(ret_low);
  }

  fn pop_word(&mut self) -> u16 {
    let ret_low = self.pop();
    let ret_high = self.pop();
    (ret_high as u16) << 8 | ret_low as u16
  }

  fn inc_reg(&mut self, reg: Reg) {
    let res = self[reg].wrapping_add(1);
    self[reg] = res;
    self.flags_set_neg_zero(res);
  }

  fn dec_reg(&mut self, reg: Reg) {
    let res = self[reg].wrapping_sub(1);
    self[reg] = res;
    self.flags_set_neg_zero(res);
  }

  fn mv_with_neg_zero(&mut self, src: Reg, dst: Reg) {
    let val = self[src];
    self[dst] = val;
    self.flags_set_neg_zero(val);
  }

  fn flags_set_neg_zero(&mut self, res: u8) {
    self[Flag::Z] = (res == 0) as u8;
    self[Flag::N] = (res & (1 << 7) != 0) as u8;
  }

  fn cmp(&mut self, reg: Reg, val: u8) {
    let (res, overflow) = self[reg].overflowing_sub(val);
    self[Flag::C] = !overflow as u8;
    self.flags_set_neg_zero(res);
  }

  fn shift_right(&mut self, val: u8) -> u8 {
    // All shift and rotate instructions preserve the bit shifted out in the carry flag.
    self[Flag::C] = (val & 1 != 0) as u8;
    let (res, _) = val.overflowing_shr(1);
    self.flags_set_neg_zero(res);
    res
  }

  fn shift_left(&mut self, val: u8) -> u8 {
    // All shift and rotate instructions preserve the bit shifted out in the carry flag.
    self[Flag::C] = (val & (1 << 7) != 0) as u8;
    let (res, _) = val.overflowing_shl(1);
    self.flags_set_neg_zero(res);
    res
  }

  fn rotate_left(&mut self, val: u8) -> u8 {
    // All shift and rotate instructions preserve the bit shifted out in the carry flag.
    // rotate left (shifts in carry bit on the right)
    let carry_bit_before_shift = self[Flag::C];
    self[Flag::C] = (val & (1 << 7) != 0) as u8;
    let (mut res, _) = val.overflowing_shl(1);
    res |= carry_bit_before_shift;
    self.flags_set_neg_zero(res);
    res
  }

  fn rotate_right(&mut self, val: u8) -> u8 {
    // All shift and rotate instructions preserve the bit shifted out in the carry flag.
    // rotate right (shifts in CARRY bit on the left) (masswerk says zero bit but I think it's an error)
    let carry_bit_before_shift = self[Flag::C];
    self[Flag::C] = (val & 1 != 0) as u8;
    let (mut res, _) = val.overflowing_shr(1);
    res |= carry_bit_before_shift << 7;
    self.flags_set_neg_zero(res);
    res
  }

  fn calc_offset_pc(&self, offset: u8) -> u16 {
    let signed = offset as i8;
    if signed >= 0 {
      let effective_offset = offset as u16;
      self.pc.wrapping_add(effective_offset)
    }
    else {
      let signed_offset = ((offset as u16) | 0xff00) as i16;
      let effective_offset = (-signed_offset) as u16;
      self.pc.wrapping_sub(effective_offset)
    }
  }

  fn add_with_carry(&mut self, lhs: u8, rhs: u8) -> u8 {
    if self[Flag::D] == 1 {
      // panic!("implement decimal mode");
    }

    let (step1, carry1) = lhs.overflowing_add(self[Flag::C]);
    let (res, carry2) = step1.overflowing_add(rhs);
    self[Flag::V] = common::bits::is_overflow(res, lhs, rhs) as u8;
    self[Flag::C] = (carry1 || carry2) as u8;
    self.flags_set_neg_zero(res);
    res
  }

  fn sub_with_borrow(&mut self, lhs: u8, rhs: u8) -> u8 {
    // Do not understand how this works, but it works.
    self.add_with_carry(lhs, rhs ^ 0xff)
  }

  fn push(&mut self, val: u8) {  
    let sp = self[Reg::SP] as usize;
    let address = (Cpu::STACK_TOP + sp) as u16;
    self.mem.write8(val, address);
    self[Reg::SP] = self[Reg::SP].wrapping_sub(1);
  }

  fn pop(&mut self) -> u8 {
    self[Reg::SP] = self[Reg::SP].wrapping_add(1);
    let sp = self[Reg::SP] as usize;
    let address = (Cpu::STACK_TOP + sp) as u16;
    self.mem.read8(address)
  }

  fn set_flags_ignore_5_4(&mut self, val: u8) {
    for bit in 0..=7usize {
      match bit {
        5 | 4 => (), // ignore break and 5
        _ => self.flags[bit] = if val & (1 << bit) == 0 { 0 } else { 1 } 
      }
    }
  }

  pub fn flags_as_byte(&self) -> u8 {
    let mut res = 0x00;
    for bit in 0..=7usize {
      res |= self.flags[bit] << bit
    }
    res
  }

  fn branch_if(&mut self, offset: u8, cond: impl Fn(&Cpu) -> bool) {
    if offset == 0 {
      // (An offset of #0 corresponds to the immedately following address — or a rather odd and expensive NOP.)
      return;
    }
    if cond(self) {
      self.inc_pc(2);
      let branch_target = self.calc_offset_pc(offset);
    
      // if hi byte changes, we crossed a page boundary and should add extra cycles
      // "add 1 to cycles if branch occurs on same page, add 2 to cycles if branch occurs to different page"
      let crossed_page = self.pc() & 0xff00 != branch_target & 0xff00;
      if crossed_page {
        self.add_extra_cycles(2);
      } else {
        self.add_extra_cycles(1);
      }

      self.set_pc(branch_target);
    }
  }

  fn read16(&self, address: u16) -> u16 {
    let val_low = self.mem.read8(address) as u16;
    let val_high = self.mem.read8(address + 1) as u16;
    (val_high << 8) | val_low
  }
}

impl Index<Reg> for Cpu {
  type Output = u8;

  fn index(&self, index: Reg) -> &u8 {
    &self.regs[index as usize]
  }
}

impl IndexMut<Reg> for Cpu {
  fn index_mut(&mut self, index: Reg) -> &mut u8 {
    &mut self.regs[index as usize]
  }
}

impl Index<Flag> for Cpu {
  type Output = u8;

  fn index(&self, index: Flag) -> &u8 {
    &self.flags[index as usize]
  }
}

impl IndexMut<Flag> for Cpu {
  fn index_mut(&mut self, index: Flag) -> &mut u8 {
    &mut self.flags[index as usize]
  }
}

impl std::fmt::Debug for Cpu {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    fn hexdec(val: u8) -> String {
      format!("{:#04x} ({})", val, val)
    }

    write!(f, "--------\n")?;
    write!(f, "pc:\t{:#06x}\nsp:\t{}\nacc:\t{}\nx:\t{}\ny:\t{}\n", self.pc, hexdec(self[Reg::SP]), hexdec(self[Reg::AC]), hexdec(self[Reg::X]), hexdec(self[Reg::Y]))?;
    write!(f, "NEG={}, OVF={}, DEC={}, INT={}, ZER={}, CAR={} ({:#010b}) ({:#04x})", self[Flag::N], self[Flag::V], self[Flag::D], self[Flag::I], self[Flag::Z], self[Flag::C], self.flags_as_byte(), self.flags_as_byte())
  }
}

impl std::fmt::Display for Cpu {
  // nestest format
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}", self[Reg::AC], self[Reg::X], self[Reg::Y], self.flags_as_byte(), self[Reg::SP])
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct TestBus([u8; 0xffff + 1]);

  impl Bus for TestBus {
    fn read8(&self, address: u16) -> u8 {
      self.0[address as usize]
    }

    fn write8(&mut self, val: u8, address: u16) {
      self.0[address as usize] = val;
    }
  }

  fn sut() -> Cpu {
    Cpu::new(Box::new(TestBus([0; 0xffff + 1])))
  }

  #[test]
  fn stack_pop_push_should_wrap() {
    let mut cpu = sut();
    cpu[Reg::SP] = 0;

    cpu.push(42);
    assert_eq!(cpu[Reg::SP], 0xff);

    let val = cpu.pop();
    assert_eq!(val, 42);
    assert_eq!(cpu[Reg::SP], 0);
  }

  #[test]
  fn inc_dec_regs_should_wrap() {
    let mut cpu = sut();
    cpu[Reg::X] = 0xff;
    cpu[Reg::Y] = 0xff;

    cpu.inc_reg(Reg::X);
    cpu.inc_reg(Reg::Y);

    assert_eq!(cpu[Reg::X], 0);
    assert_eq!(cpu[Reg::Y], 0);

    cpu.dec_reg(Reg::X);
    cpu.dec_reg(Reg::Y);

    assert_eq!(cpu[Reg::X], 0xff);
    assert_eq!(cpu[Reg::Y], 0xff);
  }

  #[test]
  fn set_flags() {
    let mut cpu = sut();
    cpu.set_flags_ignore_5_4(0b10100001);
    assert_eq!(cpu[Flag::N], 1);
    assert_eq!(cpu[Flag::V], 0);
    assert_eq!(cpu[Flag::D], 0);
    assert_eq!(cpu[Flag::I], 0);
    assert_eq!(cpu[Flag::Z], 0);
    assert_eq!(cpu[Flag::C], 1);

    let mut cpu = sut();
    cpu.set_flags_ignore_5_4(0b11001010);
    assert_eq!(cpu[Flag::N], 1);
    assert_eq!(cpu[Flag::V], 1);
    assert_eq!(cpu[Flag::D], 1);
    assert_eq!(cpu[Flag::I], 0);
    assert_eq!(cpu[Flag::Z], 1);
    assert_eq!(cpu[Flag::C], 0);
  }

  #[test]
  fn get_flags() {
    let mut cpu = sut();

    assert_eq!(cpu.flags_as_byte(), 0b00000000);

    cpu[Flag::N] = 1;
    cpu[Flag::Z] = 1;

    assert_eq!(cpu.flags_as_byte(), 0b10000010);
  }

  #[test]
  fn add_with_carry() {
    let mut cpu = sut();

    // -5 + -124
    cpu.add_with_carry(0b11111011, 0b10000100);
    assert_eq!(cpu[Flag::V], 1);

    let mut cpu = sut();
    cpu.add_with_carry(255, 1);
    assert_eq!(cpu[Flag::V], 0);
    assert_eq!(cpu[Flag::C], 1);
    assert_eq!(cpu[Flag::Z], 1);
    assert_eq!(cpu[Flag::N], 0);

    let mut cpu = sut();
    cpu.add_with_carry(254, 1);
    assert_eq!(cpu[Flag::V], 0);
    assert_eq!(cpu[Flag::C], 0);
    assert_eq!(cpu[Flag::Z], 0);
    assert_eq!(cpu[Flag::N], 1);
  }

  #[test]
  fn cmp() {
    let mut cpu = sut();
    
    cpu[Reg::Y] = 10;
    cpu.cmp(Reg::Y, 11);
    assert_eq!(cpu[Flag::Z], 0);
    assert_eq!(cpu[Flag::C], 0);
    assert_eq!(cpu[Flag::N], 1);

    cpu[Reg::Y] = 10;
    cpu.cmp(Reg::Y, 10);
    assert_eq!(cpu[Flag::Z], 1);
    assert_eq!(cpu[Flag::C], 1);
    assert_eq!(cpu[Flag::N], 0);

    cpu[Reg::Y] = 11;
    cpu.cmp(Reg::Y, 10);
    assert_eq!(cpu[Flag::Z], 0);
    assert_eq!(cpu[Flag::C], 1);
    assert_eq!(cpu[Flag::N], 0);
  }

  #[test]
  fn shift_right() {
    let mut cpu = sut();

    cpu.shift_right(0b001);
    cpu[Flag::Z] = 1;
    cpu[Flag::C] = 1;

    cpu.shift_right(0b100);
    cpu[Flag::Z] = 0;
    cpu[Flag::C] = 0;
  }

  #[test]
  fn offset_pc() {
    let mut cpu = sut();

    cpu.set_pc(0x10);
    assert_eq!(cpu.calc_offset_pc(1), 0x11); // + 1

    cpu.set_pc(0x20); // 32
    assert_eq!(cpu.calc_offset_pc(0xf4), 0x14); // // -12 == 20

    cpu.set_pc(0x0000);
    assert_eq!(cpu.calc_offset_pc(255), 0xffff); // -1

    cpu.set_pc(0xFFFF);
    assert_eq!(cpu.calc_offset_pc(1), 0x0000); // +1

    cpu.set_pc(0x0000);
    assert_eq!(cpu.calc_offset_pc(0xc1), 0xffc1); // -63
  }
}