#[macro_use]
extern crate lazy_static;

use std::{process::{Child, Stdio, ChildStderr}, net::{TcpStream, SocketAddr}, io::{Read, Write, BufReader, BufRead}, time::Duration};

use assert_cmd::prelude::CommandCargoExt;
use libcloud::{resources::{StrId, Resources}};
use rand::Rng;

const USER_PORT: u16 = 4444;
const COLOR_PORT: u16 = 5555;
const SIXEL_PORT: u16 = 6666;
const ASCII_PORT: u16 = 7777;

lazy_static! {
  static ref RES: Resources = Resources::load("resources.yaml");
}

fn assert_eq_str(expected: &[u8], actual: &[u8]) {
  let l = String::from_utf8(expected.into()).unwrap();
  let r = String::from_utf8(actual.into()).unwrap();
  assert_eq!(l, r)
}

struct SuicidalChild(Child);

impl std::ops::Drop for SuicidalChild {
  fn drop(&mut self) {
    self.0.kill().unwrap()
  }
}

impl SuicidalChild {
  fn premeditated_suicide(mut self) -> BufReader<ChildStderr> {
    let stderr = self.0.stderr.take().unwrap();
    std::mem::drop(self);
    BufReader::new(stderr)
  }
}

struct Client(TcpStream);

impl Client {
  fn connect() -> Result<Self, Box<dyn std::error::Error>> {
    Self::connect_port(USER_PORT)
  }

  fn connect_port(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{port}").parse::<SocketAddr>().unwrap();
    let socket = TcpStream::connect_timeout(&addr, Duration::from_millis(100))?;
    Ok(Self(socket))
  }

  fn expect_server_message(&mut self, expected: &[u8]) {
    let mut buf = vec![0; expected.len()];
    match self.0.read_exact(&mut buf) {
      Ok(()) => assert_eq_str(expected, &buf[0..expected.len()]),
      Err(e) => panic!("assert_server_message: {}\nExpected: {:?}\n\nActual: {:?}\n", 
        e, String::from_utf8(expected.to_vec()), String::from_utf8(buf))
    }
  }

  fn expect_welcome_and_rom_prompt(&mut self) {
    self.expect_server_message(&RES.fmt(StrId::Welcome, &["0"]));
    self.expect_server_message(&RES[StrId::RomSelection]);
  }

  fn expect_render_mode_prompt(&mut self) {
    self.expect_server_message(&RES[StrId::RenderModeSelection]);
  }

  fn expect_press_any_key_to_boot(&mut self, expected_mode: &str) {
    self.expect_server_message(&RES.fmt(StrId::AnyKeyToStart, &[expected_mode]));
  }

  fn expect_frame(&mut self) {
    let mut buf = [0; 3];
    match self.0.read_exact(&mut buf) {
      Ok(_) => assert_eq!("\x1b[H".as_bytes(), &buf),
      Err(e) => panic!("expect_frame: {}", e)
    }
  }

  fn expect_disconnected(&mut self) {
    let mut buf = [5u8; 1];
    if let Ok(n) = self.0.read(&mut buf) {
        if n != 0 { panic!("not disconnected") }
    }
  }

  fn disconnect(&mut self) {
    self.0.shutdown(std::net::Shutdown::Both).unwrap();
    std::thread::sleep(Duration::from_millis(500));
  }

  fn input(&mut self, data: &[u8]) {
    self.0.write_all(data).unwrap();
  }

  fn input_any_key(&mut self) {
    let any: u8 = rand::thread_rng().gen();
    self.0.write_all(&[any]).unwrap();
  }
}

fn start_app() -> Result<SuicidalChild, Box<dyn std::error::Error>> {
  start_app_with_settings(false, 5)
}

fn start_app_with_settings(block_dup: bool, max: usize) -> Result<SuicidalChild, Box<dyn std::error::Error>> {
  let mut cmd = std::process::Command::cargo_bin("nes-cloud-app")?;
  if block_dup {
    cmd.arg("--block-dup");
  }
  cmd.args([
    "--max-concurrent", &max.to_string(),
    "--client-read-timeout", "1500"
  ]);
  cmd.stderr(Stdio::piped());
  let child = cmd.spawn()?;

  // Ready?
  loop {
    let stream = TcpStream::connect("127.0.0.1:4444");
    if stream.is_ok() { break; }
  }
  
  std::thread::sleep(Duration::from_millis(500));

  Ok(SuicidalChild(child))
}

#[test]
fn single_client() -> Result<(), Box<dyn std::error::Error>>  {
  let _child = start_app()?;
  
  let mut client = Client::connect()?;
  client.expect_welcome_and_rom_prompt();

  Ok(())
}

