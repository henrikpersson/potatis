use std::path::PathBuf;

use nes::{cartridge::Cartridge, nes::Nes};

pub fn setup(path: PathBuf, verbose: bool) -> Nes {
  let cartridge = Cartridge::blow_dust(path).expect("failed to map rom");
  let mut nes = Nes::insert_headless_host(cartridge);
  nes.debugger().verbose(verbose);
  nes
}