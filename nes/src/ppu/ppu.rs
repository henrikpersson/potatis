use core::panic;
use std::{cell::{RefCell, Cell}, rc::Rc};

use common::kilobytes;
use mos6502::memory::Bus;
use crate::{cartridge::Mirroring, frame::RenderFrame};

use super::{registers::{ControlRegister, StatusRegister, OpenBus, MaskRegister}, palette};

const PALETTE_SIZE: usize = 32;

#[derive(Debug)]
#[repr(u16)]
#[allow(dead_code)]
enum Register { 
  PPUCTRL = 0, // ... + base 0x2000
  PPUMASK = 1,
  PPUSTATUS = 2,
  OAMADDR = 3,
  OAMDATA = 4,
  PPUSCROLL = 5,
  PPUADDR = 6,
  PPUDATA = 7,
  OAMDMA = 8
}

impl From<u16> for Register {
  fn from(n: u16) -> Register {
    unsafe { std::mem::transmute(n) } // hehe
  }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
  PreRender,
  Render,
  PostRender,
  VBlank
}

#[allow(dead_code)]
pub struct PPU {
  vram: [u8; kilobytes::KB2], // AKA CIRAM, AKA nametables
  vram_mirroring: Mirroring,

  rom_mapper: Rc<RefCell<dyn Bus>>,
  
  ctrl: ControlRegister,
  status: StatusRegister,
  mask: MaskRegister,
  openbus: OpenBus,

  palettes: [u8; PALETTE_SIZE],

  oam: [u8; 256],
  oam_address: u8,

  cycle: usize,
  scanline: usize,
  frame: RenderFrame,

  nmi_pending: bool,
  
  first_write: Cell<bool>,
  v: Cell<u16>,
  t: Cell<u16>,
  data_buffer: Cell<u8>,

  frame_ready: bool,
}

#[allow(dead_code)]
impl PPU {
  const BLARRG_PALETTE: [u8; PALETTE_SIZE] = [
    0x09,0x01,0x00,0x01,
    0x00,0x02,0x02,0x0D,
    0x08,0x10,0x08,0x24,
    0x00,0x00,0x04,0x2C,
    0x09,0x01,0x34,0x03,
    0x00,0x04,0x00,0x14,
    0x08,0x3A,0x00,0x02,
    0x00,0x20,0x2C,0x08
  ];

  pub fn new(mapper: Rc<RefCell<dyn Bus>>, mirroring: Mirroring) -> PPU {
    PPU {
      rom_mapper: mapper,
      vram: [0; kilobytes::KB2],
      vram_mirroring: mirroring,
      cycle: 21, // from nestest. TODO: will this fuck stuff up?
      scanline: 0, 
      ctrl: ControlRegister::new(),
      status: StatusRegister::new(),
      mask: MaskRegister::default(),
      palettes: Self::BLARRG_PALETTE,
      frame: RenderFrame::new(),
      openbus: OpenBus::default(),
      nmi_pending: false,

      oam: [0; 256],
      oam_address: 0,

      first_write: Cell::new(true),
      v: Cell::new(0),
      t: Cell::new(0),
      data_buffer: Cell::new(0),

      frame_ready: false,
    }
  }

  fn inc_vram(&self) {
    self.v.set(self.v.get() + self.ctrl.vram_inc());
  }

  pub fn cpu_read_register(&self, address: u16) -> u8 {
    let ppu_reg: Register = address.into();
    match ppu_reg {
      Register::PPUSTATUS => {
        self.first_write.set(true);
        self.status.read(&self.openbus)
      }
      Register::PPUDATA => {
        let value = self.internal_read(self.v.get());
        let return_value = match self.v.get() {
          0..=0x3eff => self.data_buffer.get(),
          _ => (value & 0b00111111) | (self.openbus.read() & 0b11000000) // palette, high 2 bits should be from decay
        };
        self.data_buffer.set(value);

        self.inc_vram();
        self.openbus.write(return_value);
        return_value
      }
      Register::OAMDATA => self.oam[self.oam_address as usize],
      _ => self.openbus.read()
    }
  }