#[test]
fn same_src_addr() -> Result<(), Box<dyn std::error::Error>>  {
  let _child = start_app_with_settings(true, 5)?;

  let mut client = Client::connect()?;
  client.expect_welcome_and_rom_prompt();

  let mut client2 = Client::connect()?;
  client2.expect_server_message(&RES[StrId::AlreadyConnected]);

  client.disconnect();

  let mut client3 = Client::connect()?;
  client3.expect_welcome_and_rom_prompt();

  Ok(())
}

#[test]
fn max_clients() -> Result<(), Box<dyn std::error::Error>>  {
  let max = 5;
  let _child = start_app_with_settings(false, max)?;

  let mut clients = vec![];
  for _ in 0..max + 1 {
    clients.push(Client::connect().unwrap());
  }

  for (i, c) in clients.iter_mut().enumerate() {
    if i < max {
      c.expect_server_message(&RES.fmt(StrId::Welcome, &[&i.to_string()]));
      c.expect_server_message(&RES[StrId::RomSelection]);
    }
    else {
      c.expect_server_message(&RES[StrId::TooManyPlayers]);
    }
  }

  let mut bailer = clients.remove(0);
  bailer.disconnect();

  let mut next = Client::connect().unwrap();
  next.expect_welcome_and_rom_prompt();

  Ok(())
}


#[test]
fn select_valid_included_rom_invalid_render_mode_selection() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect()?;
  
  client.expect_welcome_and_rom_prompt();
  client.input(&[b'1']);

  client.expect_server_message(&RES[StrId::RenderModeSelection]);
  client.input(&[0x0a]); // enter, ignore

  // Expect nothing from server, enter (0x0a) is ignored

  // Invalid input
  client.input(&[b'4']); // valid: 1-3
  // Try again
  client.expect_server_message(&RES[StrId::InvalidRenderModeSelection]);

  client.expect_disconnected();

  Ok(())
}

#[test]
fn select_valid_included_rom_valid_render_mode_selection() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect()?;
  
  client.expect_welcome_and_rom_prompt();
  client.input(&[b'1']);

  client.expect_server_message(&RES[StrId::RenderModeSelection]);
  client.input(&[0x0a]); // enter, ignore

  // Expect nothing from server, enter (0x0a) is ignored

  client.input(&[b'1']); // valid: 1-3

  client.expect_press_any_key_to_boot("Sixel");
  client.input_any_key();
  client.expect_frame();

  Ok(())
}

#[test]
fn select_valid_included_rom_invalid_render_mode_selection_icanon() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect()?;
  
  client.expect_welcome_and_rom_prompt();
  client.input(&[b'1']);

  client.expect_server_message(&RES[StrId::RenderModeSelection]);

  // Invalid input
  client.input(&[b'4']); // valid: 1-3

  client.expect_server_message(&RES[StrId::InvalidRenderModeSelection]);
  client.expect_disconnected();

  Ok(())
}

#[test]
fn select_valid_included_rom_valid_render_mode_selection_icanon() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect()?;
  
  client.expect_welcome_and_rom_prompt();
  client.input(&[b'1']);

  client.expect_server_message(&RES[StrId::RenderModeSelection]);
  client.input(&[b'1']); // valid: 1-3

  client.expect_press_any_key_to_boot("Sixel");
  client.input_any_key();
  client.expect_frame();

  Ok(())
}

#[test]
fn select_invalid_included_rom() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect()?;

  client.expect_welcome_and_rom_prompt();

  let selection = b'a';
  client.input(&[selection]);

  client.expect_server_message(&RES[StrId::InvalidRomSelection]);

  client.expect_disconnected();

  Ok(())
}

#[test]
fn pipe_valid_rom() -> Result<(), Box<dyn std::error::Error>> {
  let rom = include_bytes!("../../test-roms/nes-test-roms/cpu_dummy_writes/cpu_dummy_writes_ppumem.nes");

  let _child = start_app()?;
  let mut client = Client::connect()?;

  client.input(rom);
  
  client.expect_welcome_and_rom_prompt();
  client.expect_server_message(&RES.fmt(
    StrId::RomInserted, 
    &["[Ines] Mapper: Nrom, Mirroring: Vertical, CHR: 1x8K, PRG: 2x16K", "319a1ece57229c48663fec8bdf3764c0"]
  ));

  client.expect_render_mode_prompt();
  client.input(&[b'2']);

  client.expect_press_any_key_to_boot("Color");
  client.input_any_key();

  client.expect_frame();

  Ok(())
}

