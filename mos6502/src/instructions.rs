use crate::{address_mode::AddressMode, cpu::Cpu};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum Opcode {
  ADC, // Add Memory to Accumulator with Carry
  SBC, // Subtract Memory from Accumulator with Borrow
  CLD, // Clear decimal mode
  CLC, // Clear carry
  CLI, // Clear interrupt
  CLV, // Clear overflow
  EOR, // Exclusive-OR Memory with Accumulator
  AND, // AND Memory with Accumulator
  ORA, // OR Memory with Accumulator
  LDX,
  LDA,
  LDY,
  TAX, // Transfer Accumulator to Index X
  TAY, // Transfer Accumulator to Index Y
  TSX, // Transfer Stack Pointer to Index X
  TXA, // Transfer Index X to Accumulato
  TXS,
  TYA, // Transfer Index Y to Accumulator
  STA, // Store Accumulator in Memory
  STX, // Store Index X in Memory
  STY, // Store Index Y in Memory
  JMP,
  JSR, // Jump to New Location Saving Return Address
  RTS, // Return from Subroutine
  BNE, // Branch on Result not Zero
  BEQ, // Branch on Result Zero
  BPL, // Branch on Result Plus
  DEX, // Decrement Index X by One
  DEY, // Decrement Index Y by One
  DEC, // Decrement Memory by One
  INC, // Increment Memory by One
  INX,
  INY,
  BIT, // Test Bits in Memory with Accumulator
  BCC, // Branch on Carry Clear
  BCS, // Branch on Carry Set
  BMI, // Branch on Result Minus
  BVC, // Branch on Overflow Clear
  BVS, // Branch on Overflow Set
  SEC, // Set Carry Flag
  SED, // Set Decimal Flag
  SEI, // Set Interrupt Disable Status
  NOP,
  CPY, // Compare Memory and Index Y
  CPX, // Compare Memory and Index X
  LSR, // Shift One Bit Right (Memory or Accumulator)
  ASL, // Shift Left One Bit (Memory or Accumulator)
  ROL, // Rotate One Bit Left (Memory or Accumulator)
  ROR, // Rotate One Bit Right (Memory or Accumulator)
  CMP, // Compare Memory with Accumulator
  PHA, // Push Accumulator on Stack
  PLA, // Pull Accumulator from Stack,
  PLP, // Pull Processor Status from Stack
  PHP, // Push Processor Status on Stack,
  JAM, // Halt, kill. Illegal, only used for own tests so far.
  BRK, // Force Break, Software interrupt
  RTI, // Return from Interrupt

  // Illegal opcodes
  LAX, // LDA oper + LDX oper
  SAX, // A AND X -> M
  USBC, // effectively same as normal SBC immediate, instr. E9.
  DCP, // DEC oper + CMP oper
  ISC, // INC oper + SBC oper
  SLO, // ASL oper + ORA oper
  RLA, // ROL oper + AND oper
  SRE, // LSR oper + EOR oper
  RRA, // ROR oper + ADC oper
  PHX, // push x
  PHY, // push y
  PLX, // pull x
  PLY, // pull y
  ANC, // AND oper + set C as ASL
  ANC2, // effectively the same as instr. 0B (ANC)
  ALR, // AND oper + LSR
}

#[derive(Clone, Debug)]
pub struct Instruction {
  opcode: Opcode,
  mode: AddressMode,
  cycles: usize,
  operands: (Option<u8>, Option<u8>),
  size: u8,
}

impl Instruction {
  fn imp(opcode: Opcode, cycles: usize) -> Self {
    Self { opcode, cycles, mode: AddressMode::Impl, size: 1, operands: (None, None) }
  }

  fn two(opcode: Opcode, cycles: usize, mode: AddressMode, operand: u8) -> Self {
    Self { opcode, cycles, mode, size: 2, operands: (Some(operand), None) }
  }

  fn thr(opcode: Opcode, cycles: usize, mode: AddressMode, operand0: u8, operand1: u8) -> Self {
    Self { opcode, cycles, mode, size: 3, operands: (Some(operand0), Some(operand1)) }
  }

