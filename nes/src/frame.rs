pub const W: usize = 256;
pub const H: usize = 240;
const BYTES_PER_PIXEL: usize = 3; // 0xRR0xGG0xBB

pub struct RenderFrame {
  pixels: [u8; W * H * BYTES_PER_PIXEL],
}

impl RenderFrame {
  pub fn new() -> RenderFrame {
    RenderFrame { pixels: [0; W * H * BYTES_PER_PIXEL] }
  }

  pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
    let row = y * W * BYTES_PER_PIXEL;
    let col = x * BYTES_PER_PIXEL; 
    let i = row + col;

    if i + 3 < self.pixels.len() {
      // let pixel = &mut self.pixels[start..start+3];
      // pixel.copy_from_slice(&[rgb.0, rgb.1, rgb.2]);
      self.pixels[i] = rgb.0;
      self.pixels[i + 1] = rgb.1;
      self.pixels[i + 2] = rgb.2;
    }
  }

  pub fn pixels(&self) -> &[u8] {
    &self.pixels[..]
  }

  pub fn pitch(&self) -> usize {
    W * BYTES_PER_PIXEL
  }
}

