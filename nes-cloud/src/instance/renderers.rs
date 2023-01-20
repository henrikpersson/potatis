use std::{fs::File, io::{Read, BufWriter}};
use nes::frame::{RenderFrame, PixelFormat, PixelFormatRGB888};

use crate::ansi::{Ansi, self};

const UPPER_BLOCK: &str = "\u{2580}";

#[derive(Debug, Clone, Copy)]
pub enum RenderMode { 
  Color,
  Ascii,
  Sixel,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub(crate) struct Rgb(u8, u8, u8);

impl ansi_colours::AsRGB for Rgb {
  fn as_u32(&self) -> u32 {
    let mut i = (self.0 as u32) << 16;
    i |= (self.1 as u32) << 8;
    i |= self.2 as u32;
    i
  }
}

pub trait Renderer {
  fn render(&mut self, frame: &RenderFrame) -> Vec<u8>;
  // fn tx_speed(&self) -> usize;
}

pub fn create(mode: RenderMode) -> Box<dyn Renderer> {
  match mode {
    RenderMode::Color => Box::new(UnicodeColorRenderer::new()),
    RenderMode::Ascii => Box::new(AsciiRenderer::new()),
    RenderMode::Sixel => Box::new(SixelRenderer::new()),
  }
}

struct SixelRenderer {
  sixel: sixel_rs::encoder::Encoder,
  buf: File,
}

impl SixelRenderer {
  pub fn new() -> Self {
    let outfile = tempfile::Builder::new()
      .prefix("sixel")
      .tempfile()
      .unwrap();

    let sixel = sixel_rs::encoder::Encoder::new().unwrap();
    sixel.set_quality(sixel_rs::optflags::Quality::Low).unwrap();
    sixel.set_output(outfile.path()).unwrap();
    sixel.set_height(sixel_rs::optflags::SizeSpecification::Percent(300)).unwrap();
    sixel.set_width(sixel_rs::optflags::SizeSpecification::Percent(300)).unwrap();

    Self {
      sixel,
      buf: outfile.into_file(),
    }
  }
}

impl Renderer for SixelRenderer {
  fn render(&mut self, frame: &RenderFrame) -> Vec<u8> {
    self.buf.set_len(0).unwrap();

    // TODO: Avoid created a new file here. Reuse old tmp.
    let infile = tempfile::Builder::new()
      .prefix("frame")
      .tempfile()
      .unwrap();
    let inpath = infile.path().to_owned();

    let w = &mut BufWriter::new(infile);
    let mut png = png::Encoder::new(
      w, 
      nes::frame::NTSC_WIDTH as u32, 
      nes::frame::NTSC_HEIGHT as u32
    );
    png.set_color(png::ColorType::Rgb);
    png.set_depth(png::BitDepth::Eight);
    let mut writer = png.write_header().unwrap();
    let pixels: Vec<u8> = frame.pixels_ntsc().collect();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    
    self.sixel.encode_file(&inpath).unwrap();

    let mut buf = ansi::CURSOR_HOME_BYTES.to_vec();
    self.buf.read_to_end(&mut buf).unwrap();
    buf
  }
}

struct UnicodeColorRenderer {
  buf: String
}

impl UnicodeColorRenderer {
  const COLS: usize = nes::frame::NTSC_WIDTH;
  const ROWS: usize = nes::frame::NTSC_HEIGHT;

