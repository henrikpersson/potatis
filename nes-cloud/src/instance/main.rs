#![feature(iter_array_chunks)]

use std::{error::Error, net::TcpStream, io::{Write, Read}, os::unix::prelude::FromRawFd, ops::Sub, path::PathBuf, fmt::{Display}, time::Duration, sync::mpsc::{Sender, Receiver}, ptr::read};
use log::{info, warn, debug, error};
use nes::{cartridge::{Cartridge, Header, HeapRom, error::CartridgeError}, nes::Nes};
use renderers::RenderMode;

use crate::{io::CloudStream, host::CloudHost};

use libcloud::{self, logging, resources::{StrId, Resources}, ServerMode, utils::{ReadByte, strhash}};

mod renderers;
mod io;
mod ansi;
mod host;

const FD_STDOUT: i32 = 1;

#[derive(Debug)]
enum RomSelection {
  Invalid(char),
  Included(PathBuf),
  Cart(Cartridge<HeapRom>, md5::Digest)
}

#[derive(Debug)]
struct InstanceError<S : AsRef<str>>(S);

impl<S : AsRef<str>> Display for InstanceError<S> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0.as_ref())
  }
}

impl<S : AsRef<str> + std::fmt::Debug> std::error::Error for InstanceError<S> {}

fn pipe_or_select_rom(r: &mut impl Read, res: &Resources) -> Result<RomSelection, Box<dyn Error>> {
  let byte = r.read_byte()?;
  if byte == nes::cartridge::MAGIC[0] {
    return read_rom(r);
  }

  // Covert b'1' > ascii 49 > 1u32
  if let Some(selection) = (byte as char).to_digit(10) {
    let roms_included = res.included_roms();
    if selection > 0 && selection <= roms_included.len() as u32 {
      let path: PathBuf = roms_included.get(selection.sub(1) as usize).unwrap().to_path_buf();
      return Ok(RomSelection::Included(path))
    }
  }

  Ok(RomSelection::Invalid(byte as char))
}

fn read_rom(r: &mut impl Read) -> Result<RomSelection, Box<dyn Error>> {
  let mut rest_of_magic = [0u8; 3];
  r.read_exact(&mut rest_of_magic)?;

  if rest_of_magic != nes::cartridge::MAGIC[1..] {
    return Err(Box::new(CartridgeError::InvalidCartridge("magic")))
  }

  let mut header_buf = [0u8; 16];
  r.read_exact(&mut header_buf)?;

  let mut full_header = nes::cartridge::MAGIC.to_vec();
  full_header.append(&mut header_buf.to_vec());
  let header = Header::parse(&full_header)?;

  let mut rom_buf = vec![0; header.total_size_excluding_header() - 4];
  r.read_exact(&mut rom_buf)?;

  let mut full_cart = full_header;
  full_cart.append(&mut rom_buf);

  let hash = md5::compute(&full_cart);
  let cart = Cartridge::blow_dust_vec(full_cart)?;
  Ok(RomSelection::Cart(cart, hash))
}

fn select_render_mode(stream: &mut impl Read) -> Result<RenderMode, Box<dyn Error>> {
  fn prompt(stream: &mut impl Read, first: bool) -> Result<RenderMode, Box<dyn Error>> {
    let input = stream.read_byte()?;
    match input {
      b'1' => Ok(RenderMode::Sixel),
      b'2' => Ok(RenderMode::Color),
      b'3' => Ok(RenderMode::Ascii),
      0x0a if first => prompt(stream, false),
      _ => return Err(Box::new(InstanceError(format!("Invalid render selection: {:#04x}", input))))
    }
  }

  prompt(stream, true)
}

fn recv_thread(mut stream: CloudStream, tx: Sender<u8>) {
  info!("Starting recv thread.");

  let mut buf = [0u8; 1];
  while stream.read_exact(&mut buf).is_ok() {
    debug!("got input: {} ({:#04x})", buf[0] as char, buf[0]);
    tx.send(buf[0]).unwrap();
  }

  warn!("Recv thread died")
}

