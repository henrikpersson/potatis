use bitflags::bitflags;

bitflags! {
  #[derive(Default)]
  pub struct JoypadButton: u8 {
    const A = 0b00000001;
    const B = 0b00000010;
    const SELECT = 0b00000100;
    const START = 0b00001000;
    const UP = 0b00010000;
    const DOWN = 0b00100000;
    const LEFT = 0b01000000;
    const RIGHT = 0b10000000;
  }
}

#[derive(Debug)]
pub enum JoypadEvent {
  Press(JoypadButton),
  Release(JoypadButton)
}

#[derive(Default)]
pub struct Joypad {
  state: JoypadButton,
  out: u8
}

/*
bit 	  7   6   5     	4     	3   2     1     0    
button 	A 	B 	Select 	Start 	Up 	Down 	Left 	Right 
 */
impl Joypad {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn read(&mut self) -> u8 {
    // It reads 8 times, once per button
    let val = self.out & 1;
    self.out >>= 1;
    val
  }

  pub fn strobe(&mut self, val: u8) {
    if val & 1 == 1 { // Strobe is high
      self.out = self.state.bits;
    }
  }

  pub fn on_event(&mut self, event: JoypadEvent) {
    match event {
      JoypadEvent::Press(b) => self.state.set(b, true),
      JoypadEvent::Release(b) => self.state.set(b, false),
    }
  }
}