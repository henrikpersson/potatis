#[derive(Default, PartialEq, Eq, Copy, Clone)]
pub(crate) enum Phase {
  PreRender,
  #[default] Render,
  PostRender,
  EnteringVblank,
  Vblank
}

pub(crate) enum Rendering { Enabled, Disabled }

#[derive(Default)]
pub(crate) struct State {
  phase: Phase,
  cycle: usize,
  scanline: usize,
  clock: usize,
  odd_frame: bool,
}

impl State {
  pub fn next(&mut self, rendering_enabled: bool) -> (Phase, usize, Rendering) {
    self.cycle = self.clock % 341;
    self.scanline = self.clock / 341;
    self.clock += 1;

    self.phase = match self.scanline {
      261 => Phase::PreRender,
      0..=239 => Phase::Render,
      240 => Phase::PostRender,
      241 => Phase::EnteringVblank,
      242..=260 => Phase::Vblank,
      _ => unreachable!()
    };

    if self.phase == Phase::PreRender {
      if self.cycle == 339 && self.odd_frame && rendering_enabled {
        self.clock = 0;
      }
      if self.cycle == 340 {
        self.clock = 0;
      }
    }

    self.odd_frame = !self.odd_frame;

    (
      self.phase, 
      self.cycle, 
      if rendering_enabled { Rendering::Enabled } else { Rendering::Disabled }
    )
  }

  pub fn even_frame(&self) -> bool {
    !self.odd_frame
  }

  pub fn scanline(&self) -> usize {
    self.scanline
  }

  pub fn cycle(&self) -> usize {
    self.cycle
  }

  #[allow(dead_code)]
  pub fn clock(&self) -> usize {
    self.clock
  }
}