use alloc::vec::Vec;

type SetPixelFn = fn(&mut [u8], i: usize, rgb: (u8, u8, u8));

pub trait SetPixel {
  fn set_pixel(buf: &mut [u8], i: usize, rgb: (u8, u8, u8));
}

pub trait PixelFormat {
  const BYTES_PER_PIXEL: usize;
}

pub struct PixelFormatRGB888;

impl PixelFormat for PixelFormatRGB888 {
  const BYTES_PER_PIXEL: usize = 3;
}

impl SetPixel for PixelFormatRGB888 {
  fn set_pixel(buf: &mut [u8], i: usize, rgb: (u8, u8, u8)) {
    buf[i] = rgb.0;
    buf[i + 1] = rgb.1;
    buf[i + 2] = rgb.2;
  }
}

pub struct PixelFormatRGB565;

impl PixelFormat for PixelFormatRGB565 {
  const BYTES_PER_PIXEL: usize = 2;
}

impl SetPixel for PixelFormatRGB565 {
  fn set_pixel(buf: &mut [u8], i: usize, rgb: (u8, u8, u8)) {
    // RGB888 to 565
    let b = (rgb.2 as u16 >> 3) & 0x1f;
    let g = ((rgb.1 as u16 >> 2) & 0x3f) << 5;
    let r = ((rgb.0 as u16 >> 3) & 0x1f) << 11;

    // u16 to 2xu8
    let p = r | g | b;
    buf[i] = (p >> 8) as u8;
    buf[i + 1] = (p & 0xff) as u8;
  }
}

pub trait DisplayRegion {  
  const WIDTH: usize;
  const HEIGHT: usize;
  const OVERSCAN_PIXELS: usize;
}

pub struct DisplayRegionNTSC;

impl DisplayRegion for DisplayRegionNTSC {
  const WIDTH: usize = 240;
  const HEIGHT: usize = 224;
  const OVERSCAN_PIXELS: usize = 8;
}

pub struct DisplayRegionPAL;

impl DisplayRegion for DisplayRegionPAL {
  const WIDTH: usize = 256;
  const HEIGHT: usize = 240;
  const OVERSCAN_PIXELS: usize = 0;
}

pub struct RenderFrame {
  width: usize,
  bytes_per_pixel: usize,
  overscan_pixels: usize,
  buf: Vec<u8>,
  set_pixel_fn: SetPixelFn 
}

impl RenderFrame {
  pub fn new<DISPLAY, FORMAT>() -> Self
    where
      DISPLAY : DisplayRegion,
      FORMAT : PixelFormat + SetPixel + 'static
  {
    Self {
      width: DISPLAY::WIDTH,
      bytes_per_pixel: FORMAT::BYTES_PER_PIXEL,
      overscan_pixels: DISPLAY::OVERSCAN_PIXELS,
      buf: vec![0; DISPLAY::WIDTH * DISPLAY::HEIGHT * FORMAT::BYTES_PER_PIXEL],
      set_pixel_fn: FORMAT::set_pixel
    }
  }

  fn overscan(&self, x: &mut usize, y: &mut usize) {
    // 0 == no draw
    // width - 8 == no draw
    // 8 == 0
    if *x >= self.overscan_pixels { 
      *x -= self.overscan_pixels;
    }

    if *y >= self.overscan_pixels {
      *y -= self.overscan_pixels;
    }
  }

  pub fn set_pixel_xy(&mut self, mut x: usize, mut y: usize, rgb: (u8, u8, u8)) {
    self.overscan(&mut x, &mut y);

    let i = ((y * self.width) + x) * self.bytes_per_pixel;
    if i < self.buf.len() {
      (self.set_pixel_fn)(&mut self.buf, i, rgb);
    }
  }

  pub fn pixels(&self) -> &[u8] {
    &self.buf[..]
  }

  pub fn pitch(&self) -> usize {
    self.width * self.bytes_per_pixel
  }
}

