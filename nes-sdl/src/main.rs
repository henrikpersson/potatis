use std::{path::PathBuf};
use nes::{cartridge::Cartridge, nes::Nes, mos6502::debugger::Breakpoint};
use structopt::StructOpt;
use common::utils;

mod sdl;
use crate::sdl::SdlHostPlatform;

#[derive(StructOpt, Debug)]
struct Cli {
  path: PathBuf,
  #[structopt(short, long, parse(try_from_str = utils::parse_hex))]
  breakpoint: Option<u16>,
  #[structopt(short, long)]
  opcode_breakpoint: Option<String>,
  #[structopt(short, long)]
  verbose: bool,
  #[structopt(short, long)]
  debug: bool
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Cli = Cli::from_args();
  println!("Loading {:?}.", args.path);

  let cartridge = Cartridge::blow_dust(args.path)?;
  println!("Loaded! {}", cartridge);

  let mut nes = Nes::insert(cartridge, SdlHostPlatform::new());
  nes.show_fps(std::env::var("SHOW_FPS").is_ok());

  let debugger = nes.debugger();
  debugger.verbose(args.verbose);

  if let Some(bp) = args.breakpoint {
    debugger.add_breakpoint(Breakpoint::Address(bp));
  }

  if let Some(opbp) = args.opcode_breakpoint {
    debugger.add_breakpoint(Breakpoint::Opcode(opbp));
  }

  if args.debug {
    debugger.enable();
  }

  while nes.powered_on() {
    nes.tick();
  }

  Ok(())
}