use bitflags::bitflags;
use std::cell::{RefCell, Cell};

// https://www.nesdev.org/wiki/PPU_programmer_reference#Controller_($2000)_%3E_write

#[derive(Default)]
pub struct OpenBus { // AKA data bus, decay register
  data: Cell<u8>,
  cycles: usize
}

impl OpenBus { 
  const CPU_CLOCK_HZ: usize = 1_790_000;

  pub fn read(&self) -> u8 {
    self.data.get()
  }

  pub fn write(&self, data: u8) {
    self.data.set(data);
  }

  pub fn tick_for_decay(&mut self) {
    self.cycles += 1;
    if self.cycles >= Self::CPU_CLOCK_HZ {
      self.data.set(0);
      self.cycles = 0;
    }
  }
}

/*
7  bit  0
---- ----
VPHB SINN
|||| ||||
|||| ||++- Base nametable address
|||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
|||| |+--- VRAM address increment per CPU read/write of PPUDATA
|||| |     (0: add 1, going across; 1: add 32, going down)
|||| +---- Sprite pattern table address for 8x8 sprites
||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
|||+------ Background pattern table address (0: $0000; 1: $1000)
||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
|+-------- PPU master/slave select
|          (0: read backdrop from EXT pins; 1: output color on EXT pins)
+--------- Generate an NMI at the start of the
           vertical blanking interval (0: off; 1: on)
*/

bitflags! {
  pub struct ControlRegister: u8 {
    const NAMETABLE1 = 1;
    const NAMETABLE2 = 1 << 1;
    const VRAM_ADDR_INC_PER_PPUDATA_ACCESS = 1 << 2;
    const SPRITE_TABLE_ADDRESS = 1 << 3;
    const BACKGROUND_TABLE_ADDRESS = 1 << 4;
    const SPRITE_SIZE = 1 << 5;
    const MASTER_SLAVE_SELECT = 1 << 6;
    const GENERATE_NMI_AT_VBI = 1 << 7;
  }
}

impl ControlRegister {
  pub fn new() -> ControlRegister {
    ControlRegister::empty() // TODO: startup state?
  }

  // TODO: After power/reset, writes to this register are ignored for about 30,000 cycles.
  pub fn write(&mut self, status: &StatusRegister, v: u8) -> bool {
    let was_nmi_on = self.generate_nmi_at_vblank_interval();

    self.bits = v;

    if self.intersects(Self::SPRITE_SIZE | Self::MASTER_SLAVE_SELECT) {
      todo!("not yet implemented PPU control flag: {:?}", self)
    }

    if self.base_table_address() != 0x2000 {
      println!("WARNING: BASE NAMETABLE SELECTED: {:#06x}", self.base_table_address());
    }

    // If the PPU is currently in vertical blank, and the PPUSTATUS ($2002) vblank flag is still set (1), 
    // changing the NMI flag in bit 7 of $2000 from 0 to 1 will immediately generate an NMI.
    let trigger_nmi = status.in_vblank() && !was_nmi_on && self.generate_nmi_at_vblank_interval();
    trigger_nmi
  }

  pub fn generate_nmi_at_vblank_interval(&self) -> bool {
    self.contains(ControlRegister::GENERATE_NMI_AT_VBI)
  }

  pub fn vram_inc(&self) -> u16 {
    match self.contains(ControlRegister::VRAM_ADDR_INC_PER_PPUDATA_ACCESS) {
      false => 1,
      true => 32,
    }
  }

  pub fn base_table_address(&self) -> u16 {
    match self.bits & 0b00000011 {
      // (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
      0 => 0x2000,
      1 => 0x2400,
      2 => 0x2800,
      3 => 0x2c00,
      _ => unreachable!()
    }
  }

  pub fn background_table_address(&self) -> u16 {
    if self.contains(ControlRegister::BACKGROUND_TABLE_ADDRESS) {
      0x1000
    } else {
      0x0000
    }
  }

  pub fn sprite_table_address(&self) -> u16 {
    if self.contains(ControlRegister::SPRITE_TABLE_ADDRESS) {
      0x1000
    } else {
      0x0000
    }
  }
}

/*
7  bit  0
---- ----
BGRs bMmG
|||| ||||
|||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
|||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
|||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
|||| +---- 1: Show background
|||+------ 1: Show sprites
||+------- Emphasize red (green on PAL/Dendy)
|+-------- Emphasize green (red on PAL/Dendy)
+--------- Emphasize blue
*/

