use crate::frame;

pub const NTSC_W: usize = 240;
pub const NTSC_H: usize = 224;

const NTSC_OVERSCAN: usize = 8;
const NTSC_OVERSCAN_BYTES: usize = NTSC_OVERSCAN * frame::BYTES_PER_PIXEL;
const NTSC_OVERSCAN_PITCH: usize = NTSC_W * frame::BYTES_PER_PIXEL;

pub struct Ntsc {
  pub pixels: Vec<u8>,
  pub pitch: usize
}

pub fn ntsc(pixels: &[u8]) -> Ntsc {
  let top = NTSC_OVERSCAN_BYTES * frame::W;
  let bottom = pixels.len() - (NTSC_OVERSCAN_BYTES * frame::W);

  let rows: Vec<&[u8]> = pixels[top..bottom].chunks_exact(frame::W * frame::BYTES_PER_PIXEL).collect();
  let mut overscan_pixels = Vec::with_capacity(NTSC_W * NTSC_H * frame::BYTES_PER_PIXEL);

  for col in rows {
    let left = NTSC_OVERSCAN_BYTES;
    let right = col.len() - NTSC_OVERSCAN_BYTES;
    overscan_pixels.append(&mut col[left..right].to_vec());
  }

  Ntsc { pixels: overscan_pixels, pitch: NTSC_OVERSCAN_PITCH }
}