use nes::{cartridge::Cartridge, nes::{Nes, HostSystem, Shutdown}, joypad::{JoypadButton, JoypadEvent}, display};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, Default)]
pub enum KeyState { Pressed, Released, #[default] None }

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardState([KeyState; 8]);

#[wasm_bindgen]
extern {
  pub type BrowserNes;

  #[wasm_bindgen(js_namespace = console)]
  fn log(s: &str);

  #[wasm_bindgen(method)]
  pub fn on_frame_ready(this: &BrowserNes, ptr: *const u8, len: usize);

  #[wasm_bindgen(method)]
  pub fn poll_keyboard(this: &BrowserNes, ptr: *mut u8);

  #[wasm_bindgen(method)]
  pub fn delay(this: &BrowserNes, millis: usize);
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
  console_error_panic_hook::set_once();
  log("rust init'd");
  Ok(())
}

struct WasmHostSystem {
  browser: BrowserNes,
  keyboard: KeyboardState,
  time: wasm_timer::Instant
}

impl HostSystem for WasmHostSystem {
  fn render(&mut self, frame: &nes::frame::RenderFrame) {
    let ntsc = display::ntsc(frame.pixels());
    let happy_js: Vec<u8> = ntsc.pixels.chunks_exact(3)
      .flat_map(|p| [p[0], p[1], p[2], 0xff])
      .collect();
    self.browser.on_frame_ready(happy_js.as_ptr(), happy_js.len());
  }

  fn poll_events(&mut self, joypad: &mut nes::joypad::Joypad) -> Shutdown {
    self.browser.poll_keyboard(self.keyboard.0.as_mut_ptr() as *mut u8);

    for (i, k) in self.keyboard.0.iter().enumerate() {
      let button = match i {
        0 => JoypadButton::RIGHT,
        1 => JoypadButton::LEFT,
        2 => JoypadButton::DOWN,
        3 => JoypadButton::UP,
        4 => JoypadButton::START,
        5 => JoypadButton::SELECT,
        6 => JoypadButton::B,
        7 => JoypadButton::A,
        _ => continue
      };

      let joypad_event = match k {
        KeyState::Pressed => JoypadEvent::Press(button),
        KeyState::Released => JoypadEvent::Release(button),
        KeyState::None => continue,
      };

      joypad.on_event(joypad_event);
    }

    Shutdown::No
  }

  fn elapsed_millis(&self) -> usize {
    self.time.elapsed().as_millis() as usize
  }

  fn delay(&self, d: std::time::Duration) {
    self.browser.delay(d.as_millis() as usize);
  }
}

impl WasmHostSystem {
  pub fn new(browser: BrowserNes) -> Self {
    Self { browser, keyboard: KeyboardState::default(), time: wasm_timer::Instant::now() }
  }
}

#[wasm_bindgen]
pub struct NesWasm {
  nes: Nes,
}

#[wasm_bindgen]
impl NesWasm {
  pub fn new(browser: BrowserNes, bin: &[u8]) -> Self {
    let cart = if let Ok(cart) = Cartridge::load(bin) {
      cart
    } else {
      log("ERROR Failed to load. Invalid ROM. Loading nestest instead.");
      Cartridge::load(include_bytes!("../../test-roms/nestest/nestest.nes")).unwrap()
    };

    log(format!("nes init! {}", cart).as_str());
    let nes = Nes::insert(cart, WasmHostSystem::new(browser));
    Self { nes }
  }

  pub fn tick(&mut self) {
    self.nes.tick();
  }
}