  fn new() -> Self {
    UnicodeColorRenderer { buf: String::with_capacity(160000) }
  }
}

impl Renderer for UnicodeColorRenderer {
  fn render(&mut self, frame: &RenderFrame) -> Vec<u8> {
    self.buf.clear();
    self.buf.push_str(crate::ansi::CURSOR_HOME);

    let p: Vec<u8> = frame.pixels_ntsc().collect();
    let mut c_upper: Option<Rgb> = None;
    let mut c_lower: Option<Rgb> = None;
    for row in (0..Self::ROWS).step_by(2) {
      for col in 0..Self::COLS {
        let upper_i = ((row * Self::COLS) + col) * PixelFormatRGB888::BYTES_PER_PIXEL;
        let upper = Rgb(p[upper_i], p[upper_i + 1], p[upper_i + 2]);

        let lower_i = (((row + 1) * Self::COLS) + col) * PixelFormatRGB888::BYTES_PER_PIXEL;
        let lower = Rgb(p[lower_i], p[lower_i + 1], p[lower_i + 2]);

        if Some(upper) != c_upper {
          self.buf.push_str(&Ansi::open_fg(upper));
          c_upper = Some(upper);
        }

        if Some(lower) != c_lower {
          self.buf.push_str(&Ansi::open_bg(lower));
          c_lower = Some(lower);
        }

        self.buf.push_str(UPPER_BLOCK);
      }
      
      self.buf.push('\n')
    }

    self.buf.as_bytes().to_vec()
  }
}

struct AsciiRenderer {
  buf: String
}

impl AsciiRenderer {
  const CHARSET: &str = " .-`',:_;~\"/!|\\i^trc*v?s()+lj1=e{[]z}<xo7f>aJy3Iun542b6Lw9k#dghq80VpT$YACSFPUZ%mEGXNO&DKBR@HQWM";
  const MAX: f64 = Self::CHARSET.len() as f64;

  fn new() -> Self {
    Self { buf: String::with_capacity(50000) }
  }
}

impl Renderer for AsciiRenderer {
  fn render(&mut self, frame: &RenderFrame) -> Vec<u8> {
    self.buf.clear();
    self.buf.push_str(crate::ansi::CURSOR_HOME);

    frame.pixels_ntsc()
      .array_chunks::<{nes::frame::PixelFormatRGB888::BYTES_PER_PIXEL}>()
      .enumerate()
      .for_each(|(n, p)| {
        // https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color
        let g: f64 = ((0.2126 * p[0] as f64) + (0.7152 * p[1] as f64) + (0.0722 * p[2] as f64)) / 255.0;
        let i = ((Self::MAX * g) + 0.5).floor();
        let c = Self::CHARSET.chars().nth(i as usize).unwrap_or('.');
        self.buf.push(c);

        if n % nes::frame::NTSC_WIDTH == 0 {
          self.buf.push('\n')
        }
      });

    self.buf.as_bytes().to_vec()
  }
}

#[cfg(test)]
mod tests {
  use nes::frame::{RenderFrame, PixelFormatRGB888};

  use crate::renderers::{UnicodeColorRenderer, SixelRenderer};
  use super::{AsciiRenderer, Renderer};

  #[test]
  fn frame_sizes() {
    let buf888 = include_bytes!("../../tests/frame_888_pal.bin");
    let mut frame888 = RenderFrame::new::<PixelFormatRGB888>();
    frame888.replace_buf(buf888);

    let sixel888 = SixelRenderer::new().render(&frame888).len();
    let color = UnicodeColorRenderer::new().render(&frame888).len();
    let ascii = AsciiRenderer::new().render(&frame888).len();

    assert!(8_000 <= sixel888, "sixel 888 too big: {sixel888}kb"); // 0.24mb/s at 30 fps
    assert!(153_000 <= color, "color too big: {color}kb"); // 1.5mb/s at 10
    assert!(40_000 <= ascii, "ascii too big: {ascii}kb"); // 0.8mb/s at 20

    // let buf565 = include_bytes!("../../tests/frame_565_pal.bin");
    // let mut frame565 = RenderFrame::new::<PixelFormatRGB565>();
    // frame565.replace_buf(buf565);

    // let sixel565 = SixelRenderer::with_format(sixel_rs::sys::PixelFormat::RGB565)
    //   .render(&frame565).len() / 1000;

    // assert!(5 <= sixel565, "sixel 565 too big: {sixel565}kb");
  }
}