  pub fn cpu_write_register(&mut self, val: u8, address: u16) {
    // println!("write {:#06x}", address);
    self.openbus.write(val);

    let ppu_reg: Register = address.into();
    match ppu_reg {
      Register::PPUCTRL => {
        self.nmi_pending = self.ctrl.write(&self.status, val);
      }
      Register::PPUSCROLL => {
        self.first_write.set(!self.first_write.get());
      }
      Register::PPUADDR => {
        let t = self.t.get_mut();
        let v = self.v.get_mut();
        if self.first_write.get() {
          *t = (val as u16) << 8 | (*t & 0x00ff);
        }
        else {
          *t = (*t & 0xFF00) | val as u16;
			    *v = *t;
        }
        
        self.first_write.set(!self.first_write.get());
      }
      Register::PPUDATA => {
        let vram_address = self.v.get();
        self.internal_write(val, vram_address);
        self.inc_vram();
      }
      Register::PPUMASK => self.mask.write(val),
      Register::OAMADDR => self.oam_address = val,
      Register::OAMDATA => {
        self.oam[self.oam_address as usize] = val;
        self.oam_address = self.oam_address.wrapping_add(1);
      }
      _ => ()
    }
  }

  pub fn tick(&mut self, ppu_cycles_to_tick: usize) {
    for _ in 0..ppu_cycles_to_tick {
      self.openbus.tick_for_decay();

      if self.mask.show_background() || self.mask.show_background_left() {
        self.render_background_pixel();
      }

      if self.mask.show_sprites() || self.mask.show_sprites_left() {
        self.render_sprite_pixel();
      }

      if self.cycle == 1 {
        if self.scanline == 241 {
          self.status.set_vblank(true);
          if self.ctrl.generate_nmi_at_vblank_interval() {
            self.nmi_pending = true;
          }
        } else if self.scanline == 261 {
          self.frame_ready = true;
          self.status.set_vblank(false);
        }
      }

      self.cycle += 1;
      if self.cycle > 340 {
        self.cycle = 0;
        self.scanline += 1;

        if self.scanline > 261 {
          self.scanline = 0;
        }
      }
    }
  }

  fn render_background_pixel(&mut self) {
    let x = self.cycle as u16;
    let y = self.scanline as u16;

    if x > 256 || y > 240 {
      return
    }

    let nametable_base = 0x2000;
    let yoffset = (y / 8) * 32;
    let xoffset = x / 8;
    let address = nametable_base + xoffset + yoffset;

    let bg_offset = if self.ctrl.background_table_address() != 0 { 256 } else { 0 };
    let tile = self.internal_read(address) as u16 + bg_offset;
    let attr = self.lookup_attribute_table(address);

    let row = y % 8;
    let plane1 = self.internal_read(tile * 16 + row);
    let plane2 = self.internal_read(tile * 16 + row + 8);

    let col = x % 8;
    let a = if (plane1 & (1 << col)) != 0 { 1 } else { 0 };
    let b = if (plane2 & (1 << col)) != 0 { 2 } else { 0 };
    let palette_index = a + b;

    let mut color_index = self.palettes[(attr * 4 + palette_index) as usize];
    if palette_index == 0 {
      color_index = self.palettes[0]; // TODO?
    }

    let pixel = palette::palette_to_rgb(color_index);
    let reverse_x = (x - col) + (7 - col);
    self.frame.set_pixel(reverse_x as usize, y as usize, pixel);
  }

