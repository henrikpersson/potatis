use nes::nes::Nes;

mod common;

const STATUS_RUNNING: u8 = 0x80;
const STATUS_NEEDS_RESET: u8 = 0x81;
const STATUS_SUCCESS: u8 = 0x00;
const VALID_MAGIC: [u8; 3] = [0xde, 0xb0, 0x61];

#[test]
fn instr_test_v5_official_mmc1() {
  run_blargg_test("instr_test-v5/official_only.nes", "All 16 tests passed", STATUS_SUCCESS)
}

#[test]
fn instr_test_v5_basic_nrom() {
  run_blargg_test("instr_test-v5/rom_singles/01-basics.nes", "01-basics\n\nPassed", STATUS_SUCCESS);
}

#[test]
fn instr_misc() {
  // The last steps tests dummy reads on PPU. Not yet implemented.
  let success = "Test requires $2002 mirroring every 8 bytes to $3FFA\n\n03-dummy_reads\n\nFailed #2\n\nWhile running test";
  run_blargg_test("instr_misc/instr_misc.nes", success, 1);
}

#[test]
fn ppu_vbl_nmi() {
  let success = "VBL period is too long with BG off\n\n01-vbl_basics\n\nFailed #8";
  // let success = "$2002 should be mirrored at $200A\n\n01-vbl_basics\n\nFailed #5";
  run_blargg_test("ppu_vbl_nmi/rom_singles/01-vbl_basics.nes", success, 0x08);
}

#[test]
fn ppu_open_bus() {
  let success = "Bits 2-4 of sprite attributes should always be clear when read\n\nppu_open_bus\n\nFailed #10";
  run_blargg_test("ppu_open_bus/ppu_open_bus.nes", success, 0x0a);
}

#[test]
#[ignore = "impl ppu, mapper 3"]
fn ppu_read_buffer() {
  run_blargg_test("ppu_read_buffer/test_ppu_read_buffer.nes", "dunno", STATUS_SUCCESS);
}

fn run_blargg_test(test: &str, success_string: &str, success_status: u8) {
  let path = format!("../test-roms/nes-test-roms/{}", test);
  let mut nes = common::setup(path.into(), std::env::var("VERBOSE").is_ok());

  let result: String;
  let mut status: Option<u8> = None;

  // nes.debugger().enable();
  nes.debugger().watch_memory_range(0x6004..=0x6004+100, |mem| {
    println!("{}", read_null_terminated_string(&mem));
  });

  loop {
    nes.tick();

    if check_and_update_status(&nes, &mut status) {
      match status {
        Some(STATUS_RUNNING) => (),
        Some(STATUS_NEEDS_RESET) => panic!("needs reset.."),
        Some(0x00..=0x7F) => { // Completed, status is the result code.
          let mem_view = nes.bus().read_range(0x6004..=0x6004+100);
          result = read_null_terminated_string(&mem_view);
          break
        },
        _ => panic!("unknown status")
      }
    }
  }

  println!("status code: {:#04x}", status.unwrap());
  assert_eq!(success_string, result.trim());
  assert_eq!(Some(success_status), status);
}

fn read_null_terminated_string(range: &[u8]) -> String {
  let string: Vec<u8> = range.iter().take_while(|&b| *b != 0x00).cloned().collect();
  String::from_utf8(string).unwrap()
}

fn check_and_update_status(nes: &Nes, current_status: &mut Option<u8>) -> bool {
  let mem = nes.cpu().bus();
  if mem.read_range(0x6001..=0x6003) == VALID_MAGIC {
    let new_status = mem.read8(0x6000);
    if Some(new_status) != *current_status {
      *current_status = Some(new_status);
      return true;
    }
  }
  false
}