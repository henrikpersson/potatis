use crate::renderers::Rgb;

pub const CURSOR_HOME: &str = "\x1b[H";
pub const CURSOR_HOME_BYTES: &[u8] = CURSOR_HOME.as_bytes();
// pub const CLEAR: &str = "\x1b[2J";

pub(crate) struct Ansi<'a>(&'a str);

#[allow(dead_code)]
impl Ansi<'_> {
  pub fn open_fg(rgb: Rgb) -> String {
    let index = ansi_colours::ansi256_from_rgb(rgb);
    format!("\x1b[38;5;{}m", index)
  }

  pub fn open_bg(bg: Rgb) -> String {
    let index = ansi_colours::ansi256_from_rgb(bg);
    format!("\x1b[48;5;{}m", index)
  }

  pub fn fg(self, rgb: Rgb) -> String {
    let index = ansi_colours::ansi256_from_rgb(rgb);
    format!("\x1b[38;5;{}m{}", index, self.0)
  }

  pub fn bg(self, rgb: Rgb) -> String {
    let index = ansi_colours::ansi256_from_rgb(rgb);
    format!("\x1b[48;5;{}m{}", index, self.0)
  }

  pub fn fg_bg(self, fg: Rgb, bg: Rgb) -> String {
    let fgi = ansi_colours::ansi256_from_rgb(fg);
    let bgi = ansi_colours::ansi256_from_rgb(bg);
    format!("\x1b[38;5;{}m\x1b[48;5;{}m{}", fgi, bgi, self.0)
  }
}