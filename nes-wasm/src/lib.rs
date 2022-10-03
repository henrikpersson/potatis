use nes::{cartridge::Cartridge, nes::{Nes, HostSystem}, joypad::{JoypadButton, JoypadEvent}};
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
}

impl HostSystem for WasmHostSystem {
  fn render(&mut self, frame: &nes::frame::RenderFrame) {
    self.browser.on_frame_ready(frame.pixels().as_ptr(), frame.pixels().len());
  }

  fn poll_events(&mut self, joypad: &mut nes::joypad::Joypad) {
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
  }
}

impl WasmHostSystem {
  pub fn new(browser: BrowserNes) -> Self {
    Self { browser, keyboard: KeyboardState::default() }
  }
}

#[wasm_bindgen]
pub struct NesWasm {
  nes: Nes,
}

#[wasm_bindgen]
impl NesWasm {
  pub fn new(browser: BrowserNes, mut bin: &[u8]) -> Self {
    if bin.is_empty() {
      bin = include_bytes!("../../test-roms/nestest/nestest.nes");
    }
  
    let cart = Cartridge::blow_binary_dust(bin).unwrap();
    log(format!("nes init! {}", cart).as_str());

    let nes = Nes::insert(cart, WasmHostSystem::new(browser));
    Self { nes }
  }

  pub fn tick(&mut self) {
    self.nes.tick();
  }
}