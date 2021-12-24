use sdl2::keyboard::Keycode;

pub struct Keypad {
    key: Option<u8>,
}

impl Keypad {
    pub fn new() -> Self { Self { key: None } }

    pub fn is_pressed(&self, key: u8) -> bool { if let Some(i) = self.key { i == key } else { false } }

    pub fn get_key(&self) -> Option<u8> { self.key }

    pub fn down_key(&mut self, key: Keycode) { self.key = self.get_key_value(key); }

    pub fn up_key(&mut self) { self.key = None }

    // Original             Current
    // +---+---+---+---+    +---+---+---+---+
    // | 1 | 2 | 3 | C |    | 1 | 2 | 3 | 4 |
    // +---+---+---+---+    +---+---+---+---+
    // | 4 | 5 | 6 | D |    | Q | W | E | R |
    // +---+---+---+---+    +---+---+---+---+
    // | 7 | 8 | 9 | E |    | A | S | D | F |
    // +---+---+---+---+    +---+---+---+---+
    // | A | 0 | B | F |    | Z | X | C | V |
    // +---+---+---+---+    +---+---+---+---+
    fn get_key_value(&self, key: Keycode) -> Option<u8> {
        match key {
            Keycode::Num1 => Some(1),
            Keycode::Num2 => Some(2),
            Keycode::Num3 => Some(3),
            Keycode::Num4 => Some(0xC),
            Keycode::Q => Some(4),
            Keycode::W => Some(5),
            Keycode::E => Some(6),
            Keycode::R => Some(0xD),
            Keycode::A => Some(7),
            Keycode::S => Some(8),
            Keycode::D => Some(9),
            Keycode::F => Some(0xE),
            Keycode::Z => Some(0xA),
            Keycode::X => Some(0),
            Keycode::C => Some(0xB),
            Keycode::V => Some(0xF),
            _ => None,
        }
    }
}
