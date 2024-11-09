use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::ToString;
use core::cell::RefCell;
use core::time::Duration;
use mos6502::cpu::AC;
use mos6502::cpu::SP;
use mos6502::cpu::X;
use mos6502::cpu::Y;

use mos6502::cpu::Cpu;
#[cfg(feature = "debugger")]
use mos6502::debugger::AttachedDebugger;
use mos6502::mos6502::Mos6502;

use crate::cartridge::Cartridge;
use crate::cartridge::Rom;
use crate::fonts;
use crate::frame::PixelFormatRGB565;
use crate::frame::PixelFormatRGB888;
use crate::frame::RenderFrame;
use crate::joypad::Joypad;
use crate::nesbus::NesBus;
use crate::ppu::ppu::Ppu;
use crate::ppu::ppu::TickEvent;

const DEFAULT_FPS_MAX: usize = 60;

#[derive(PartialEq, Eq)]
pub enum Shutdown {
  Yes,
  No,
  Reset,
}

impl From<bool> for Shutdown {
  fn from(b: bool) -> Self {
    if b {
      Shutdown::Yes
    } else {
      Shutdown::No
    }
  }
}

#[derive(PartialEq, Default)]
pub enum HostPixelFormat {
  #[default]
  Rgb888,
  Rgb565,
}

pub trait HostPlatform {
  fn render(&mut self, frame: &RenderFrame);
  fn poll_events(&mut self, joypad: &mut Joypad) -> Shutdown;

  fn elapsed_millis(&self) -> usize {
    // Not required. Up to platform to implement for FPS control.
    0
  }

  fn delay(&self, _: Duration) {
    // Not required. Up to platform to implement for FPS control.
  }

  fn pixel_format(&self) -> HostPixelFormat {
    HostPixelFormat::default()
  }

  fn alloc_render_frame(&self) -> RenderFrame {
    match self.pixel_format() {
      HostPixelFormat::Rgb888 => RenderFrame::new::<PixelFormatRGB888>(),
      HostPixelFormat::Rgb565 => RenderFrame::new::<PixelFormatRGB565>(),
    }
  }
}

#[derive(Default)]
struct HeadlessHost;
impl HostPlatform for HeadlessHost {
  fn render(&mut self, _: &RenderFrame) {}
  fn poll_events(&mut self, _: &mut Joypad) -> Shutdown {
    Shutdown::No
  }
  fn elapsed_millis(&self) -> usize {
    0
  }
  fn delay(&self, _: Duration) {}
}

pub struct Nes {
  machine: Mos6502<NesBus>,
  ppu: Rc<RefCell<Ppu>>,
  host: Box<dyn HostPlatform>,
  joypad: Rc<RefCell<Joypad>>,
  timing: FrameTiming,
  pub show_fps: bool,
  shutdown: Shutdown,
}

impl Nes {
  pub fn insert<H: HostPlatform + 'static, R: Rom + 'static>(
    cartridge: Cartridge<R>,
    host: H,
  ) -> Self {
    let mirroring = cartridge.mirroring();
    let rom_mapper = crate::mappers::for_cart(cartridge);

    let frame = host.alloc_render_frame();
    let ppu = Rc::new(RefCell::new(Ppu::new(rom_mapper.clone(), mirroring, frame)));
    let joypad = Rc::new(RefCell::new(Joypad::default()));
    let bus = NesBus::new(rom_mapper.clone(), ppu.clone(), joypad.clone());

    let mut cpu = Cpu::new(bus);
    cpu.reset();

    let machine = Mos6502::new(cpu);

    Self {
      machine,
      ppu,
      host: Box::new(host),
      joypad,
      timing: FrameTiming::new(),
      shutdown: Shutdown::No,
      show_fps: false,
    }
  }

  pub fn insert_headless_host<R: Rom + 'static>(cartridge: Cartridge<R>) -> Self {
    Self::insert(cartridge, HeadlessHost)
  }

  pub fn tick(&mut self) {
    let cpu_cycles = self.machine.tick();

    let mut ppu = self.ppu.borrow_mut();
    let ppu_event = ppu.tick(cpu_cycles * 3);

    if ppu_event == TickEvent::EnteredVblank {
      if self.show_fps {
        let fps = self.timing.fps_avg(self.host.elapsed_millis());
        fonts::draw(fps.to_string().as_str(), (10, 10), ppu.frame_mut());
      }

      self.host.render(ppu.frame());
      self.shutdown = self.host.poll_events(&mut self.joypad.borrow_mut());
      if let Some(delay) = self.timing.post_render(self.host.elapsed_millis()) {
        self.host.delay(delay);
      }
      self.timing.post_delay(self.host.elapsed_millis());

      if ppu.nmi_on_vblank() {
        self.machine.cpu.nmi();
      }
    }

    if ppu_event == TickEvent::TriggerIrq {
      self.machine.cpu.irq();
    }

    if self.shutdown == Shutdown::Reset {
      self.machine.cpu.reset();
      self.shutdown = Shutdown::No
    }
  }

  #[cfg(feature = "debugger")]
  pub fn debugger(&mut self) -> AttachedDebugger<NesBus> {
    self.machine.debugger()
  }

  pub fn cpu_cycles(&self) -> usize {
    self.machine.total_cycles
  }

  pub fn cpu(&self) -> &Cpu<NesBus> {
    &self.machine.cpu
  }

  pub fn cpu_mut(&mut self) -> &mut Cpu<NesBus> {
    &mut self.machine.cpu
  }

  pub fn bus(&self) -> &NesBus {
    &self.machine.cpu.bus
  }

  pub fn fps_max(&mut self, fps_max: usize) {
    self.timing.fps_max(fps_max);
  }

  pub fn show_fps(&mut self, show_fps: bool) {
    self.show_fps = show_fps;
  }

  pub fn powered_on(&self) -> bool {
    self.shutdown != Shutdown::Yes
  }
}

struct FrameTiming {
  frame_n: usize,
  last_frame_timestamp: usize,
  frame_limit_ms: usize,
}

impl FrameTiming {
  pub fn new() -> Self {
    Self {
      frame_n: 0,
      last_frame_timestamp: 0,
      frame_limit_ms: 1000 / DEFAULT_FPS_MAX,
    }
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
        return Some(Duration::from_millis(
          (self.frame_limit_ms - ms_to_render_frame) as u64,
        ));
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
impl core::fmt::Debug for Nes {
  // A:00 X:00 Y:00 P:26 SP:FB PPU:  0,120 CYC:40
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let c = self.cpu();
    let scanline = self.ppu.borrow_mut().scanline();
    let ppu_cycle = self.ppu.borrow_mut().cycle() + 21;
    let ppuw = 3;
    if ppu_cycle < 100 {
      write!(
        f,
        "{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:ppuw$}, {:>2} CYC:{}",
        c.pc,
        c.regs[AC],
        c.regs[X],
        c.regs[Y],
        c.flags.bits(),
        c.regs[SP],
        scanline,
        ppu_cycle,
        self.machine.total_cycles
      )
    } else {
      write!(
        f,
        "{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:ppuw$},{:>2} CYC:{}",
        c.pc,
        c.regs[AC],
        c.regs[X],
        c.regs[Y],
        c.flags.bits(),
        c.regs[SP],
        scanline,
        ppu_cycle,
        self.machine.total_cycles
      )
    }
  }
}
