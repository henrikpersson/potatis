use core::cell::RefCell;
use alloc::{rc::Rc};
use common::kilobytes;
use mos6502::memory::Bus;

use crate::{ppu::ppu::Ppu, joypad::Joypad, mappers::Mapper};


pub struct NesBus {
  ram: [u8; kilobytes::KB2],
  rom: Rc<RefCell<dyn Mapper>>,
  ppu: Rc<RefCell<Ppu>>,
  joypad: Rc<RefCell<Joypad>>
}

#[derive(Debug, PartialEq, Eq)]
enum MappedDevice {
  Ram,
  Ppu,
  Apu,
  PpuOamDma,
  Joypad,
  CpuTest,
  Cartridge,
}

impl NesBus {
  pub fn new(rom: Rc<RefCell<dyn Mapper>>, ppu: Rc<RefCell<Ppu>>, joypad: Rc<RefCell<Joypad>>) -> Self {
    Self { 
      rom,
      ram: [0; kilobytes::KB2],
      ppu,
      joypad
    }
  }

  fn map(&self, address: u16) -> (MappedDevice, u16) {
    match address {
      0x0000..=0x07ff => (MappedDevice::Ram, address),
      0x0800..=0x1fff => (MappedDevice::Ram, address & 0x07ff),
      0x2000..=0x2007 => (MappedDevice::Ppu, address - 0x2000),
      0x2008..=0x3fff => (MappedDevice::Ppu, address % 8),
      0x4014          => (MappedDevice::PpuOamDma, address),
      0x4000..=0x4015 => (MappedDevice::Apu, address - 0x4000),
      0x4016..=0x4017 => (MappedDevice::Joypad, address),
      0x4018..=0x401f => (MappedDevice::CpuTest, address - 0x4018),
      0x4020..=0xffff => (MappedDevice::Cartridge, address),
    }
  }
}

impl Bus for NesBus {
  fn read8(&self, address: u16) -> u8 {
    let (device, mapped_address) = self.map(address);
    match device {
      MappedDevice::Ram => self.ram[mapped_address as usize],
      MappedDevice::Ppu => self.ppu.borrow_mut().cpu_read_register(mapped_address),
      MappedDevice::Apu => 0,
      MappedDevice::PpuOamDma => 0,
      MappedDevice::Joypad => {
        match address {
          0x4016 => self.joypad.borrow_mut().read(), // Joystick 1 data
          0x4017 => 0, // Joystick 2 data
          _ => unreachable!()
        }
      }
      MappedDevice::CpuTest => 0,
      MappedDevice::Cartridge => self.rom.borrow().read8(mapped_address),
    }
  }

  fn write8(&mut self, val: u8, address: u16) {
    let (device, mapped_address) = self.map(address);

    match device {
      MappedDevice::Ram => self.ram[mapped_address as usize] = val,
      MappedDevice::Ppu => self.ppu.borrow_mut().cpu_write_register(val, mapped_address),
      MappedDevice::Apu => (),
      MappedDevice::PpuOamDma => {
        // Dump CPU page XX00..XXFF to PPU OAM
        let page_start = (val as u16) << 8;
        let mem = (page_start..=page_start+0xff).map(|addr| self.read8(addr));
        // println!("{:#04x} - dumping {:#06x}..{:#06x}", val, page_start, page_start+0xff);
        self.ppu.borrow_mut().cpu_oam_dma(mem);
      }
      MappedDevice::Joypad => {
        match address {
          0x4016 => self.joypad.borrow_mut().strobe(val), // Joystick strobe
          0x4017 => (), // APU Frame counter control
          _ => unreachable!()
        }  
      }
      MappedDevice::CpuTest => (),
      MappedDevice::Cartridge => self.rom.borrow_mut().write8(val, address),
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{frame::{RenderFrame, PixelFormatRGB888}, cartridge::Mirroring};
  use super::*;

  struct TestBus{}

  impl Mapper for TestBus {}

  impl Bus for TestBus {
    fn read8(&self, _: u16) -> u8 {
      todo!()
    }

    fn write8(&mut self, _: u8, _: u16) {
      todo!()
    }
  }

  fn sut() -> NesBus {
    let bus = Rc::new(RefCell::new(TestBus{}));
    let joypad = Joypad::default();
    let frame = RenderFrame::new::<PixelFormatRGB888>();
    NesBus::new(
      bus.clone(), 
      Rc::new(RefCell::new(Ppu::new(bus, Mirroring::Horizontal, frame))),
      Rc::new(RefCell::new(joypad))
    )
  }

  #[test]
  fn test_map_ram_mirror() {
    let bus = sut();
    
    assert_eq!(bus.map(0x07ff), (MappedDevice::Ram, 0x07ff));
    assert_eq!(bus.map(0x0800), (MappedDevice::Ram, 0x0000));
    assert_eq!(bus.map(0x1fff), (MappedDevice::Ram, 0x07ff));
    assert_eq!(bus.map(0x1001), (MappedDevice::Ram, 0x0001));
  }

  #[test]
  fn test_map_ppu_mirror() {
    let bus = sut();
    
    assert_eq!(bus.map(0x2000), (MappedDevice::Ppu, 0));
    assert_eq!(bus.map(0x3456), (MappedDevice::Ppu, 6));
    assert_eq!(bus.map(0x2008), (MappedDevice::Ppu, 0));
    assert_eq!(bus.map(0x3fff), (MappedDevice::Ppu, 7));

    assert_eq!(bus.map(0x2022), (MappedDevice::Ppu, 2));

    for a in (0x2002..=0x3ffa).step_by(8) {
      assert_eq!(bus.map(a), (MappedDevice::Ppu, 2));
    }

    for a in (0x2007..=0x3fff).step_by(8) {
      assert_eq!(bus.map(a), (MappedDevice::Ppu, 7));
    }

    for a in (0x2000..=0x3fff).step_by(8) {
      assert_eq!(bus.map(a), (MappedDevice::Ppu, 0));
    }
  }
}