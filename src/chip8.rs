use std::{collections::HashSet, time::Duration};

use rand::Rng;
use sdl2::{event::Event, keyboard::Keycode};

const ADDR_START_PROGRAM: u16 = 0x200;

const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Chip8<'a> {
    // V0 - VF
    pub v: [u8; 16],

    // Index register
    pub i: u16,

    // Stack
    pub stack: [u8; 64],

    // Stack pointer
    pub sp: u8,

    // Delay timer
    pub dt: u8,

    // Sound timer
    pub st: u8,

    // Display
    pub display: [[bool; 64]; 32],

    // Program counter
    pub pc: u16,

    // Memory
    // 0x000 - 0x080 -> Font Set
    // 0x200 - 0xFFF -> Program/Data Space
    pub memory: [u8; 4096],

    // Keyboard
    pub keyboard: [bool; 16],

    sdl: &'a sdl2::Sdl,
}

impl Chip8<'static> {
    pub fn new(sdl_context: &sdl2::Sdl) -> Chip8 {
        let mut mem = [0; 4096];
        mem[..80].copy_from_slice(&FONT_SET);

        Chip8 {
            v: [0; 16],
            i: 0,
            stack: [0; 64],
            sp: 0,
            dt: 0,
            st: 0,
            display: [[false; 64]; 32],
            pc: ADDR_START_PROGRAM,
            memory: mem,
            keyboard: [false; 16],
            sdl: sdl_context,
        }
    }

    pub fn run_op_code(&mut self, code: u16) {
        let (op1, op2, op3, op4) = (
            ((code & 0xF000) >> 12) as u8,
            ((code & 0x0F00) >> 8) as u8,
            ((code & 0x00F0) >> 4) as u8,
            (code & 0x000F) as u8,
        );

        // A 12-bit value, the lowest 12 bits of the instruction
        let nnn: u16 = code & 0xFFF;

        // A 4-bit value, the lowest 4 bits of the instruction
        let n: u8 = op4;

        // A 4-bit value, the lower 4 bits of the high byte of the instruction
        let x: u8 = op2;

        // A 4-bit value, the upper 4 bits of the low byte of the instruction
        let y: u8 = op3;

        // An 8-bit value, the lowest 8 bits of the instruction
        let kk: u8 = (code & 0xFF) as u8;

        match (op1, op2, op3, op4) {
            (0x0, 0x0, 0xE, 0xE) => self.ret(),
            (0x0, 0x0, 0xE, 0x0) => self.cls(),
            (0x1, _, _, _) => self.jp_addr(nnn),
            (0x2, _, _, _) => self.call_addr(nnn),
            (0x3, _, _, _) => self.se_vx_byte(x, kk),
            (0x4, _, _, _) => self.sne_vx_byte(x, kk),
            (0x5, _, _, 0x0) => self.se_vx_vy(x, y),
            (0x6, _, _, _) => self.ld_vx_byte(x, kk),
            (0x7, _, _, _) => self.add_vx_byte(x, kk),
            (0x8, _, _, 0x0) => self.ld_vx_vy(x, y),
            (0x8, _, _, 0x1) => self.or_vx_vy(x, y),
            (0x8, _, _, 0x2) => self.and_vx_vy(x, y),
            (0x8, _, _, 0x3) => self.xor_vx_vy(x, y),
            (0x8, _, _, 0x4) => self.add_vx_vy(x, y),
            (0x8, _, _, 0x5) => self.sub_vx_vy(x, y),
            (0x8, _, _, 0x6) => self.shr_vx_vy(x),
            (0x8, _, _, 0x7) => self.subn_vx_vy(x, y),
            (0x8, _, _, 0xE) => self.shl_vx_vy(x),
            (0x9, _, _, 0x0) => self.sne_vx_vy(x, y),
            (0xA, _, _, _) => self.ld_i_addr(nnn),
            (0xB, _, _, _) => self.jp_v0_addr(nnn),
            (0xC, _, _, _) => self.rnd_vx_byte(x, kk),
            (0xD, _, _, _) => self.drw_vx_vy_nibble(x, y, n),
            (0xE, _, 0x9, 0xE) => self.skp_vx(x),
            (0xE, _, 0xA, 0x1) => self.sknp_vx(x),
            (0xF, _, 0x0, 0x7) => self.ld_vx_dt(x),
            (0xF, _, 0x0, 0xA) => self.ld_vx_k(x),
            (0xF, _, 0x1, 0x5) => self.ld_dt_vx(x),
            (0xF, _, 0x1, 0x8) => self.ld_st_vx(x),
            (0xF, _, 0x1, 0xE) => self.add_i_vx(x),
            (0xF, _, 0x2, 0x9) => self.ld_f_vx(x),
            (0xF, _, 0x3, 0x3) => self.ld_b_vx(x),
            (0xF, _, 0x5, 0x5) => self.ld_i_vx(x),
            (0xF, _, 0x6, 0x5) => self.ld_vx_i(x),
            _ => println!("undefined"),
        }
    }

    // 00EE - RET
    fn ret(&mut self) {
        self.pc = self.stack[self.sp as usize] as u16;
        if self.sp != 0 { self.sp -= 1; }
    }

    // 00E0 - CLS
    fn cls(&mut self) {
        self.display = [[false; 64]; 32];
        self.pc += 2;
    }

    // 1nnn - JP addr
    fn jp_addr(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    // 2nnn - CALL addr
    fn call_addr(&mut self, nnn: u16) {
        self.sp += 1;
        self.stack[self.sp as usize] = self.pc as u8;
        self.pc = nnn;
    }

    // 3xkk - SE Vx, byte
    fn se_vx_byte(&mut self, x: u8, kk: u8) {
        if self.v[x as usize] == kk { self.pc += 4; }
    }

    // 4xkk - SNE Vx, byte
    fn sne_vx_byte(&mut self, x: u8, kk: u8) {
        if self.v[x as usize] != kk { self.pc += 4; }
    }

    // 5xy0 - SE Vx, Vy
    fn se_vx_vy(&mut self, x: u8, y: u8) {
        if self.v[x as usize] == self.v[y as usize] { self.pc += 4; }
    }

    // 6xkk - LD Vx, byte
    fn ld_vx_byte(&mut self, x: u8, kk: u8) {
        self.v[x as usize] = kk;
        self.pc += 2;
    }

    // 7xkk - ADD Vx, byte
    fn add_vx_byte(&mut self, x: u8, kk: u8) {
        self.v[x as usize] += kk;
        self.pc += 2;
    }

    // 8xy0 - LD Vx, Vy
    fn ld_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
        self.pc += 2;
    }

    // 8xy1 - OR Vx, Vy
    fn or_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] |= self.v[y as usize];
        self.pc += 2;
    }

    // 8xy2 - AND Vx, Vy
    fn and_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] &= self.v[y as usize];
        self.pc += 2;
    }

    // 8xy3 - XOR Vx, Vy
    fn xor_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] ^= self.v[y as usize];
        self.pc += 2;
    }

    // 8xy4 - ADD Vx, Vy
    fn add_vx_vy(&mut self, x: u8, y: u8) {
        let sum = self.v[x as usize] as u16 + self.v[y as usize] as u16;
        self.v[x as usize] = sum as u8;
        self.v[0xF] = (sum > 255) as u8;
        self.pc += 2;
    }

    // 8xy5 - SUB Vx, Vy
    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        self.v[0xf] = (self.v[x as usize] > self.v[y as usize]) as u8;
        self.v[x as usize] -= self.v[y as usize];
        self.pc += 2;
    }

    // 8xy6 - SHR Vx {, Vy}
    fn shr_vx_vy(&mut self, x: u8) {
        self.v[0xf] = (self.v[x as usize] & 0b1 == 1) as u8;
        self.v[x as usize] >>= 1;
        self.pc += 2;
    }

    // 8xy7 - SUBN Vx, Vy
    fn subn_vx_vy(&mut self, x: u8, y: u8) {
        self.v[0xf] = (self.v[y as usize] > self.v[x as usize]) as u8;
        self.v[x as usize] = self.v[y as usize] - self.v[x as usize];
        self.pc += 2;
    }

    // 8xyE - SHL Vx {, Vy}
    fn shl_vx_vy(&mut self, x: u8) {
        self.v[0xf] = (self.v[x as usize] >> 7 == 1) as u8;
        self.v[x as usize] <<= 1;
        self.pc += 2;
    }

    // 9xy0 - SNE Vx, Vy
    fn sne_vx_vy(&mut self, x: u8, y: u8) {
        if self.v[x as usize] != self.v[y as usize] { self.pc += 4; }
    }

    // Annn - LD I, addr
    fn ld_i_addr(&mut self, nnn: u16) {
        self.i = nnn;
        self.pc += 2;
    }

    // Bnnn - JP V0, addr
    fn jp_v0_addr(&mut self, nnn: u16) {
        self.pc = self.v[0] as u16 + nnn;
    }

    // Cxkk - RND Vx, byte
    fn rnd_vx_byte(&mut self, x: u8, kk: u8) {
        let random_byte: u8 = rand::thread_rng().gen_range(0..255);
        self.v[x as usize] = random_byte & kk;
        self.pc += 2;
    }

    // Dxyn - DRW Vx, Vy, nibble
    // ............................................
    // : display  ^  value : new_display : erased :
    // :...................:.............:........:
    // :       0 :       0 :           0 :      0 :
    // :       0 :       1 :           1 :      0 :
    // :       1 :       0 :           1 :      0 :
    // :       1 :       1 :           0 :      1 :
    // :.........:.........:.............:........:
    fn drw_vx_vy_nibble(&mut self, x: u8, y: u8, n: u8) {
        for byte in 0..n {
            let y_axis = ((self.v[y as usize] + byte) % 32) as usize;
            for bit in 0..8 {
                let x_axis = ((self.v[x as usize] + bit) % 64) as usize;
                let value = self.memory[self.i as usize + byte as usize] >> (7 - bit);
                self.v[0xF] = self.display[y_axis][x_axis] as u8 & value;
                self.display[y_axis][x_axis] = self.display[y_axis][x_axis] ^ (value == 1);
            }
        }
    }

    // Ex9E - SKP Vx
    fn skp_vx(&mut self, x: u8) {
        if self.keyboard[self.v[x as usize] as usize] { self.pc += 4; }
    }

    // ExA1 - SKNP Vx
    fn sknp_vx(&mut self, x: u8) {
        if !self.keyboard[self.v[x as usize] as usize] { self.pc += 2; }
    }

    // Fx07 - LD Vx, DT
    fn ld_vx_dt(&mut self, x: u8) {
        self.v[x as usize] = self.dt;
        self.pc += 2;
    }

    // Fx0A - LD Vx, K
    fn ld_vx_k(&mut self, x: u8) {
        let mut events = self.sdl.event_pump().unwrap();
        'event: loop {
            for event in events.poll_iter() { if let Event::Quit { .. } = event { break 'event; } }

            let keys: HashSet<Keycode> = events
                .keyboard_state()
                .pressed_scancodes()
                .filter_map(Keycode::from_scancode)
                .collect();

            for key_code in keys.iter() {
                let key_value = get_key_value(*key_code);
                if key_value != None {
                    self.v[x as usize] = key_value.unwrap();
                    self.pc += 2;
                    break 'event;
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    // Fx15 - LD DT, Vx
    fn ld_dt_vx(&mut self, x: u8) {
        self.dt = self.v[x as usize];
        self.pc += 2;
    }

    // Fx18 - LD ST, Vx
    fn ld_st_vx(&mut self, x: u8) {
        self.st = self.v[x as usize];
        self.pc += 2;
    }

    // Fx1E - ADD I, Vx
    fn add_i_vx(&mut self, x: u8) {
        self.i += self.v[x as usize] as u16;
        self.pc += 2;
    }

    // Fx29 - LD F, Vx
    fn ld_f_vx(&mut self, x: u8) {
        self.i = self.memory[(self.v[x as usize] * 5) as usize] as u16;
        self.pc += 2;
    }

    // Fx33 - LD B, Vx
    fn ld_b_vx(&mut self, x: u8) {
        let data = self.v[x as usize];
        self.memory[self.i as usize] = data / 100;
        self.memory[(self.i + 1) as usize] = (data % 100) / 10;
        self.memory[(self.i + 2) as usize] = data % 10;
        self.pc += 2;
    }

    // Fx55 - LD [I], Vx
    fn ld_i_vx(&mut self, x: u8) {
        for j in 0..x { self.memory[(self.i + j as u16) as usize] = self.v[j as usize]; }
        self.pc += 2;
    }

    // Fx65 - LD Vx, [I]
    fn ld_vx_i(&mut self, x: u8) {
        for j in 0..x { self.v[j as usize] = self.memory[(self.i + j as u16) as usize]; }
        self.pc += 2;
    }
}

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
fn get_key_value(key: Keycode) -> Option<u8> {
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