  pub fn size(&self) -> u8 {
    self.size
  }

  pub fn opcode(&self) -> Opcode {
    self.opcode
  }

  pub fn mode(&self) -> AddressMode {
    self.mode
  }

  pub fn cycles(&self) -> usize {
    self.cycles
  }

  pub fn resolve_operand_value_and_address(&self, cpu: &mut Cpu) -> (u8, u16) {
    let address = self.mode.resolve(cpu, self.operands, self.num_extra_cycles());
    let value = cpu.bus().read8(address);
    (value, address)
  }

  pub fn resolve_operand_value(&self, cpu: &mut Cpu) -> u8 {
    match self.mode {
      AddressMode::Imm => self.operands.0.unwrap(),
      _ => {
        let address = self.mode.resolve(cpu, self.operands, self.num_extra_cycles());
        cpu.bus().read8(address)
      }
    }
  }

  pub fn resolve_operand_address(&self, cpu: &mut Cpu) -> u16 {
    self.mode.resolve(cpu, self.operands, self.num_extra_cycles())
  }

  pub fn operand0(&self) -> u8 {
    self.operands.0.unwrap()
  }

  pub fn operands(&self) -> (Option<u8>, Option<u8>) {
    self.operands
  }

  fn num_extra_cycles(&self) -> usize {
    match self.opcode {
      // these instructions don't add a cycle when they cross page bounds
      Opcode::DCP => 0,
      Opcode::STA => 0,
      Opcode::SLO => 0,
      Opcode::RLA => 0,
      Opcode::SRE => 0,
      Opcode::RRA => 0,
      // isc in indy apparently adds 4 cycles.. bc many instructions in one i guess
      Opcode::ISC => {
        match self.mode {
          AddressMode::IndY => 4,
          _ => 0,
        }
      }
      _ => 1,
    }
  }

