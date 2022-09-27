
use std::{rc::Rc, cell::RefCell};

use mos6502::{mos6502::Mos6502, memory::{Bus}, cpu::{Cpu, Reg}, debugger::Debugger};
use crate::{cartridge::Cartridge, nesbus::NesBus, ppu::ppu::PPU, joypad::Joypad, frame::RenderFrame};

pub trait HostSystem {
  fn render(&mut self, frame: &RenderFrame);
  fn poll_events(&mut self, joypad: &mut Joypad);
}

#[derive(Default)]
struct HeadlessHost;
impl HostSystem for HeadlessHost {
  fn render(&mut self, _: &RenderFrame) {}
  fn poll_events(&mut self, _: &mut Joypad) {}
}

pub struct Nes {
  machine: Mos6502,
  ppu: Rc<RefCell<PPU>>,
  host: Box<dyn HostSystem>,
  joypad: Rc<RefCell<Joypad>>,
}

impl Nes {
  pub fn insert<H : HostSystem + 'static>(cartridge: Cartridge, host: H) -> Self {
    let mirroring = cartridge.mirroring();
    let rom_mapper = crate::mappers::for_cart(cartridge);

    let ppu = Rc::new(RefCell::new(PPU::new(rom_mapper.clone(), mirroring)));
    let joypad = Rc::new(RefCell::new(Joypad::default()));
    let bus = NesBus::new(rom_mapper.clone(), ppu.clone(), joypad.clone());

    let mut cpu = Cpu::new(Box::new(bus));
    cpu.reset();

    let mut machine = Mos6502::new(cpu);
    machine.inc_cycles(7); // Startup cycles.. (not sure, from nestest)

    Self { 
      machine,
      ppu,
      host: Box::new(host),
      joypad
    }
  }

  pub fn insert_headless_host(cartridge: Cartridge) -> Self {
    Self::insert(cartridge, HeadlessHost::default())
  }

  pub fn debugger(&mut self) -> &mut Debugger {
    self.machine.debugger()
  }

  pub fn cpu(&self) -> &Cpu {
    self.machine.cpu()
  }

  pub fn cpu_mut(&mut self) -> &mut Cpu {
    self.machine.cpu_mut()
  }

  pub fn bus(&self) -> &Box<dyn Bus> {
    self.machine.bus()
  }

  pub fn cpu_ticks(&self) -> usize {
    self.machine.ticks()
  }

  pub fn tick(&mut self) {
    let cpu_cycles = self.machine.tick();
    self.ppu.borrow_mut().tick(cpu_cycles * 3);

    self.host.poll_events(&mut self.joypad.borrow_mut());

    let mut ppu = self.ppu.borrow_mut();
  
    if ppu.frame_ready_to_render() {
      let frame = ppu.frame();
      self.host.render(frame);
      ppu.clear_frame_ready();
    }

    if ppu.is_nmi_pending() {
      // println!("NMI");
      self.machine.cpu_mut().interrupt_nmi();
      ppu.clear_pending_nmi();
    }
  }
}

// mainly for nestest
impl std::fmt::Debug for Nes {
  // A:00 X:00 Y:00 P:26 SP:FB PPU:  0,120 CYC:40
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let c = self.cpu();
    let scanline = self.ppu.borrow_mut().scanline();
    let ppu_cycle = self.ppu.borrow_mut().current_cycle();
    // let ppuw = if scanline >= 10 { 3 } else { 3 };
    let ppuw = 3;
    if ppu_cycle < 100 {
      write!(f, 
        "{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:ppuw$}, {:>2} CYC:{}", 
        c.pc(),
        c[Reg::AC], c[Reg::X], c[Reg::Y], c.flags_as_byte(), c[Reg::SP],
        scanline, ppu_cycle,
        self.machine.cycles()
      )
    }
    else {
      write!(f, 
        "{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:ppuw$},{:>2} CYC:{}", 
        c.pc(),
        c[Reg::AC], c[Reg::X], c[Reg::Y], c.flags_as_byte(), c[Reg::SP],
        scanline, ppu_cycle,
        self.machine.cycles()
      )
    }
  }
}