use std::{fs, path::PathBuf, collections::HashMap, fmt::Debug};
use log::debug;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum StrId {
  Welcome,
  AlreadyConnected,
  TooManyPlayers,
  RomSelection,
  InvalidRomSelection,
  InvalidRom,
  RomInserted,
  RenderModeSelection,
  InvalidRenderModeSelection,
  AnyKeyToStart,
}

#[derive(Debug, Deserialize)]
pub struct Fps {
  pub sixel: usize,
  pub color: usize,
  pub ascii: usize,
}

#[derive(Debug, Deserialize)]
pub struct Resources {
  included_roms: Vec<PathBuf>,
  fps: Fps,
  tx_mb_limit: usize,
  strings: HashMap<StrId, String>,
}

impl Resources {
  pub fn load(filepath: &str) -> Resources {
    let f = match fs::File::open(filepath) {
      Ok(f) => f,
      Err(e) => panic!("could not open resource file ({}): {}", filepath, e)
    };

    let res: Resources = serde_yaml::from_reader(f).unwrap();
    for p in &res.included_roms {
      if !p.exists() {
        panic!("Included ROM not found: {:?}", p)
      }
      debug!("Included ROM: {:?}", p);
    }

    res
  }

  pub fn fmt(&self, index: StrId, args: &[&str]) -> Vec<u8> {
    let mut fstr = String::from_utf8(self[index].to_vec()).unwrap();
    if !fstr.contains("{}") {
      panic!("fmtwhat: {:?}", index);
    }
    for arg in args {
      fstr = fstr.replacen("{}", arg, 1)
    }
    fstr.as_bytes().to_vec()
  }

  pub fn included_roms(&self) -> &Vec<PathBuf> {
    &self.included_roms
  }

  pub fn load_rom(&mut self, path: &PathBuf) -> Vec<u8> {
    debug!("Loading included ROM: {:?}", path);
    std::fs::read(path).expect("failed to load included ROM")
  }

  pub fn fps_conf(&self) -> &Fps {
    &self.fps
  }

  pub fn tx_mb_limit(&self) -> usize {
    self.tx_mb_limit
  }
}

impl std::ops::Index<StrId> for Resources {
  type Output = [u8];

  fn index(&self, index: StrId) -> &[u8] {
    self.strings.get(&index).unwrap().as_bytes()
  }
}

#[cfg(test)]
mod tests {
  use super::{Resources, StrId};

  #[test]
  fn res_fmt() {
    let r = Resources::load("resources.yaml");
    assert_eq!(
      "\nYou have inserted a ROM:\nfoo\nbar\n", 
      String::from_utf8(r.fmt(StrId::RomInserted, &["foo", "bar"])).unwrap()
    );
  }  
}