  fn render_sprite_pixel(&mut self) {
    if self.cycle > 256 || self.scanline > 240 {
      return
    }

    for oam_index in (0..64).step_by(4) { // TODO
      let y = self.oam[oam_index];
      let sprite_index = self.oam[oam_index + 1];
      let attrs = self.oam[oam_index + 2];
      let x = self.oam[oam_index + 3];

      // is it visible?
      if x >= 249 || y >= 239 {
        continue;
      }

      let offset = if self.ctrl.sprite_table_address() != 0 { 256 } else { 0 };
      let tile: u16 = sprite_index as u16 + offset;
      let flip_x = attrs & 0b01000000;
      let flip_y = attrs & 0b10000000;

      let row = (self.scanline % 8) as u16;
      let plane1 = self.internal_read(tile * 16 + row);
      let plane2 = self.internal_read(tile * 16 + row + 8);

      let col = self.cycle % 8;
      let a = if (plane1 & (1 << col)) != 0 { 1 } else { 0 };
      let b = if (plane2 & (1 << col)) != 0 { 2 } else { 0 };
      let palette_index = a + b;

      let color_index = self.palettes[(0x10 + (attrs & 0x03) * 4 + palette_index) as usize];
      if palette_index == 0 { // transparent??
        continue;
      }

      let rgb = palette::palette_to_rgb(color_index);
      let x_offset = if flip_x == 0 { 7 - col as usize } else { col as usize };
      let y_offset = if flip_y == 0 { row as usize } else { 7 - row as usize };
      self.frame.set_pixel(x as usize + x_offset, y as usize + y_offset, rgb);
    }
  }

  fn lookup_attribute_table(&mut self, vram_address: u16) -> u8 {
    // 32x32 attr table address
	  let row = ((vram_address & 0x3e0) >> 5) / 4;
	  let col = (vram_address & 0x1f) / 4;

	  // 16x16 metatile??
    let a = if (vram_address & 0b01000000) != 0 { 4 } else { 0 };
    let b = if (vram_address & 0b00000010) != 0 { 2 } else { 0 };
	  let shift = a + b;

	  // attr table offset
	  let offset = (vram_address & 0xc00) + 0x400 - 64 + (row * 8 + col);

	  (self.vram[offset as usize] & (0b0000011 << shift)) >> shift
  }

  pub fn cpu_oam_dma(&mut self, mem: &[u8]) {
    assert!(mem.len() == 256);
    for byte in mem {
      self.oam[self.oam_address as usize] = *byte;
      self.oam_address = self.oam_address.wrapping_add(1);
    }
  }

  pub fn frame(&self) -> &RenderFrame {
    &self.frame
  }

  pub fn scanline(&self) -> usize {
    self.scanline
  }

  pub fn current_cycle(&self) -> usize {
    self.cycle
  }

  pub fn is_nmi_pending(&self) -> bool {
    self.nmi_pending
  }

  pub fn clear_pending_nmi(&mut self) {
    self.nmi_pending = false;
  }

  pub fn frame_ready_to_render(&self) -> bool {
    self.frame_ready
  }

  pub fn clear_frame_ready(&mut self) {
    self.frame_ready = false;
  }

  fn mirror_vram(mode: &Mirroring, vram_address: u16) -> u16 {
    // https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    // https://www.nesdev.org/wiki/PPU_nametables

    // substract the 0x2000 base for vram, divide by nametable size (1kb) to get the table index.
    let name_table = (vram_address - 0x2000) as usize / common::kilobytes::KB1;

    let mapped_address = match (&mode, name_table) {
        (Mirroring::Vertical, 0) => vram_address,
        (Mirroring::Vertical, 1) => vram_address,
        (Mirroring::Vertical, 2) => vram_address - common::kilobytes::KB2 as u16,
        (Mirroring::Vertical, 3) => vram_address - common::kilobytes::KB2 as u16,
        (Mirroring::Horizontal, 0) => vram_address,
        (Mirroring::Horizontal, 1) => vram_address - common::kilobytes::KB1 as u16,
        (Mirroring::Horizontal, 2) => vram_address - common::kilobytes::KB1 as u16,
        (Mirroring::Horizontal, 3) => vram_address - common::kilobytes::KB2 as u16,
        _ => panic!("nametable mirroring? {}", name_table) //vram_address,
    };
    
    // substract vram base because the bus is gonna index the 2kb array directly.
    mapped_address - 0x2000
  }