#[test]
fn pipe_valid_rom_same_frame_same_crc_detect_disconnect() -> Result<(), Box<dyn std::error::Error>> {
  // nestest has the same frame forever without any input
  let rom = include_bytes!("../../test-roms/nestest/nestest.nes");

  let child = start_app()?;
  let mut client = Client::connect()?;

  client.input(rom);
  
  client.expect_welcome_and_rom_prompt();
  client.expect_server_message(&RES.fmt(
    StrId::RomInserted, 
    &["[Ines] Mapper: Nrom, Mirroring: Horizontal, CHR: 1x8K, PRG: 1x16K", "4068f00f3db2fe783e437681fa6b419a"]
  ));
  
  client.expect_render_mode_prompt();
  client.input(&[b'3']);

  client.expect_press_any_key_to_boot("Ascii");
  client.input_any_key();

  client.expect_frame();

  client.disconnect();

  let stderr = child.premeditated_suicide();
  stderr.lines()
    .map(|l| l.unwrap())
    .find(|l| l.contains("Instance died."))
    .expect("Emulation process did not die. Probably stuck in a same CRC loop.");

  Ok(())
}

#[test]
fn pipe_invalid_rom() -> Result<(), Box<dyn std::error::Error>> {
  let rom = &[b'N', b'E', b'S', 0xff];

  let _child = start_app()?;
  let mut client = Client::connect()?;

  client.input(rom);
  
  client.expect_welcome_and_rom_prompt();
  client.expect_server_message(&RES.fmt(
    StrId::InvalidRom, 
    &["InvalidCartridge(\"magic\")"]
  ));
  client.expect_disconnected();

  Ok(())
}

#[test]
fn pipe_unsupported_rom() -> Result<(), Box<dyn std::error::Error>> {
  let rom = include_bytes!("../../test-roms/nes-test-roms/other/oam3.nes");

  let _child = start_app()?;
  let mut client = Client::connect()?;

  client.input(rom);
  
  client.expect_welcome_and_rom_prompt();
  client.expect_server_message(&RES.fmt(
    StrId::InvalidRom, 
    &["NotYetImplemented(\"Mapper 7\")"]
  ));
  client.expect_disconnected();

  Ok(())
}

#[test]
fn pipe_magic_then_timeout() -> Result<(), Box<dyn std::error::Error>> {
  let rom = &[b'N', b'E', b'S', 0x1a, 0xff];

  let _child = start_app()?;
  let mut client = Client::connect()?;

  client.input(rom);
  
  client.expect_welcome_and_rom_prompt();
  client.expect_disconnected();

  Ok(())
}

#[test]
fn render_mode_ascii() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect_port(ASCII_PORT)?;

  client.expect_welcome_and_rom_prompt();
  client.input(&[b'1']);
  client.expect_press_any_key_to_boot("Ascii");
  client.input_any_key();
  client.expect_frame();

  Ok(())
}

#[test]
fn render_mode_color() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect_port(COLOR_PORT)?;

  client.expect_welcome_and_rom_prompt();
  client.input(&[b'1']);
  client.expect_press_any_key_to_boot("Color");
  client.input_any_key();
  client.expect_frame();

  Ok(())
}

#[test]
fn render_mode_sixel() -> Result<(), Box<dyn std::error::Error>> {
  let _child = start_app()?;
  let mut client = Client::connect_port(SIXEL_PORT)?;
  
  client.expect_welcome_and_rom_prompt();
  client.input(&[b'1']);
  client.expect_press_any_key_to_boot("Sixel");
  client.input_any_key();
  client.expect_frame();

  Ok(())
}

#[test]
fn instance_panic_notify_client_and_close() -> Result<(), Box<dyn std::error::Error>> {
  let rom = include_bytes!("../../test-roms/nes-test-roms/cpu_dummy_writes/cpu_dummy_writes_ppumem.nes");

  std::env::set_var("PANIC", "1");

  let child = start_app()?;
  let mut client = Client::connect()?;

  client.input(rom);
  
  client.expect_welcome_and_rom_prompt();
  client.expect_server_message(&RES.fmt(
    StrId::RomInserted, 
    &["[Ines] Mapper: Nrom, Mirroring: Vertical, CHR: 1x8K, PRG: 2x16K", "319a1ece57229c48663fec8bdf3764c0"]
  ));

  client.expect_render_mode_prompt();
  client.input(&[b'1']);

  client.expect_press_any_key_to_boot("Sixel");
  client.input_any_key();

  client.expect_frame();

  // Instance did panic by now
  std::thread::sleep(Duration::from_millis(1200));
  std::env::remove_var("PANIC");

  // Kill app process so we can read stderr until EOF
  let stderr = child.premeditated_suicide();
  
  let exp = format!("Client disconnected: ClientId({}) (0 connected)", client.0.local_addr().unwrap());
  stderr.lines()
    .map(|l| l.unwrap())
    .find(|l| l.contains(&exp))
    .expect("Instance did not die on panic");

  Ok(())
}