  pub fn disassemble(opbyte: u8, operand1: u8, operand2: u8) -> Instruction {
    match opbyte {
      0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => Instruction::imp(Opcode::JAM, 0),

      0x00 => Instruction::imp(Opcode::BRK, 7),
      0x40 => Instruction::imp(Opcode::RTI, 6),

      0x38 => Instruction::imp(Opcode::SEC, 2),
      0xf8 => Instruction::imp(Opcode::SED, 2),
      0x78 => Instruction::imp(Opcode::SEI, 2),
  
      0x18 => Instruction::imp(Opcode::CLC, 2),
      0xd8 => Instruction::imp(Opcode::CLD, 2),
      0x58 => Instruction::imp(Opcode::CLI, 2),
      0xb8 => Instruction::imp(Opcode::CLV, 2),
  
      0xaa => Instruction::imp(Opcode::TAX, 2),
      0xa8 => Instruction::imp(Opcode::TAY, 2),
      0xba => Instruction::imp(Opcode::TSX, 2),
      0x8a => Instruction::imp(Opcode::TXA, 2),
      0x9a => Instruction::imp(Opcode::TXS, 2),
      0x98 => Instruction::imp(Opcode::TYA, 2),

      0x24 => Instruction::two(Opcode::BIT, 3, AddressMode::Zero, operand1),
      0x2c => Instruction::thr(Opcode::BIT, 4, AddressMode::Abs, operand1, operand2),
  
      0x69 => Instruction::two(Opcode::ADC, 2, AddressMode::Imm, operand1),
      0x65 => Instruction::two(Opcode::ADC, 3, AddressMode::Zero, operand1),
      0x75 => Instruction::two(Opcode::ADC, 4, AddressMode::ZeroX, operand1),
      0x6d => Instruction::thr(Opcode::ADC, 4, AddressMode::Abs, operand1, operand2),
      0x7d => Instruction::thr(Opcode::ADC, 4, AddressMode::AbsX, operand1, operand2),
      0x79 => Instruction::thr(Opcode::ADC, 4, AddressMode::AbsY, operand1, operand2),
      0x61 => Instruction::two(Opcode::ADC, 6, AddressMode::IndX, operand1),
      0x71 => Instruction::two(Opcode::ADC, 5, AddressMode::IndY, operand1),

      0xe9 => Instruction::two(Opcode::SBC, 2, AddressMode::Imm, operand1),
      0xe5 => Instruction::two(Opcode::SBC, 3, AddressMode::Zero, operand1),
      0xf5 => Instruction::two(Opcode::SBC, 4, AddressMode::ZeroX, operand1),
      0xed => Instruction::thr(Opcode::SBC, 4, AddressMode::Abs, operand1, operand2),
      0xfd => Instruction::thr(Opcode::SBC, 4, AddressMode::AbsX, operand1, operand2),
      0xf9 => Instruction::thr(Opcode::SBC, 4, AddressMode::AbsY, operand1, operand2),
      0xe1 => Instruction::two(Opcode::SBC, 6, AddressMode::IndX, operand1),
      0xf1 => Instruction::two(Opcode::SBC, 5, AddressMode::IndY, operand1),
  
      0xa2 => Instruction::two(Opcode::LDX, 2, AddressMode::Imm, operand1),
      0xa6 => Instruction::two(Opcode::LDX, 3, AddressMode::Zero, operand1),
      0xb6 => Instruction::two(Opcode::LDX, 4, AddressMode::ZeroY, operand1),
      0xae => Instruction::thr(Opcode::LDX, 4, AddressMode::Abs, operand1, operand2),
      0xbe => Instruction::thr(Opcode::LDX, 4, AddressMode::AbsY, operand1, operand2),
      0x49 => Instruction::two(Opcode::EOR, 2, AddressMode::Imm, operand1),
      0x45 => Instruction::two(Opcode::EOR, 3, AddressMode::Zero, operand1),
      0x55 => Instruction::two(Opcode::EOR, 4, AddressMode::ZeroX, operand1),
      0x4d => Instruction::thr(Opcode::EOR, 4, AddressMode::Abs, operand1, operand2),
      0x5d => Instruction::thr(Opcode::EOR, 4, AddressMode::AbsX, operand1, operand2),
      0x59 => Instruction::thr(Opcode::EOR, 4, AddressMode::AbsY, operand1, operand2),
      0x41 => Instruction::two(Opcode::EOR, 6, AddressMode::IndX, operand1),
      0x51 => Instruction::two(Opcode::EOR, 5, AddressMode::IndY, operand1),

      0x09 => Instruction::two(Opcode::ORA, 2, AddressMode::Imm, operand1),
      0x05 => Instruction::two(Opcode::ORA, 3, AddressMode::Zero, operand1),
      0x15 => Instruction::two(Opcode::ORA, 4, AddressMode::ZeroX, operand1),
      0x0d => Instruction::thr(Opcode::ORA, 4, AddressMode::Abs, operand1, operand2),
      0x1d => Instruction::thr(Opcode::ORA, 4, AddressMode::AbsX, operand1, operand2),
      0x19 => Instruction::thr(Opcode::ORA, 4, AddressMode::AbsY, operand1, operand2),
      0x01 => Instruction::two(Opcode::ORA, 6, AddressMode::IndX, operand1),
      0x11 => Instruction::two(Opcode::ORA, 5, AddressMode::IndY, operand1),

      0x29 => Instruction::two(Opcode::AND, 2, AddressMode::Imm, operand1),
      0x25 => Instruction::two(Opcode::AND, 3, AddressMode::Zero, operand1),
      0x35 => Instruction::two(Opcode::AND, 4, AddressMode::ZeroX, operand1),
      0x2d => Instruction::thr(Opcode::AND, 4, AddressMode::Abs, operand1, operand2),
      0x3d => Instruction::thr(Opcode::AND, 4, AddressMode::AbsX, operand1, operand2),
      0x39 => Instruction::thr(Opcode::AND, 4, AddressMode::AbsY, operand1, operand2),
      0x21 => Instruction::two(Opcode::AND, 6, AddressMode::IndX, operand1),
      0x31 => Instruction::two(Opcode::AND, 5, AddressMode::IndY, operand1),
  
      0xa0 => Instruction::two(Opcode::LDY, 2, AddressMode::Imm, operand1),
      0xa4 => Instruction::two(Opcode::LDY, 3, AddressMode::Zero, operand1),
      0xb4 => Instruction::two(Opcode::LDY, 4, AddressMode::ZeroX, operand1),
      0xac => Instruction::thr(Opcode::LDY, 4, AddressMode::Abs, operand1, operand2),
      0xbc => Instruction::thr(Opcode::LDY, 4, AddressMode::AbsX, operand1, operand2),
  
      0xa9 => Instruction::two(Opcode::LDA, 2, AddressMode::Imm, operand1),
      0xa5 => Instruction::two(Opcode::LDA, 3, AddressMode::Zero, operand1),
      0xb5 => Instruction::two(Opcode::LDA, 4, AddressMode::ZeroX, operand1),
      0xad => Instruction::thr(Opcode::LDA, 4, AddressMode::Abs, operand1, operand2),
      0xbd => Instruction::thr(Opcode::LDA, 4, AddressMode::AbsX, operand1, operand2),
      0xb9 => Instruction::thr(Opcode::LDA, 4, AddressMode::AbsY, operand1, operand2),
      0xa1 => Instruction::two(Opcode::LDA, 6, AddressMode::IndX, operand1),
      0xb1 => Instruction::two(Opcode::LDA, 5, AddressMode::IndY, operand1),
  
      0x85 => Instruction::two(Opcode::STA, 3, AddressMode::Zero, operand1),
      0x95 => Instruction::two(Opcode::STA, 4, AddressMode::ZeroX, operand1),
      0x8d => Instruction::thr(Opcode::STA, 4, AddressMode::Abs, operand1, operand2),
      0x9d => Instruction::thr(Opcode::STA, 5, AddressMode::AbsX, operand1, operand2),
      0x99 => Instruction::thr(Opcode::STA, 5, AddressMode::AbsY, operand1, operand2),
      0x81 => Instruction::two(Opcode::STA, 6, AddressMode::IndX, operand1),
      0x91 => Instruction::two(Opcode::STA, 6, AddressMode::IndY, operand1),

      0x86 => Instruction::two(Opcode::STX, 3, AddressMode::Zero, operand1),
      0x96 => Instruction::two(Opcode::STX, 4, AddressMode::ZeroY, operand1),
      0x8e => Instruction::thr(Opcode::STX, 4, AddressMode::Abs, operand1, operand2),

      0x84 => Instruction::two(Opcode::STY, 3, AddressMode::Zero, operand1),
      0x94 => Instruction::two(Opcode::STY, 4, AddressMode::ZeroX, operand1),
      0x8c => Instruction::thr(Opcode::STY, 4, AddressMode::Abs, operand1, operand2),
  
      0x4c => Instruction::thr(Opcode::JMP, 3, AddressMode::Abs, operand1, operand2),
      0x6c => Instruction::thr(Opcode::JMP, 5, AddressMode::Ind, operand1, operand2),
      0x20 => Instruction::thr(Opcode::JSR, 6, AddressMode::Abs, operand1, operand2),
      0x60 => Instruction::imp(Opcode::RTS, 6),
  
      0xd0 => Instruction::two(Opcode::BNE, 2, AddressMode::Rel, operand1),
      0xf0 => Instruction::two(Opcode::BEQ, 2, AddressMode::Rel, operand1),
      0x10 => Instruction::two(Opcode::BPL, 2, AddressMode::Rel, operand1),
      0x90 => Instruction::two(Opcode::BCC, 2, AddressMode::Rel, operand1),
      0xb0 => Instruction::two(Opcode::BCS, 2, AddressMode::Rel, operand1),
      0x30 => Instruction::two(Opcode::BMI, 2, AddressMode::Rel, operand1),
      0x50 => Instruction::two(Opcode::BVC, 2, AddressMode::Rel, operand1),
      0x70 => Instruction::two(Opcode::BVS, 2, AddressMode::Rel, operand1),
  
      0xca => Instruction::imp(Opcode::DEX, 2),
      0x88 => Instruction::imp(Opcode::DEY, 2),
      0xe8 => Instruction::imp(Opcode::INX, 2),
      0xc8 => Instruction::imp(Opcode::INY, 2),

      0xc6 => Instruction::two(Opcode::DEC, 5, AddressMode::Zero, operand1),
      0xd6 => Instruction::two(Opcode::DEC, 6, AddressMode::ZeroX, operand1),
      0xce => Instruction::thr(Opcode::DEC, 6, AddressMode::Abs, operand1, operand2),
      0xde => Instruction::thr(Opcode::DEC, 7, AddressMode::AbsX, operand1, operand2),

      0xe6 => Instruction::two(Opcode::INC, 5, AddressMode::Zero, operand1),
      0xf6 => Instruction::two(Opcode::INC, 6, AddressMode::ZeroX, operand1),
      0xee => Instruction::thr(Opcode::INC, 6, AddressMode::Abs, operand1, operand2),
      0xfe => Instruction::thr(Opcode::INC, 7, AddressMode::AbsX, operand1, operand2),
  
      // 1 byte NOPs
      0xea | 0x1a | 0x3a => Instruction::imp(Opcode::NOP, 2),

      // 2 byte NOPs 2 cycles
      0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 => Instruction::two(Opcode::NOP, 2, AddressMode::Nop, operand1),
      // 2 byte NOPs 3 cycles
      0x04 | 0x44 | 0x64 => Instruction::two(Opcode::NOP, 3, AddressMode::Nop, operand1),
      // 2 byte NOPs 4 cycles
      0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 => Instruction::two(Opcode::NOP, 4, AddressMode::Nop, operand1),

      // 3 byte nop
      0x0c => Instruction::thr(Opcode::NOP, 4, AddressMode::Nop, operand1, operand2),
      // 3 byte NOPs, absX for cycle
      0x1c | 0x3c | 0x5c | 0x7c | 0xdc | 0xfc => Instruction::thr(Opcode::NOP, 4, AddressMode::AbsX, operand1, operand2),
  
      0xc0 => Instruction::two(Opcode::CPY, 2, AddressMode::Imm, operand1),
      0xc4 => Instruction::two(Opcode::CPY, 3, AddressMode::Zero, operand1),
      0xcc => Instruction::thr(Opcode::CPY, 4, AddressMode::Abs, operand1, operand2),
      0xe0 => Instruction::two(Opcode::CPX, 2, AddressMode::Imm, operand1),
      0xe4 => Instruction::two(Opcode::CPX, 3, AddressMode::Zero, operand1),
      0xec => Instruction::thr(Opcode::CPX, 4, AddressMode::Abs, operand1, operand2),
      
      0xc9 => Instruction::two(Opcode::CMP, 2, AddressMode::Imm, operand1),
      0xc5 => Instruction::two(Opcode::CMP, 3, AddressMode::Zero, operand1),
      0xd5 => Instruction::two(Opcode::CMP, 4, AddressMode::ZeroX, operand1),
      0xcd => Instruction::thr(Opcode::CMP, 4, AddressMode::Abs, operand1, operand2),
      0xdd => Instruction::thr(Opcode::CMP, 4, AddressMode::AbsX, operand1, operand2),
      0xd9 => Instruction::thr(Opcode::CMP, 4, AddressMode::AbsY, operand1, operand2), 
      0xc1 => Instruction::two(Opcode::CMP, 6, AddressMode::IndX, operand1),
      0xd1 => Instruction::two(Opcode::CMP, 5, AddressMode::IndY, operand1),
  
      0x4a => Instruction::imp(Opcode::LSR, 2),
      0x46 => Instruction::two(Opcode::LSR, 5, AddressMode::Zero, operand1),
      0x56 => Instruction::two(Opcode::LSR, 6, AddressMode::ZeroX, operand1),
      0x4e => Instruction::thr(Opcode::LSR, 6, AddressMode::Abs, operand1, operand2),
      0x5e => Instruction::thr(Opcode::LSR, 7, AddressMode::AbsX, operand1, operand2),

      0x0a => Instruction::imp(Opcode::ASL, 2),
      0x06 => Instruction::two(Opcode::ASL, 5, AddressMode::Zero, operand1),
      0x16 => Instruction::two(Opcode::ASL, 6, AddressMode::ZeroX, operand1),
      0x0e => Instruction::thr(Opcode::ASL, 6, AddressMode::Abs, operand1, operand2),
      0x1e => Instruction::thr(Opcode::ASL, 7, AddressMode::AbsX, operand1, operand2),

      0x2a => Instruction::imp(Opcode::ROL, 2),
      0x26 => Instruction::two(Opcode::ROL, 5, AddressMode::Zero, operand1),
      0x36 => Instruction::two(Opcode::ROL, 6, AddressMode::ZeroX, operand1),
      0x2e => Instruction::thr(Opcode::ROL, 6, AddressMode::Abs, operand1, operand2),
      0x3e => Instruction::thr(Opcode::ROL, 7, AddressMode::AbsX, operand1, operand2),

      0x6a => Instruction::imp(Opcode::ROR, 2),
      0x66 => Instruction::two(Opcode::ROR, 5, AddressMode::Zero, operand1),
      0x76 => Instruction::two(Opcode::ROR, 6, AddressMode::ZeroX, operand1),
      0x6e => Instruction::thr(Opcode::ROR, 6, AddressMode::Abs, operand1, operand2),
      0x7e => Instruction::thr(Opcode::ROR, 7, AddressMode::AbsX, operand1, operand2),
  
      0x48 => Instruction::imp(Opcode::PHA, 3),
      0x68 => Instruction::imp(Opcode::PLA, 4),
      0x08 => Instruction::imp(Opcode::PHP, 3),
      0x28 => Instruction::imp(Opcode::PLP, 4),

      0xa7 => Instruction::two(Opcode::LAX, 3, AddressMode::Zero, operand1),
      0xb7 => Instruction::two(Opcode::LAX, 4, AddressMode::ZeroY, operand1),
      0xaf => Instruction::thr(Opcode::LAX, 4, AddressMode::Abs, operand1, operand2),
      0xbf => Instruction::thr(Opcode::LAX, 4, AddressMode::AbsY, operand1, operand2),
      0xa3 => Instruction::two(Opcode::LAX, 6, AddressMode::IndX, operand1),
      0xb3 => Instruction::two(Opcode::LAX, 5, AddressMode::IndY, operand1),

      0x87 => Instruction::two(Opcode::SAX, 3, AddressMode::Zero, operand1),
      0x97 => Instruction::two(Opcode::SAX, 4, AddressMode::ZeroY, operand1),
      0x8f => Instruction::thr(Opcode::SAX, 4, AddressMode::Abs, operand1, operand2),
      0x83 => Instruction::two(Opcode::SAX, 6, AddressMode::IndX, operand1),

      0xeb => Instruction::two(Opcode::USBC, 2, AddressMode::Imm, operand1),

      0xc7 => Instruction::two(Opcode::DCP, 5, AddressMode::Zero, operand1),
      0xd7 => Instruction::two(Opcode::DCP, 6, AddressMode::ZeroX, operand1),
      0xcf => Instruction::thr(Opcode::DCP, 6, AddressMode::Abs, operand1, operand2),
      0xdf => Instruction::thr(Opcode::DCP, 7, AddressMode::AbsX, operand1, operand2),
      0xdb => Instruction::thr(Opcode::DCP, 7, AddressMode::AbsY, operand1, operand2),
      0xc3 => Instruction::two(Opcode::DCP, 8, AddressMode::IndX, operand1),
      0xd3 => Instruction::two(Opcode::DCP, 8, AddressMode::IndY, operand1),

      0xe7 => Instruction::two(Opcode::ISC, 5, AddressMode::Zero, operand1),
      0xf7 => Instruction::two(Opcode::ISC, 6, AddressMode::ZeroX, operand1),
      0xef => Instruction::thr(Opcode::ISC, 6, AddressMode::Abs, operand1, operand2),
      0xff => Instruction::thr(Opcode::ISC, 7, AddressMode::AbsX, operand1, operand2),
      0xfb => Instruction::thr(Opcode::ISC, 7, AddressMode::AbsY, operand1, operand2),
      0xe3 => Instruction::two(Opcode::ISC, 8, AddressMode::IndX, operand1),
      0xf3 => Instruction::two(Opcode::ISC, 4, AddressMode::IndY, operand1),

      0x07 => Instruction::two(Opcode::SLO, 5, AddressMode::Zero, operand1),
      0x17 => Instruction::two(Opcode::SLO, 6, AddressMode::ZeroX, operand1),
      0x0f => Instruction::thr(Opcode::SLO, 6, AddressMode::Abs, operand1, operand2),
      0x1f => Instruction::thr(Opcode::SLO, 7, AddressMode::AbsX, operand1, operand2),
      0x1b => Instruction::thr(Opcode::SLO, 7, AddressMode::AbsY, operand1, operand2),
      0x03 => Instruction::two(Opcode::SLO, 8, AddressMode::IndX, operand1),
      0x13 => Instruction::two(Opcode::SLO, 8, AddressMode::IndY, operand1),

      0x27 => Instruction::two(Opcode::RLA, 5, AddressMode::Zero, operand1),
      0x37 => Instruction::two(Opcode::RLA, 6, AddressMode::ZeroX, operand1),
      0x2f => Instruction::thr(Opcode::RLA, 6, AddressMode::Abs, operand1, operand2),
      0x3f => Instruction::thr(Opcode::RLA, 7, AddressMode::AbsX, operand1, operand2),
      0x3b => Instruction::thr(Opcode::RLA, 7, AddressMode::AbsY, operand1, operand2),
      0x23 => Instruction::two(Opcode::RLA, 8, AddressMode::IndX, operand1),
      0x33 => Instruction::two(Opcode::RLA, 8, AddressMode::IndY, operand1),

      0x47 => Instruction::two(Opcode::SRE, 5, AddressMode::Zero, operand1),
      0x57 => Instruction::two(Opcode::SRE, 6, AddressMode::ZeroX, operand1),
      0x4f => Instruction::thr(Opcode::SRE, 6, AddressMode::Abs, operand1, operand2),
      0x5f => Instruction::thr(Opcode::SRE, 7, AddressMode::AbsX, operand1, operand2),
      0x5b => Instruction::thr(Opcode::SRE, 7, AddressMode::AbsY, operand1, operand2),
      0x43 => Instruction::two(Opcode::SRE, 8, AddressMode::IndX, operand1),
      0x53 => Instruction::two(Opcode::SRE, 8, AddressMode::IndY, operand1),

      0x67 => Instruction::two(Opcode::RRA, 5, AddressMode::Zero, operand1),
      0x77 => Instruction::two(Opcode::RRA, 6, AddressMode::ZeroX, operand1),
      0x6f => Instruction::thr(Opcode::RRA, 6, AddressMode::Abs, operand1, operand2),
      0x7f => Instruction::thr(Opcode::RRA, 7, AddressMode::AbsX, operand1, operand2),
      0x7b => Instruction::thr(Opcode::RRA, 7, AddressMode::AbsY, operand1, operand2),
      0x63 => Instruction::two(Opcode::RRA, 8, AddressMode::IndX, operand1),
      0x73 => Instruction::two(Opcode::RRA, 8, AddressMode::IndY, operand1),

      // TODO illegal, not sure if they should be NOPS or real
      // 0xda => Instruction::imp(Opcode::PHX, 3),
      // 0x5a => Instruction::imp(Opcode::PHY, 3),
      // 0xfa => Instruction::imp(Opcode::PLX, 4),
      // 0x7a => Instruction::imp(Opcode::PLY, 4),
      0xda => Instruction::imp(Opcode::NOP, 2),
      0x5a => Instruction::imp(Opcode::NOP, 2),
      0xfa => Instruction::imp(Opcode::NOP, 2),
      0x7a => Instruction::imp(Opcode::NOP, 2),

      0x0b => Instruction::two(Opcode::ANC, 2, AddressMode::Imm, operand1),
      0x2b => Instruction::two(Opcode::ANC2, 2, AddressMode::Imm, operand1),

      0x4b => Instruction::two(Opcode::ALR, 2, AddressMode::Imm, operand1),
  
      _ => panic!("Unknown opcode: {:#04x}", opbyte)
    }
  }
}