  fn internal_read(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x1fff => self.rom_mapper.borrow().read8(address), // CHR
      0x2000..=0x2fff => self.vram[Self::mirror_vram(&self.vram_mirroring, address) as usize],
      0x3000..=0x3eff => self.vram[Self::mirror_vram(&self.vram_mirroring, address - 0x1000) as usize], // -0x1000 because mirror_vram expects base 0x2000
      0x3f00..=0x3fff => {
        // Palette incl mirrors
        let i = address as usize % PALETTE_SIZE;
        self.palettes[i]
      },
      _  => 0
    }
  }
  
  fn internal_write(&mut self, val: u8, address: u16) {
    match address {
      0x0000..=0x1fff => self.rom_mapper.borrow_mut().write8(val, address), // CHR RAM
      0x2000..=0x2fff => self.vram[Self::mirror_vram(&self.vram_mirroring, address) as usize] = val,
      0x3000..=0x3eff => self.vram[Self::mirror_vram(&self.vram_mirroring, address - 0x1000) as usize] = val, // -0x1000 because mirror_vram expects base 0x2000
      0x3f00..=0x3fff => {
        // Palette incl mirrors
        let i = address as usize % PALETTE_SIZE;
        self.palettes[i] = val;
      },
      _  => ()
    }
  }
}


#[cfg(test)]
mod tests {
  use crate::cartridge::Mirroring;
  use super::PPU;

  #[test]
  fn vram_mirror() {
    let nametable1_base = 0; // 0x2000
    let nametable2_base = 0x400; // 0x2800
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2400) == nametable1_base);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2401) == nametable1_base + 1);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2000) == nametable1_base);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2001) == nametable1_base + 1);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x24ff) == nametable1_base + 0xff);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2800) == nametable2_base);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2801) == nametable2_base + 1);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x28ff) == nametable2_base + 0xff);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2c00) == nametable2_base);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2c01) == nametable2_base + 1);
    assert!(PPU::mirror_vram(&Mirroring::Horizontal, 0x2cff) == nametable2_base + 0xff);

    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2000) == nametable1_base);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2800) == nametable1_base);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2801) == nametable1_base + 1);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x28ff) == nametable1_base + 0xff);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2001) == nametable1_base + 1);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x24ff) == nametable2_base + 0xff);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2c00) == nametable2_base);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2c01) == nametable2_base + 1);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2cff) == nametable2_base + 0xff);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2400) == nametable2_base);
    assert!(PPU::mirror_vram(&Mirroring::Vertical, 0x2401) == nametable2_base + 1);
  }

  fn map(x: u8, y: u8) -> u16 {
    let yoffset = (y / 8) * 32;
    let xoffset = x / 8;
    0x2000 + xoffset as u16 + yoffset as u16
  }

  fn map_real(x: u8, y: u8) -> u16 {
    let yoffset = (y / 8) * 32;
    let xoffset = x / 8;
    0x2000 + xoffset as u16 + yoffset as u16
  }

  #[test]
  fn ixymap() {
    assert!(map(0, 0) == 0x2000);
    assert!(map(8, 0) == 0x2001);
    assert!(map(248, 0) == 0x201f);
    assert!(map(0, 8) == 0x2020);
    assert!(map(8, 8) == 0x2021);
  }

  #[test]
  fn ixymap_real() {
    assert!(map_real(7, 0) == 0x2000);
    assert!(map_real(7, 1) == 0x2000);
    assert!(map_real(0, 7) == 0x2000);
    assert!(map_real(15, 0) == 0x2001);
    assert!(map_real(9, 3) == 0x2001);
  }
}