const PALETTE_SIZE: usize = 32;

// AKA boot palette?
pub const BLARRG_PALETTE: [u8; PALETTE_SIZE] = [
  0x09,0x01,0x00,0x01,
  0x00,0x02,0x02,0x0D,
  0x08,0x10,0x08,0x24,
  0x00,0x00,0x04,0x2C,
  0x09,0x01,0x34,0x03,
  0x00,0x04,0x00,0x14,
  0x08,0x3A,0x00,0x02,
  0x00,0x20,0x2C,0x08
];

pub struct Palette {
  data: [u8; PALETTE_SIZE]
}

impl Palette {
  pub fn new() -> Self {
    Self {
      data: BLARRG_PALETTE
    }
  }

  pub fn write(&mut self, val: u8, address: u16) {
    let mirrored = Self::mirror(address) as usize;
    self.data[mirrored % PALETTE_SIZE] = val;
  }

  pub fn read(&self, address: u16) -> u8 {
    let mirrored = Self::mirror(address) as usize;
    self.data[mirrored % PALETTE_SIZE]
  }

  pub fn rgb_from_index(&self, index: u8) -> (u8, u8, u8) {
    palette_to_rgb(self.data[index as usize])
  }

  // 0x3f00..=0x3fff
  fn mirror(address: u16) -> u16 {
    // PPU mem layout mirroring
    let mirrored = match address {
      0x3f00..=0x3f1f => address,
      0x3f20..=0x3fff => 0x3f00 + (address % PALETTE_SIZE as u16),
      _ => panic!("invalid palette address: {:#06x}", address)
    };
    
    // Special palette crap: Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
    match mirrored {
      0x3f10 => 0x3f00,
      0x3f14 => 0x3f04,
      0x3f18 => 0x3f08,
      0x3f1c => 0x3f0c,
      _ => mirrored
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::ppu::palette::Palette;

  #[test]
  fn palette_mirror() {
    assert_eq!(Palette::mirror(0x3f00), 0x3f00);
    assert_eq!(Palette::mirror(0x3f1f), 0x3f1f);
    assert_eq!(Palette::mirror(0x3f20), 0x3f00);
    assert_eq!(Palette::mirror(0x3fff), 0x3f1f);
    assert_eq!(Palette::mirror(0x3f21), 0x3f01);
    assert_eq!(Palette::mirror(0x3f40), 0x3f00);

    assert_eq!(Palette::mirror(0x3f10), 0x3f00);
    assert_eq!(Palette::mirror(0x3f14), 0x3f04);
    assert_eq!(Palette::mirror(0x3f18), 0x3f08);
    assert_eq!(Palette::mirror(0x3f1c), 0x3f0c);
  }
}

// https://www.nesdev.org/wiki/PPU_palettes
pub fn palette_to_rgb(value: u8) -> (u8, u8, u8) {
  match value {
      0x00 => (101, 101, 101),
      0x01 => (0  ,  45, 105),
      0x02 => (19,   31, 127),
      0x03 => (69 ,  19, 124),
      0x04 => (96 ,  11,  98),
      0x05 => (115,  10,  55),
      0x06 => (113,  15,   7),
      0x07 => (90 ,  26,   0),
      0x08 => (52 ,  40,   0),
      0x09 => (11 ,  52,   0),
      0x0a => (0,   60,    0),    
      0x0b => (0,   61,   16),    
      0x0c => (0,   56,   64),
      0x0d => (0,    0,    0),
      0x0e => (0,    0,    0),
      0x0f => (0,    0,    0),
      0x10 => (174,174 ,174),
      0x11 => (15 ,  99,179),
      0x12 => (64 ,  81, 208),
      0x13 => (120,  65, 204),
      0x14 => (167,  54, 169),
      0x15 => (192,  52, 112),
      0x16 => (189,  60,  48),
      0x17 => (159,  74,   0),
      0x18 => (109,  92,   0),
      0x19 => (54 , 109 ,  0),
      0x1a => (7 ,  119 ,  4),
      0x1b => (0 ,  121 , 61),
      0x1c => (0,   114 ,125),
      0x1d => (0,     0,   0),
      0x1e => (0,     0,   0),
      0x1f => (0,    0,   0),
      0x20 => (254, 254, 255),
      0x21 => (93,  179, 255),
      0x22 => (143, 161, 255),
      0x23 => (200, 144, 255),
      0x24 => (247, 133, 250),
      0x25 => (255, 131, 192),
      0x26 => (255, 139, 127),
      0x27 => (239, 154,  73),
      0x28 => (189, 172,  44),
      0x29 => (133, 188,  47),
      0x2a => (85,  199,  83),    
      0x2b => (60,  201, 140),    
      0x2c => (62,  194, 205),
      0x2d => (78,   78,  78),
      0x2e => (0,     0,   0),
      0x2f => (0,     0,   0),
      0x30 => (254, 254, 255),
      0x31 => (188, 223, 255),
      0x32 => (209, 216, 255),
      0x33 => (232, 209, 255),
      0x34 => (251, 205, 253),
      0x35 => (255, 204, 229),
      0x36 => (255, 207, 202),
      0x37 => (248, 213, 180),
      0x38 => (228, 220, 168),
      0x39 => (204, 227, 169),
      0x3a => (185, 232, 184),    
      0x3b => (174, 232, 208),    
      0x3c => (175, 229, 234),
      0x3d => (182, 182, 182),
      0x3e => (0,    0,    0),
      0x3f => (0,    0,    0),
      _ => {
          // println!("eh");
          (0, 0, 0)
      }
  }
}