bitflags! {
  #[derive(Default)]
  pub struct MaskRegister: u8 {
    const GRAYSCALE = 1;
    const SHOW_BACKGROUND_LEFT = 1 << 1;
    const SHOW_SPRITES_LEFT = 1 << 2;
    const SHOW_BACKGROUND = 1 << 3;
    const SHOW_SPRITES = 1 << 4;
    const MORE_RED = 1 << 5;
    const MORE_GREEN = 1 << 6;
    const MORE_BLUE = 1 << 7;
  }
}

#[allow(dead_code)]
impl MaskRegister {
  pub fn show_background(self) -> bool {
    self.contains(MaskRegister::SHOW_BACKGROUND)
  }

  pub fn show_background_left(self) -> bool {
    self.contains(MaskRegister::SHOW_BACKGROUND_LEFT)
  }

  pub fn show_sprites(self) -> bool {
    self.contains(MaskRegister::SHOW_SPRITES)
  }

  pub fn show_sprites_left(self) -> bool {
    self.contains(MaskRegister::SHOW_SPRITES_LEFT)
  }

  pub fn write(&mut self, v: u8) {
    self.bits = v;
  }
}

/*
7  bit  0
---- ----
VSO. ....
|||| ||||
|||+-++++- Least significant bits previously written into a PPU register
|||        (due to register not being updated for this address)
||+------- Sprite overflow. The intent was for this flag to be set
||         whenever more than eight sprites appear on a scanline, but a
||         hardware bug causes the actual behavior to be more complicated
||         and generate false positives as well as false negatives; see
||         PPU sprite evaluation. This flag is set during sprite
||         evaluation and cleared at dot 1 (the second dot) of the
||         pre-render line.
|+-------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
|          a nonzero background pixel; cleared at dot 1 of the pre-render
|          line.  Used for raster timing.
+--------- Vertical blank has started (0: not in vblank; 1: in vblank).
           Set at dot 1 of line 241 (the line *after* the post-render
           line); cleared after reading $2002 and at dot 1 of the
           pre-render line.
*/

bitflags! {
  struct StatusFlags: u8 {
    const SPRITE_OVERFLOW = 0b00100000;
    const SPRITE_ZERO_HIT = 0b01000000;
    const IN_VLBANK = 0b10000000;
  }
}

pub struct StatusRegister {
  inner: RefCell<StatusFlags>
}

impl StatusRegister {
  pub fn new() -> StatusRegister {
    StatusRegister { 
      inner: RefCell::new(StatusFlags::empty()) // TODO: startup state?
    }
  }

  pub fn set_vblank(&self, in_vblank: bool) {
    self.inner.borrow_mut().set(StatusFlags::IN_VLBANK, in_vblank);
  }

  pub fn in_vblank(&self) -> bool {
    self.inner.borrow().contains(StatusFlags::IN_VLBANK)
  }

  pub fn read(&self, openbus: &OpenBus) -> u8 {
    let status = self.inner.borrow().bits;

    // Reading 2002 should clear vblank
    self.set_vblank(false);

    // Low 5 bits of $2002 should be from decay value.
    // see tests/ppu_open_bus/readme
    let busdata = openbus.read();
    let status_bus_combined = (status & 0b11100000) | (busdata & 0b00011111);
    openbus.write(status_bus_combined);

    status_bus_combined
  }
}

#[cfg(test)]
mod tests {
  use crate::ppu::registers::{StatusRegister, OpenBus, ControlRegister};

  #[test]
  fn read_status_decay_lower_5() {
    let openbus = OpenBus::default();
    
    let status = StatusRegister::new();
    status.set_vblank(true);

    assert!(status.read(&openbus) == 0x80);
    // first read should clear
    assert!(status.read(&openbus) == 0x00);

    status.set_vblank(true);
    openbus.write(0b00101010);

    assert!(status.read(&openbus) == 0b10001010);

    // vbl cleared
    assert!(status.read(&openbus) == 0b00001010);

    // bus does not mess with vbl
    status.set_vblank(true);
    openbus.write(0);
    assert!(status.in_vblank() == true);
    assert!(status.read(&openbus) == 0x80);

    // also updates the bus
    status.set_vblank(true);
    openbus.write(0);
    status.read(&openbus);
    assert!(openbus.read() == 0x80);
  }

  /*
  |||| ||++- Base nametable address
|||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
   */
  #[test]
  fn ctrl_nametable_base() {
    let mut reg = ControlRegister::new();
    
    reg.bits = 0;
    assert_eq!(reg.base_table_address(), 0x2000);

    reg.bits = 0b00000001;
    assert_eq!(reg.base_table_address(), 0x2400);

    reg.bits = 0b00000010;
    assert_eq!(reg.base_table_address(), 0x2800);

    reg.bits = 0b10000011;
    assert_eq!(reg.base_table_address(), 0x2c00);

    reg.bits = 0b00000111;
    assert_eq!(reg.base_table_address(), 0x2c00);
  }
}