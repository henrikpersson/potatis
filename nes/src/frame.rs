pub const W: usize = 256;
pub const H: usize = 240;
pub const BYTES_PER_PIXEL: usize = 4; // 0xRR0xGG0xBBxAA

pub struct RenderFrame {
  pixels: [u8; W * H * BYTES_PER_PIXEL],
}

impl RenderFrame {
  #[allow(clippy::should_implement_trait)]
  pub fn default() -> RenderFrame {
    RenderFrame { pixels: [0; W * H * BYTES_PER_PIXEL] }
  }

  pub fn set_pixel_xy(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
    self.set_pixel((y * W) + x, rgb)
  }

  pub fn set_pixel(&mut self, i: usize, rgb: (u8, u8, u8)) {
    let i = i * BYTES_PER_PIXEL;
    self.pixels[i] = rgb.0;
    self.pixels[i + 1] = rgb.1;
    self.pixels[i + 2] = rgb.2;
    self.pixels[i + 3] = 0xff;
  }

  pub fn pixels(&self) -> &[u8] {
    &self.pixels
  }

  pub fn pitch(&self) -> usize {
    W * BYTES_PER_PIXEL
  }
}