fn emulation_thread(
  stream: CloudStream, 
  rx: Receiver<u8>, 
  cart: Cartridge<HeapRom>, 
  mode: RenderMode,
  res: &Resources,
) {
  let fps = match mode {
    RenderMode::Color => res.fps_conf().color,
    RenderMode::Ascii => res.fps_conf().ascii,
    RenderMode::Sixel => res.fps_conf().sixel,
  };

  info!("Starting emulation. FPS: {}, limit: {}", fps, res.tx_mb_limit());

  let host = CloudHost::new(stream, rx, mode, res.tx_mb_limit());
  let mut nes = Nes::insert(cart, host);
  nes.fps_max(fps);

  while nes.powered_on() {
    nes.tick();
  }

  warn!("NES powered off")
}

fn main() -> Result<(), Box<dyn Error>> {
  logging::init(std::env::var("LOG_TO_FILE")
    .map_or(false, |s| s.parse().unwrap_or(false)))?;

  let oghook = std::panic::take_hook();
  std::panic::set_hook(Box::new(move |info| {
    oghook(info);
    error!("EMU INSTANCE PANIC: {}", info);
    std::process::exit(1);
  }));

  let fd = std::env::var("FD");
  let srv_mode: ServerMode = std::env::var("MODE")
    .map_or(ServerMode::User, |s| s.parse().unwrap_or(ServerMode::User));
  info!("Instance started. FD: {:?}, Mode: {:?}", fd, srv_mode);

  let mut res = Resources::load("resources.yaml");

  let mut stream: CloudStream = match fd?.parse() {
    Ok(FD_STDOUT) => CloudStream::Offline,
    Ok(socketfd) => unsafe { CloudStream::Online(TcpStream::from_raw_fd(socketfd)) },
    Err(e) => panic!("invalid FD: {}", e)
  };
  
  // Say hello
  let players = std::env::var("PLAYERS").unwrap_or_else(|_| "0".into());
  stream.write_all(&res.fmt(StrId::Welcome, &[&players]))?;

  info!("Asking for ROM selection");
  stream.write_all(&res[StrId::RomSelection])?;
  let response = pipe_or_select_rom(&mut stream, &res);
  info!("ROM selection: {:?}", response);

  let cart = match response {
    Ok(RomSelection::Included(path)) => {
      // let mut res = res;
      let rom = res.load_rom(&path);
      match Cartridge::blow_dust_vec(rom) {
        Ok(cart) => cart,
        Err(e) => panic!("Failed to load included ROM: {}", e),
      }
    },
    Ok(RomSelection::Cart(cart, hash)) => {
      stream.write_all(&res.fmt(
        StrId::RomInserted, 
        &[&cart.to_string(), &strhash(&hash)]
      )).unwrap();
      cart
    }
    Ok(RomSelection::Invalid(_)) => {
      stream.write_all(&res[StrId::InvalidRomSelection]).unwrap();
      return Err(Box::new(InstanceError("Invalid ROM selection.")));
    },
    Err(e) if e.is::<CartridgeError>() => {
      stream.write_all(&res.fmt(
        StrId::InvalidRom, 
        &[&e.to_string()])
      ).unwrap();
      return Err(e);
    }
    Err(e) => {
      error!("IO problem on initial read. Connection lost?");
      return Err(e);
    }
  };

  let mode = match srv_mode {
    ServerMode::Color => RenderMode::Color,
    ServerMode::Ascii => RenderMode::Ascii,
    ServerMode::Sixel => RenderMode::Sixel,
    ServerMode::User => {
      info!("Asking for render mode selection");
      stream.write_all(&res[StrId::RenderModeSelection])?;
      let selection = select_render_mode(&mut stream);
      let Ok(mode) = selection else {
        stream.write_all(&res[StrId::InvalidRenderModeSelection])?;
        panic!("Invalid render mode selection: {:?}", selection);
      };
      info!("Render mode select: {:?}", mode);
      mode
    }
  };

  stream.write_all(&res.fmt(StrId::AnyKeyToStart, &[&format!("{:?}", mode)]))?;
  if stream.read_byte()? == 0x0a {
    // Read again for non-icanon ppl (0x0a from last input)
    stream.read_byte()?;
  }

  let (tx, rx) = std::sync::mpsc::channel::<u8>();
  std::thread::scope(|scope| {
    let s = stream.clone();
    scope.spawn(|| { recv_thread(s, tx) });
    scope.spawn(|| { emulation_thread(stream, rx, cart, mode, &res) });

    if std::env::var("PANIC").is_ok() {
      std::thread::sleep(Duration::from_millis(1000));
      panic!("intentional")
    }
  });

  info!("Instance died.");

  Ok(())
}


