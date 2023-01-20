use std::{fmt::Display, str::FromStr};

pub mod resources;
pub mod logging;

#[derive(Debug, Clone, Copy)]
pub enum ServerMode {
  Color,
  Ascii,
  Sixel,
  User,
}

impl Display for ServerMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl FromStr for ServerMode {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "Ascii" => Ok(Self::Ascii),
      "Color" => Ok(Self::Color),
      "Sixel" => Ok(Self::Sixel),
      "User" => Ok(Self::User),
      _ => Err(())
    }
  }
}

pub mod utils {
  use std::io::Read;

  pub trait ReadByte {
    fn read_byte(&mut self) -> Result<u8, std::io::Error>;
  }

  impl<R> ReadByte for R where R : Read {
    fn read_byte(&mut self) -> Result<u8, std::io::Error> {
      let mut buf = [0; 1];
      self.read_exact(&mut buf)?;
      Ok(buf[0])
    }
  }

  pub fn strhash(digest: &md5::Digest) -> String {
    format!("{:x}", digest)
  }
}