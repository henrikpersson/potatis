use std::collections::HashMap;

use crate::frame::RenderFrame;

const W: usize = 5;

lazy_static! {
  static ref FONTS: HashMap<char, &'static str> = {
    let mut m = HashMap::with_capacity(10);
    m.insert('0', r#"
    ....
    .  .
    .  .
    .  .
    .  .
    .  .
    ...."#);

    m.insert('1', r#"
     ..
      .
      .
      .
      .
      .
      ."#);

    m.insert('2', r#"
    ...
      .
      .
      .
    ... 
    .   
    ...."#);

    m.insert('3', r#"
    ...
      .
      .
    ...
      .
      .
    ..."#);

    m.insert('4', r#"
    .  .
    .  .
    .  .
    ....
       .
       .
       ."#);

    m.insert('5', r#"
    ....
    .  
    .  
    ....
       .
       .
    ...."#);

    m.insert('6', r#"
    ....
    .  
    .  
    ....
    .  .
    .  .
    ...."#);

    m.insert('7', r#"
    ....
       .
       .
      .  
      . 
     .
     .  "#);

  m.insert('8', r#"
    ....
    .  .
    .  .
    ....
    .  .
    .  .
    ...."#);

    m.insert('9', r#"
    ....
    .  .
    .  .
    ....
       .
       .
       ."#);
    m
  };
}

pub fn draw(s: &str, pos: (usize, usize), frame: &mut RenderFrame) {
  let fonts = s.chars().map(|c| FONTS.get(&c).expect("font missing"));

  for (char_n, font) in fonts.enumerate() {
    let char_base_x = pos.0 + char_n * W;
    let mut x = char_base_x;
    let mut y = pos.1;

    font.chars().for_each(|c| {
      match c {
        '\n' => { 
          y += 1;
          x = char_base_x;
        },
        '.' => {
          frame.set_pixel_xy(x, y, (0xff, 0, 0));
          x += 1;
        }
        ' ' => x += 1,
        _ => ()
      }
    });
  }
  
}