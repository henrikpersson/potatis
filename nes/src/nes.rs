
use std::{rc::Rc, cell::RefCell, time::{Instant, Duration}};
use mos6502::{mos6502::Mos6502, memory::Bus, cpu::{Cpu, Reg}, debugger::Debugger};
use crate::{cartridge::Cartridge, nesbus::NesBus, ppu::ppu::PPU, joypad::Joypad, frame::RenderFrame, fonts};

const SHOW_FPS: bool = true;
const DEFAULT_FPS_MAX: usize = 30;

lazy_static! {
  static ref TIME: Instant = Instant::now();
}

pub trait HostSystem {
  fn render(&mut self, frame: &RenderFrame);
  fn poll_events(&mut self, joypad: &mut Joypad);
  fn elapsed_millis(&self) -> usize {
    TIME.elapsed().as_millis() as usize
  }
  fn delay(&self, d: Duration) {
    // TODO: This should not be a sleep! We still need to poll events, etc. 
    // No need to suspend EVERYTHING.
    std::thread::sleep(d);
  }
}

#[derive(Default)]
struct HeadlessHost;
impl HostSystem for HeadlessHost {
  fn render(&mut self, _: &RenderFrame) {}
  fn poll_events(&mut self, _: &mut Joypad) {}
  fn elapsed_millis(&self) -> usize { 0 }
  fn delay(&self, _: Duration) {}
}

pub struct Nes {
  machine: Mos6502,
  ppu: Rc<RefCell<PPU>>,
  host: Box<dyn HostSystem>,
  joypad: Rc<RefCell<Joypad>>,
  timing: FrameTiming
}

impl Nes {
  pub fn insert<H : HostSystem + 'static>(cartridge: Cartridge, host: H) -> Self {
    let mirroring = cartridge.mirroring();
    let rom_mapper = crate::mappers::for_cart(cartridge);

    let ppu = Rc::new(RefCell::new(PPU::new(rom_mapper.clone(), mirroring)));
    let joypad = Rc::new(RefCell::new(Joypad::default()));
    let bus = NesBus::new(rom_mapper.clone(), ppu.clone(), joypad.clone());

    let mut cpu = Cpu::new(bus);
    cpu.reset();

    let mut machine = Mos6502::new(cpu);
    machine.inc_cycles(7); // Startup cycles.. (not sure, from nestest)

    Self { 
      machine,
      ppu,
      host: Box::new(host),
      joypad,
      timing: FrameTiming::new()
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
  
  pub fn fps_max(&mut self, fps_max: usize) {
    self.timing.fps_max(fps_max);
  }

  pub fn tick(&mut self) {
    self.host.poll_events(&mut self.joypad.borrow_mut());

    let cpu_cycles = self.machine.tick();
    self.ppu.borrow_mut().tick(cpu_cycles * 3);

    let mut ppu = self.ppu.borrow_mut();
  
    if ppu.frame_ready_to_render() {
      if SHOW_FPS {
        let fps = self.timing.fps_avg(self.host.elapsed_millis());
        fonts::draw(fps.to_string().as_str(), (10, 10), ppu.frame_mut());
      }

      self.host.render(ppu.frame());
      ppu.clear_frame_ready();

      if let Some(delay)= self.timing.post_render(self.host.elapsed_millis()) {
        self.host.delay(delay);
      }
      self.timing.post_delay(self.host.elapsed_millis());
    }

    if ppu.is_nmi_pending() {
      // println!("NMI");
      self.machine.cpu_mut().interrupt_nmi();
      ppu.clear_pending_nmi();
    }
  }
}


struct FrameTiming {
  frame_n: usize,
  last_frame_timestamp: usize,
  frame_limit_ms: usize,
}

impl FrameTiming {
  pub fn new() -> Self {
    Self { frame_n: 0, last_frame_timestamp: 0, frame_limit_ms: 1000 / DEFAULT_FPS_MAX }
  }

  pub fn fps_max(&mut self, fps_max: usize) {
    self.frame_limit_ms = 1000 / fps_max;
  }

  pub fn fps_avg(&mut self, elapsed: usize) -> usize {
    let secs = elapsed / 1000;
    if secs != 0 {
      self.frame_n / secs
    } else {
      0
    }
  }

  pub fn post_render(&mut self, elapsed: usize) -> Option<Duration> {
    if self.last_frame_timestamp != 0 {
      let ms_to_render_frame = elapsed - self.last_frame_timestamp;
      // println!("took: {}ms, target: {}ms", ms_to_render_frame, self.frame_limit_ms);
      if ms_to_render_frame < self.frame_limit_ms {
        return Some(Duration::from_millis((self.frame_limit_ms - ms_to_render_frame) as u64));
      }
    }

    None
  }

  pub fn post_delay(&mut self, elapsed: usize) {
    self.frame_n += 1;
    self.last_frame_timestamp = elapsed;
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