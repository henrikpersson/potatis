use std::{rc::Rc, cell::RefCell};

use common::kilobytes;
use mos6502::memory::Bus;

use crate::{ppu::ppu::PPU, joypad::Joypad};


pub struct NesBus {
  ram: [u8; kilobytes::KB2],
  rom: Rc<RefCell<dyn Bus>>,
  ppu: Rc<RefCell<PPU>>,
  joypad: Rc<RefCell<Joypad>>
}

#[derive(Debug, PartialEq, Eq)]
enum MappedDevice {
  RAM,
  PPU,
  APU,
  PPUOAMDMA,
  JOYPAD,
  CPUTEST,
  CARTRIDGE,
}

impl NesBus {
  pub fn new(rom: Rc<RefCell<dyn Bus>>, ppu: Rc<RefCell<PPU>>, joypad: Rc<RefCell<Joypad>>) -> Self {
    Self { 
      rom: rom,
      ram: [0; kilobytes::KB2],
      ppu: ppu,
      joypad
    }
  }

  fn map(&self, address: u16) -> (MappedDevice, u16) {
    match address {
      0x0000..=0x07ff => (MappedDevice::RAM, address),
      0x0800..=0x1fff => (MappedDevice::RAM, address & 0x07ff),
      0x2000..=0x2007 => (MappedDevice::PPU, address - 0x2000),
      0x2008..=0x3fff => (MappedDevice::PPU, address % 8),
      0x4014          => (MappedDevice::PPUOAMDMA, address),
      0x4000..=0x4015 => (MappedDevice::APU, address - 0x4000),
      0x4016..=0x4017 => (MappedDevice::JOYPAD, address),
      0x4018..=0x401f => (MappedDevice::CPUTEST, address - 0x4018),
      0x4020..=0xffff => (MappedDevice::CARTRIDGE, address),
    }
  }
}

impl Bus for NesBus {
  fn read8(&self, address: u16) -> u8 {
    let (device, mapped_address) = self.map(address);
    match device {
      MappedDevice::RAM => self.ram[mapped_address as usize],
      MappedDevice::PPU => self.ppu.borrow().cpu_read_register(mapped_address),
      MappedDevice::APU => 0,
      MappedDevice::PPUOAMDMA => 0,
      MappedDevice::JOYPAD => {
        match address {
          0x4016 => self.joypad.borrow_mut().read(), // Joystick 1 data
          0x4017 => 0, // Joystick 2 data
          _ => unreachable!()
        }
      }
      MappedDevice::CPUTEST => 0,
      MappedDevice::CARTRIDGE => self.rom.borrow().read8(mapped_address),
    }
  }

  fn write8(&mut self, val: u8, address: u16) {
    let (device, mapped_address) = self.map(address);

    match device {
      MappedDevice::RAM => self.ram[mapped_address as usize] = val,
      MappedDevice::PPU => self.ppu.borrow_mut().cpu_write_register(val, mapped_address),
      MappedDevice::APU => (),
      MappedDevice::PPUOAMDMA => {
        // Dump CPU page XX00..XXFF to PPU OAM
        let page_start = (val as u16) << 8;
        let mem: Vec<u8> = (page_start..=page_start+0xff).map(|addr| self.read8(addr)).collect();
        // println!("{:#04x} - dumping {:#06x}..{:#06x}", val, page_start, page_start+0xff);
        self.ppu.borrow_mut().cpu_oam_dma(&mem[..]);
      }
      MappedDevice::JOYPAD => {
        match address {
          0x4016 => self.joypad.borrow_mut().strobe(val), // Joystick strobe
          0x4017 => (), // APU Frame counter control
          _ => unreachable!()
        }  
      }
      MappedDevice::CPUTEST => (), // TODO
      MappedDevice::CARTRIDGE => self.rom.borrow_mut().write8(val, address),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct TestBus{}

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
    NesBus::new(
      bus.clone(), 
      Rc::new(RefCell::new(PPU::new(bus, crate::cartridge::Mirroring::FourScreen))),
      Rc::new(RefCell::new(joypad))
    )
  }

  #[test]
  fn test_map_ram_mirror() {
    let bus = sut();
    
    assert_eq!(bus.map(0x07ff), (MappedDevice::RAM, 0x07ff));
    assert_eq!(bus.map(0x0800), (MappedDevice::RAM, 0x0000));
    assert_eq!(bus.map(0x1fff), (MappedDevice::RAM, 0x07ff));
    assert_eq!(bus.map(0x1001), (MappedDevice::RAM, 0x0001));
  }

  #[test]
  fn test_map_ppu_mirror() {
    let bus = sut();
    
    assert_eq!(bus.map(0x2000), (MappedDevice::PPU, 0));
    assert_eq!(bus.map(0x3456), (MappedDevice::PPU, 6));
    assert_eq!(bus.map(0x2008), (MappedDevice::PPU, 0));
    assert_eq!(bus.map(0x3fff), (MappedDevice::PPU, 7));

    assert_eq!(bus.map(0x2022), (MappedDevice::PPU, 2));

    for a in (0x2002..=0x3ffa).step_by(8) {
      assert_eq!(bus.map(a), (MappedDevice::PPU, 2));
    }

    for a in (0x2007..=0x3fff).step_by(8) {
      assert_eq!(bus.map(a), (MappedDevice::PPU, 7));
    }

    for a in (0x2000..=0x3fff).step_by(8) {
      assert_eq!(bus.map(a), (MappedDevice::PPU, 0));
    }
  }
}