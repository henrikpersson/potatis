use alloc::boxed::Box;

use crate::{cpu::{Cpu, Reg}, memory::Bus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressMode {
  Abs,
  AbsX,
  AbsY,
  Imm,
  Impl,
  Ind,
  IndX,
  IndY,
  Rel,
  Zero,
  ZeroX,
  ZeroY,
  Nop, // Not official.. used for dev
}

impl AddressMode {
  pub fn resolve(&self, cpu: &mut Cpu, operands: (Option<u8>, Option<u8>), num_extra_cycles: usize) -> u16 {
    let mem = cpu.bus();

    if self.is_zeropage() {
      self.resolve_zeropage(cpu, operands.0.unwrap(), num_extra_cycles)
    }
    else {
      let low = operands.0.unwrap();
      let high = operands.1.unwrap();
      let address: u16 = ((high as u16) << 8) | low as u16;

      match self {
        AddressMode::Abs => address,
        AddressMode::AbsX => self.cycle_aware_add(cpu, address, cpu[Reg::X], num_extra_cycles),
        AddressMode::AbsY => self.cycle_aware_add(cpu, address, cpu[Reg::Y], num_extra_cycles),
        AddressMode::Ind => self.read16(mem, low, high),
        _ => panic!()
      }
    }
  }

  fn resolve_zeropage(&self, cpu: &mut Cpu, operand: u8, likes_extra_cycles: usize) -> u16 {
    // Zeropage indices should wrap!
    // Casting everything to u16 here is safe because hi == 0x00 == zeropage!
    match self {
      AddressMode::IndX => self.read16(cpu.bus(), operand.wrapping_add(cpu[Reg::X]), 0x00), // Zeropage, no carry
      AddressMode::IndY => {
        let address = self.read16(cpu.bus(), operand, 0x00);
        self.cycle_aware_add(cpu, address, cpu[Reg::Y], likes_extra_cycles)
      }
      AddressMode::Zero => operand as u16,
      AddressMode::ZeroX => operand.wrapping_add(cpu[Reg::X]) as u16, // Zeropage
      AddressMode::ZeroY => operand.wrapping_add(cpu[Reg::Y]) as u16, // zeropage
      _ => panic!()
    }
  }

  fn cycle_aware_add(&self, cpu: &mut Cpu, address: u16, v: u8, likes_extra_cycles: usize) -> u16 {
    let res = address.wrapping_add(v as u16);
    // println!("{:#06x} + after: {:#06x}", address, res);
    if res & 0xff00 != address & 0xff00 {
      // page cross
      cpu.add_extra_cycles(likes_extra_cycles);
    }
    res
  }

  fn read16(&self, mem: &Box<dyn Bus>, address_low: u8, address_hi: u8) -> u16 {
    let byte1_address = ((address_hi as u16) << 8) | address_low as u16;
    let byte2_address = ((address_hi as u16) << 8) | address_low.wrapping_add(1) as u16;
    let val_low = mem.read8(byte1_address) as u16;
    let val_high = mem.read8(byte2_address) as u16;
    (val_high << 8) | val_low
  }

  fn is_zeropage(&self) -> bool {
    matches!(self, 
      AddressMode::IndX  |
      AddressMode::IndY  |
      AddressMode::Zero  |
      AddressMode::ZeroX |
      AddressMode::ZeroY 
    )
  }
}