use std::{collections::HashSet, time::Instant};
use jni::{
    objects::{JClass, JObject, GlobalRef},
    sys::{jlong, jbyteArray},
    JNIEnv,
};
use nes::{cartridge::Cartridge, nes::Nes, joypad::JoypadButton};

static KEYS: &[JoypadButton] = &[
  JoypadButton::B,
  JoypadButton::A,
  JoypadButton::UP,
  JoypadButton::DOWN,
  JoypadButton::LEFT,
  JoypadButton::RIGHT,
  JoypadButton::START,
  JoypadButton::SELECT
];

#[no_mangle]
pub extern "C" fn Java_nes_potatis_Rust_init(
  env: JNIEnv<'static>, 
  _: JClass, 
  rom: jbyteArray,
  bindings: JObject,
  panic_handler: JObject
) -> jlong {

  // Panic to LogCat
  let jvm = env.get_java_vm().unwrap();
  let panic_handler = env.new_global_ref(panic_handler).unwrap();
  std::panic::set_hook(Box::new(move |info| {
    let env = jvm.get_env().unwrap();
    let s = env.new_string(info.to_string()).unwrap();
    env.call_method(&panic_handler, "panic", "(Ljava/lang/String;)V", &[s.into()]).unwrap();
  }));

  // Create global refs to unsure JVM doesn't GC them
  let bindings = env.new_global_ref(bindings).unwrap();
  let rom = env.convert_byte_array(rom).unwrap();
  let rom = if rom.is_empty() {
      include_bytes!("../../test-roms/nestest/nestest.nes").to_vec()
    } else {
      rom
    };
  
  let cart = Cartridge::blow_dust_vec(rom).unwrap();
  let host = AndroidHost::new(env, bindings);
  let nes = Nes::insert(cart, host);

  Box::into_raw(Box::new(nes)) as i64
}

#[no_mangle]
pub extern "C" fn Java_nes_potatis_Rust_tick(_: JNIEnv, _: JClass, ptr: jlong) {
  unsafe {
    let nes = &mut *(ptr as *mut Nes);
    nes.tick()
  }
}

#[no_mangle]
pub extern "C" fn Java_nes_potatis_Rust_destroy(_: JNIEnv, _: JClass, ptr: jlong) {
  unsafe {
    let _ = Box::from_raw(ptr as *mut Nes);
    // dropped
  }
}

struct AndroidHost {
  env: JNIEnv<'static>,
  bindings: GlobalRef,
  pressed: HashSet<JoypadButton>,
  time: Instant,
}

impl AndroidHost {
  fn new(env: JNIEnv<'static>, bindings: GlobalRef) -> Self {
    Self { env, bindings, pressed: HashSet::with_capacity(8), time: Instant::now() }
  }
}

impl nes::nes::HostPlatform for AndroidHost {
  fn elapsed_millis(&self) -> usize {
    self.time.elapsed().as_millis() as usize
  }

  fn delay(&self, dur: std::time::Duration) {
    std::thread::sleep(dur)
  }

  fn render(&mut self, frame: &nes::frame::RenderFrame) {
    let pixels: Vec<u8> = frame.pixels_ntsc().collect();
    unsafe {
      let jpixels: jbyteArray = self.env.byte_array_from_slice(&pixels).unwrap();
      let jobj = JObject::from_raw(jpixels);

      // TODO: Is it possible/good for perf to cache the method lookup? call_method_unchecked.
      self.env.call_method(&self.bindings, "render", "([B)V", &[jobj.into()]).unwrap();
    }
  }

  fn poll_events(&mut self, joypad: &mut nes::joypad::Joypad) -> nes::nes::Shutdown {
    let state = self.env.call_method(&self.bindings, "input", "()B", &[]).unwrap();
    let state = state.b().unwrap();
    
    let was_pressed = self.pressed.clone();
    self.pressed.clear();
    for (i, k) in KEYS.iter().enumerate() {
      if (state >> i) & 1 == 1 {
        self.pressed.insert(*k);
      }
    }

    self.pressed.iter().for_each(|btn| {
      joypad.on_event(nes::joypad::JoypadEvent::Press(*btn));
    });

    was_pressed.symmetric_difference(&self.pressed).for_each(|btn| {
      joypad.on_event(nes::joypad::JoypadEvent::Release(*btn));
    });
    

    nes::nes::Shutdown::No
  }
}