
pub mod kilobytes {
  pub const KB1: usize = 1024;
  pub const KB2: usize = 2048;
  pub const KB4: usize = 4096;
  pub const KB8: usize = 8192;
  pub const KB16: usize = 16384;
  pub const KB32: usize = 32768;
}

pub mod utils {
  pub fn parse_hex(src: &str) -> Result<u16, std::num::ParseIntError> {
    u16::from_str_radix(src, 16)
  }
}

#[allow(dead_code)]
pub mod bits {
  pub const BIT0: usize = 1;
  pub const BIT1: usize = 1 << 1;
  pub const BIT2: usize = 1 << 2;
  pub const BIT3: usize = 1 << 3;
  pub const BIT4: usize = 1 << 4;
  pub const BIT5: usize = 1 << 5;
  pub const BIT6: usize = 1 << 6;
  pub const BIT7: usize = 1 << 7;

  pub fn is_signed(n: u8) -> bool {
    n & (1 << 7) != 0
  }
  
  pub fn is_overflow(res: u8, lhs: u8, rhs: u8) -> bool {
    if is_signed(lhs) && is_signed(rhs) && !is_signed(res) {
      true
    }
    else { 
      !is_signed(lhs) && !is_signed(rhs) && is_signed(res) 
    }
  }
}