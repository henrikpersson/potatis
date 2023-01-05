use nes::{joypad::{Joypad, JoypadEvent, JoypadButton}, frame::RenderFrame, nes::{HostSystem, Shutdown}};
use sdl2::{pixels::PixelFormatEnum, event::Event, keyboard::Keycode, Sdl, render::{Texture, Canvas, TextureCreator}, video::{Window, WindowContext}};

pub struct SdlHostSystem<'a> {
  context: Sdl,
  canvas: Canvas<Window>,
  texture: Texture<'a>,
  _creator: TextureCreator<WindowContext>
}

impl SdlHostSystem<'_> {
  pub fn new() -> Self {
    // TODO: Configurable
    let scale = 4;
    let w = nes::display::NTSC_W as u32;
    let h = nes::display::NTSC_H as u32;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Potatis", w * scale, h * scale)
      .position_centered()
      .build()
      .unwrap();

    let canvas = window.into_canvas()
      .present_vsync()
      .build()
      .unwrap();

    let mut creator = canvas.texture_creator();
    let texture: Texture = unsafe {
      let ptr = &mut creator as *mut TextureCreator<WindowContext>;
      (*ptr)
        .create_texture_target(PixelFormatEnum::RGBA32, w, h)
        .unwrap()
    };
    
    Self {
      _creator: creator,
      context: sdl_context,
      canvas,
      texture
    }
  }
}

impl HostSystem for SdlHostSystem<'_> {
  fn render(&mut self, frame: &RenderFrame) {
    let ntsc = nes::display::ntsc(frame.pixels());
    self.texture.update(None, &ntsc.pixels, ntsc.pitch).unwrap();
    self.canvas.copy(&self.texture, None, None).unwrap();
    self.canvas.present();
  }

  fn poll_events(&mut self, joypad: &mut Joypad) -> Shutdown {
    for event in self.context.event_pump().unwrap().poll_iter() {
      if let Some(joypad_ev) = map_joypad(&event) {
        joypad.on_event(joypad_ev);
        continue;
      }
      
      match event {
        Event::Quit {..} |
        Event::KeyDown { keycode: Some(Keycode::Q), .. } |
        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Shutdown::Yes,
        Event::KeyDown { keycode: Some(Keycode::R), .. } => return Shutdown::Reset,
        _ => ()
      }
    }
    Shutdown::No
  }
}

fn map_joypad(sdlev: &Event) -> Option<JoypadEvent> {
  match sdlev {
    Event::KeyDown { keycode: Some(keycode), .. } => {
      map_button(keycode).map(JoypadEvent::Press)
    }
    Event::KeyUp { keycode: Some(keycode), .. } => {
      map_button(keycode).map(JoypadEvent::Release)
    }
    _ => None
  }
}

fn map_button(keycode: &Keycode) -> Option<JoypadButton> {
  match keycode {
    Keycode::W => Some(JoypadButton::UP),
    Keycode::A => Some(JoypadButton::LEFT),
    Keycode::S => Some(JoypadButton::DOWN),
    Keycode::D => Some(JoypadButton::RIGHT),
    Keycode::K => Some(JoypadButton::B),
    Keycode::L => Some(JoypadButton::A),
    Keycode::Return => Some(JoypadButton::START),
    Keycode::Space => Some(JoypadButton::SELECT),
    _ => None
  }
}