#[cfg(test)]
mod tests {
  use std::{io::Cursor, error::Error};
  use libcloud::{resources::Resources, utils::strhash};

  use crate::{RomSelection, pipe_or_select_rom};

  impl PartialEq for RomSelection {
    fn eq(&self, other: &Self) -> bool {
      match (self, other) {
        (Self::Invalid(l0), Self::Invalid(r0)) => l0 == r0,
        (Self::Included(l0), Self::Included(r0)) => l0 == r0,
        (Self::Cart(_, _), Self::Cart(_, _)) => true,
        _ => false,
      }
    }
  }

  fn input(b: &[u8]) -> Result<RomSelection, Box<dyn Error>> {
    let mut input = Cursor::new(b);
    pipe_or_select_rom(&mut input, &Resources::load("resources.yaml"))
  }

  #[test]
  fn test_select_rom() {
    assert_eq!(input(&[b'0']).unwrap(), RomSelection::Invalid('0'));
    matches!(input(&[b'1']).unwrap(), RomSelection::Included(_));
    assert_eq!(input(&[b'6']).unwrap(), RomSelection::Invalid('6'));
    matches!(input(&[b'1', b'3']).unwrap(), RomSelection::Included(_));
    assert_eq!(input(&[b'a', b'b']).unwrap(), RomSelection::Invalid('a'));
  }

  #[test]
  fn test_pipe_invalid_rom() {
    assert!(input(&[b'N']).is_err());
    assert!(input(&[b'N', b'E']).is_err());
    assert!(input(&[b'N', b'E', b'S']).is_err());
    assert!(input(&[b'N', b'E', b'S', 0x1a]).is_err());
    assert!(input(&[b'N', b'E', b'S', 0x1a, 0x00]).is_err());
  }

  #[test]
  fn test_pipe_unsupported_rom() {
    let mapper7 = include_bytes!("../../../test-roms/nes-test-roms/other/oam3.nes");
    let result = input(mapper7);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap().to_string(), "NotYetImplemented(\"Mapper 7\")")
  }

  #[test]
  fn test_pipe_valid_roms() {
    fn assert_pipe_rom(rom: &[u8], expected: &'static str, exphash: &'static str) {
      match input(rom) {
        Ok(RomSelection::Cart(cart, hash)) => {
          assert_eq!(cart.to_string(), expected);
          assert_eq!(exphash, strhash(&hash));
        },
        Ok(_) => panic!("invalid response") ,
        Err(e) => panic!("{}", e),
      }
    }

    let pm = include_bytes!("../../../test-roms/nestest/nestest.nes");
    assert_pipe_rom(pm, "[Ines] Mapper: Nrom, Mirroring: Horizontal, CHR: 1x8K, PRG: 1x16K", "4068f00f3db2fe783e437681fa6b419a");
    
    let pm = include_bytes!("../../../test-roms/nes-test-roms/instr_misc/instr_misc.nes");
    assert_pipe_rom(pm, "[Ines] Mapper: Mmc1, Mirroring: Vertical, CHR RAM: 1x8K, PRG: 4x16K", "df401ddc57943c774a225e9fb9b305a0");
  }
}