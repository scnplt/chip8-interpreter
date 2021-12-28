use std::fs::File;
use std::io::Read;
use std::time::Instant;

use sdl2::{
    event::Event,
    EventPump,
    keyboard::Keycode,
    pixels::Color,
    rect::Rect,
    render::Canvas,
    Sdl,
    video::Window,
};

use crate::keypad::Keypad;

const ADDR_PROGRAM_START: u16 = 0x200;

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

const FRAME_WIDTH: usize = 64;
const FRAME_HEIGHT: usize = 32;

const SCALE: u32 = 10;

pub const WINDOW_TITLE: &str = "CHIP-8 interpreter";
const WINDOW_WIDTH: u32 = (FRAME_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (FRAME_HEIGHT as u32) * SCALE;

const RGB_BLACK: (u8, u8, u8) = (0, 0, 0);
const RGB_WHITE: (u8, u8, u8) = (255, 255, 255);

pub struct Chip8 {
    // V0 - VF
    v: [u8; 16],

    // Index register
    i: u16,

    // Stack
    stack: [u16; 32],

    // Stack pointer
    sp: u8,

    // Delay timer
    dt: u8,

    // Sound timer
    st: u8,

    // Display
    frame: [[u8; FRAME_WIDTH]; FRAME_HEIGHT],

    // Program counter
    pc: u16,

    // Memory
    memory: [u8; 4096],

    // Keypad
    keypad: Keypad,

    // Canvas
    canvas: Canvas<Window>,

    // Event Pump
    event_pump: EventPump,
}

impl Chip8 {
    pub fn new(sdl: &Sdl) -> Self {
        let mut memory = [0; 4096];
        memory[..80].copy_from_slice(&FONT_SET);

        let video_subsystem = sdl.video().expect("Could not create Video Subsystem!");
        let window_builder = video_subsystem.window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
            .build().expect("Could not create Window!");
        let canvas = window_builder.into_canvas().build().expect("Could not create Canvas!");

        Self {
            v: [0; 16],
            i: ADDR_PROGRAM_START,
            stack: [0; 32],
            sp: 0,
            dt: 0,
            st: 0,
            frame: [[0; FRAME_WIDTH]; FRAME_HEIGHT],
            pc: ADDR_PROGRAM_START,
            memory,
            keypad: Keypad::new(),
            canvas,
            event_pump: sdl.event_pump().expect("Event Issue"),
        }
    }

    pub fn load_rom(&mut self, path: &str) {
        let file = File::open(path).expect("Could not read rom!");
        for (i, byte) in file.bytes().enumerate() {
            let addr = ADDR_PROGRAM_START as usize + i;
            self.memory[addr] = byte.expect("Could not read rom!");
        }
    }

    pub fn start_cycle(&mut self, delay: u64) {
        let mut start = Instant::now();
        'cycle: loop {
            for event in self.event_pump.poll_iter() {
                if let Event::Quit { .. } = event { break 'cycle; }
            }

            let keys: Vec<Keycode> = self.event_pump.keyboard_state()
                .pressed_scancodes().filter_map(Keycode::from_scancode).collect();

            for key in keys {
                if key == Keycode::Escape { break 'cycle; }
                self.keypad.down_key(key);
            }

            if start.elapsed().as_millis() <= delay as u128 { continue; }
            start = Instant::now();

            let pc = self.pc as usize;
            let op_code = ((self.memory[pc] as u16) << 8) | self.memory[pc + 1] as u16;
            self.run_op_code(op_code);
            self.update_screen();

            if self.dt > 0 { self.dt -= 1; }
            if self.st > 0 { self.st -= 1; }
            self.keypad.up_key();
        }
    }

    fn update_screen(&mut self) {
        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let rgb = if self.frame[y][x] == 1 { RGB_WHITE } else { RGB_BLACK };
                let color = Color::from(rgb);

                self.canvas.set_draw_color(color);
                self.canvas.fill_rect(Rect::new(
                    (x as u32 * SCALE) as i32,
                    (y as u32 * SCALE) as i32,
                    SCALE,
                    SCALE,
                )).expect("Fill Rect Issue");
            }
        }
        self.canvas.present();
    }

    fn run_op_code(&mut self, code: u16) {
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
            _ => self.next_program(),
        }
    }

    // 00EE - RET
    fn ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize] as u16;
        self.next_program();
    }

    // 00E0 - CLS
    fn cls(&mut self) {
        self.frame = [[0; FRAME_WIDTH]; FRAME_HEIGHT];
        self.next_program();
    }

    // 1nnn - JP addr
    fn jp_addr(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    // 2nnn - CALL addr
    fn call_addr(&mut self, nnn: u16) {
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = nnn;
    }

    // 3xkk - SE Vx, byte
    fn se_vx_byte(&mut self, x: u8, kk: u8) {
        self.next_program();
        if self.v[x as usize] == kk { self.next_program(); }
    }

    // 4xkk - SNE Vx, byte
    fn sne_vx_byte(&mut self, x: u8, kk: u8) {
        self.next_program();
        if self.v[x as usize] != kk { self.next_program(); }
    }

    // 5xy0 - SE Vx, Vy
    fn se_vx_vy(&mut self, x: u8, y: u8) {
        self.next_program();
        if self.v[x as usize] == self.v[y as usize] { self.next_program(); }
    }

    // 6xkk - LD Vx, byte
    fn ld_vx_byte(&mut self, x: u8, kk: u8) {
        self.v[x as usize] = kk;
        self.next_program();
    }

    // 7xkk - ADD Vx, byte
    fn add_vx_byte(&mut self, x: u8, kk: u8) {
        self.v[x as usize] = self.v[x as usize].overflowing_add(kk).0;
        self.next_program();
    }

    // 8xy0 - LD Vx, Vy
    fn ld_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
        self.next_program();
    }

    // 8xy1 - OR Vx, Vy
    fn or_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] |= self.v[y as usize];
        self.next_program();
    }

    // 8xy2 - AND Vx, Vy
    fn and_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] &= self.v[y as usize];
        self.next_program();
    }

    // 8xy3 - XOR Vx, Vy
    fn xor_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] ^= self.v[y as usize];
        self.next_program();
    }

    // 8xy4 - ADD Vx, Vy
    fn add_vx_vy(&mut self, x: u8, y: u8) {
        let (sum, overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
        self.v[x as usize] = sum;
        self.v[0xF] = overflow as u8;
        self.next_program();
    }

    // 8xy5 - SUB Vx, Vy
    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        let (result, overflow) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
        self.v[x as usize] = result;
        self.v[0xF] = !overflow as u8;
        self.next_program();
    }

    // 8xy6 - SHR Vx {, Vy}
    fn shr_vx_vy(&mut self, x: u8) {
        self.v[0xF] = (self.v[x as usize] & 1 == 1) as u8;
        self.v[x as usize] >>= 1;
        self.next_program();
    }

    // 8xy7 - SUBN Vx, Vy
    fn subn_vx_vy(&mut self, x: u8, y: u8) {
        let (result, overflow) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
        self.v[0xF] = !overflow as u8;
        self.v[x as usize] = result;
        self.next_program();
    }

    // 8xyE - SHL Vx {, Vy}
    fn shl_vx_vy(&mut self, x: u8) {
        self.v[0xF] = (self.v[x as usize] >> 7 == 1) as u8;
        self.v[x as usize] <<= 1;
        self.next_program();
    }

    // 9xy0 - SNE Vx, Vy
    fn sne_vx_vy(&mut self, x: u8, y: u8) {
        self.next_program();
        if self.v[x as usize] != self.v[y as usize] { self.next_program(); }
    }

    // Annn - LD I, addr
    fn ld_i_addr(&mut self, nnn: u16) {
        self.i = nnn;
        self.next_program();
    }

    // Bnnn - JP V0, addr
    fn jp_v0_addr(&mut self, nnn: u16) {
        self.pc = (self.v[0] as u16 + nnn).min(0xFFF);
    }

    // Cxkk - RND Vx, byte
    fn rnd_vx_byte(&mut self, x: u8, kk: u8) {
        self.v[x as usize] = rand::random::<u8>() & kk;
        self.next_program();
    }

    // Dxyn - DRW Vx, Vy, nibble
    fn drw_vx_vy_nibble(&mut self, x: u8, y: u8, n: u8) {
        self.v[0xF] = 0;
        for byte in 0..n {
            let y = (self.v[y as usize].overflowing_add(byte).0 % 32) as usize;
            let sprite = self.memory[(self.i + byte as u16) as usize];
            for bit in 0..8 {
                let x = (self.v[x as usize].overflowing_add(bit).0 % 64) as usize;
                let pixel = (sprite >> (7 - bit)) & 1;
                self.v[0xF] |= self.frame[y][x] & pixel;
                self.frame[y][x] ^= pixel;
            }
        }
        self.next_program();
    }

    // Ex9E - SKP Vx
    fn skp_vx(&mut self, x: u8) {
        self.next_program();
        if self.keypad.is_pressed(self.v[x as usize]) { self.next_program(); }
    }

    // ExA1 - SKNP Vx
    fn sknp_vx(&mut self, x: u8) {
        self.next_program();
        if !self.keypad.is_pressed(self.v[x as usize]) { self.next_program(); }
    }

    // Fx07 - LD Vx, DT
    fn ld_vx_dt(&mut self, x: u8) {
        self.v[x as usize] = self.dt;
        self.next_program();
    }

    // Fx0A - LD Vx, K
    fn ld_vx_k(&mut self, x: u8) {
        if let Some(key) = self.keypad.get_key() {
            self.v[x as usize] = key;
            self.next_program();
        }
    }

    // Fx15 - LD DT, Vx
    fn ld_dt_vx(&mut self, x: u8) {
        self.dt = self.v[x as usize];
        self.next_program();
    }

    // Fx18 - LD ST, Vx
    fn ld_st_vx(&mut self, x: u8) {
        self.st = self.v[x as usize];
        self.next_program();
    }

    // Fx1E - ADD I, Vx
    fn add_i_vx(&mut self, x: u8) {
        self.i += self.v[x as usize] as u16;
        self.next_program();
    }

    // Fx29 - LD F, Vx
    fn ld_f_vx(&mut self, x: u8) {
        self.i = (self.v[x as usize] * 5) as u16;
        self.next_program();
    }

    // Fx33 - LD B, Vx
    fn ld_b_vx(&mut self, x: u8) {
        let data = self.v[x as usize];
        self.memory[self.i as usize] = data / 100;
        self.memory[(self.i + 1) as usize] = (data % 100) / 10;
        self.memory[(self.i + 2) as usize] = data % 10;
        self.next_program();
    }

    // Fx55 - LD [I], Vx
    fn ld_i_vx(&mut self, x: u8) {
        for j in 0..=x as u16 { self.memory[(self.i + j) as usize] = self.v[j as usize]; }
        self.next_program();
    }

    // Fx65 - LD Vx, [I]
    fn ld_vx_i(&mut self, x: u8) {
        for j in 0..=x as u16 { self.v[j as usize] = self.memory[(self.i + j) as usize]; }
        self.next_program();
    }

    fn next_program(&mut self) { self.pc = (self.pc + 2).min(0xFFF); }
}

// cargo test -- --test-threads=1
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_00e0() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());
        chip.frame = [[1; 64]; 32];
        chip.run_op_code(0x00E0);
        assert_eq!(chip.frame, [[0; 64]; 32]);
        assert_eq!(chip.pc, 0x202)
    }

    #[test]
    fn test_00ee() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());
        chip.sp = 2;
        chip.stack = [3; 32];
        chip.run_op_code(0x00EE);
        assert_eq!(chip.sp, 1);
        assert_eq!(chip.pc, 3 + 2);
    }

    #[test]
    fn test_1nnn() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());
        chip.run_op_code(0x1444);
        assert_eq!(chip.pc, 0x444);
    }

    #[test]
    fn test_2nnn() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());
        chip.run_op_code(0x2456);

        assert_eq!(chip.sp, 1);
        assert_eq!(chip.pc, 0x456);
        assert_eq!(chip.stack[(chip.sp - 1) as usize], 0x200)
    }

    #[test]
    fn test_3xkk() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        // Vx == kk
        chip.v[2] = 0x12;
        chip.run_op_code(0x3212);
        assert_eq!(chip.pc, 0x204);

        // Vx != kk
        chip.v[2] = 0x11;
        chip.run_op_code(0x3212);
        assert_eq!(chip.pc, 0x206)
    }

    #[test]
    fn test_4xkk() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        // Vx != kk
        chip.v[2] = 0x12;
        chip.run_op_code(0x4211);
        assert_eq!(chip.pc, 0x204);

        // Vx == kk
        chip.v[2] = 0x11;
        chip.run_op_code(0x4211);
        assert_eq!(chip.pc, 0x206);
    }

    #[test]
    fn test_5xy0() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        // Vx == Vy
        chip.v[2] = 0x2;
        chip.v[3] = 0x2;
        chip.run_op_code(0x5230);
        assert_eq!(chip.pc, 0x204);

        // Vx != Vy
        chip.v[3] = 0x3;
        chip.run_op_code(0x5230);
        assert_eq!(chip.pc, 0x206);
    }

    #[test]
    fn test_6xkk() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.run_op_code(0x6233);
        assert_eq!(chip.v[2], 0x33);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_7xkk() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[2] = 0x2;
        chip.run_op_code(0x7201);
        assert_eq!(chip.v[2], 0x3);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_8xy0() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0x2;
        chip.v[2] = 0x3;
        chip.run_op_code(0x8120);
        assert_eq!(chip.v[1], 0x3);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_8xy1() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0xF0;
        chip.v[2] = 0x0F;
        chip.run_op_code(0x8121);
        assert_eq!(chip.v[1], 0xFF);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_8xy2() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0xFF;
        chip.v[2] = 0x0F;
        chip.run_op_code(0x8122);
        assert_eq!(chip.v[1], 0x0F);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_8xy3() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0xF0;
        chip.v[2] = 0xFF;
        chip.run_op_code(0x8123);
        assert_eq!(chip.v[1], 0x0F);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_8xy4() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0xAA;
        chip.v[2] = 0xAA;
        chip.run_op_code(0x8124);
        assert_eq!(chip.v[1], 0x54);
        assert_eq!(chip.v[0xF], 1);
        assert_eq!(chip.pc, 0x202);

        chip.v[1] = 0x11;
        chip.v[2] = 0x22;
        chip.run_op_code(0x8124);
        assert_eq!(chip.v[1], 0x33);
        assert_eq!(chip.v[0xF], 0);
        assert_eq!(chip.pc, 0x204);
    }

    #[test]
    fn test_8xy5() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0xFF;
        chip.v[2] = 0x11;
        chip.run_op_code(0x8125);
        assert_eq!(chip.v[1], 0xEE);
        assert_eq!(chip.v[0xF], 1);
        assert_eq!(chip.pc, 0x202);

        chip.v[1] = 0x11;
        chip.v[2] = 0xFF;
        chip.run_op_code(0x8125);
        assert_eq!(chip.v[1], 0x12);
        assert_eq!(chip.v[0xF], 0);
        assert_eq!(chip.pc, 0x204);
    }

    #[test]
    fn test_8xy6() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[5] = 14;
        chip.run_op_code(0x8506);
        assert_eq!(chip.v[0xF], 0);
        assert_eq!(chip.v[5], 7);
    }

    #[test]
    fn test_8xy7() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0x1;
        chip.v[2] = 0x2;
        chip.run_op_code(0x8127);
        assert_eq!(chip.v[0xF], 1);
        assert_eq!(chip.v[1], 0x1);
        assert_eq!(chip.pc, 0x202);

        chip.v[1] = 0x2;
        chip.v[2] = 0x1;
        chip.run_op_code(0x8127);
        assert_eq!(chip.v[0xF], 0);
        assert_eq!(chip.v[1], 0xFF);
        assert_eq!(chip.pc, 0x204);
    }

    #[test]
    fn test_8xye() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 0xAA;
        chip.run_op_code(0x810E);
        assert_eq!(chip.v[0xF], 1);
        assert_eq!(chip.v[1], 0x54);
        assert_eq!(chip.pc, 0x202);

        chip.run_op_code(0x810E);
        assert_eq!(chip.v[0xF], 0);
        assert_eq!(chip.v[1], 0xA8);
        assert_eq!(chip.pc, 0x204);
    }

    #[test]
    fn test_9xy0() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 1;
        chip.v[2] = 2;
        chip.run_op_code(0x9120);
        assert_eq!(chip.pc, 0x204);

        chip.v[1] = 2;
        chip.run_op_code(0x9120);
        assert_eq!(chip.pc, 0x206);
    }

    #[test]
    fn test_annn() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.run_op_code(0xA123);
        assert_eq!(chip.i, 0x123);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_bnnn() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[0] = 2;
        chip.run_op_code(0xB123);
        assert_eq!(chip.pc, 0x125);
    }

    #[test]
    fn test_cxkk() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 1;
        chip.run_op_code(0xC1AA);
        assert_ne!(chip.v[1], 1);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_dxyn() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.i = 0x400;
        chip.v[0] = 2;
        chip.v[1] = 1;
        chip.memory[0x400] = 0b11101010;
        chip.memory[0x401] = 0b10101100;
        chip.memory[0x402] = 0b10101010;
        chip.memory[0x403] = 0b11101001;
        chip.run_op_code(0xD014);
        assert_eq!(chip.frame[1][2..10], [1, 1, 1, 0, 1, 0, 1, 0]);
        assert_eq!(chip.frame[2][2..10], [1, 0, 1, 0, 1, 1, 0, 0]);
        assert_eq!(chip.frame[3][2..10], [1, 0, 1, 0, 1, 0, 1, 0]);
        assert_eq!(chip.frame[4][2..10], [1, 1, 1, 0, 1, 0, 0, 1]);
        assert_eq!(chip.v[0xF], 0);
        assert_eq!(chip.pc, 0x202);

        chip.run_op_code(0xD004);
        assert_eq!(chip.v[0xF], 1);
        assert_eq!(chip.pc, 0x204);
    }

    #[test]
    fn test_ex9e() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 1;
        chip.keypad.down_key(Keycode::Num1);
        chip.run_op_code(0xE19E);
        assert_eq!(chip.pc, 0x204);

        chip.keypad.up_key();
        chip.run_op_code(0xE19E);
        assert_eq!(chip.pc, 0x206);
    }

    #[test]
    fn test_exa1() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 1;
        chip.run_op_code(0xE1A1);
        assert_eq!(chip.pc, 0x204);

        chip.keypad.down_key(Keycode::Num1);
        chip.run_op_code(0xE1A1);
        assert_eq!(chip.pc, 0x206);
    }

    #[test]
    fn test_fx07() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.dt = 2;
        chip.run_op_code(0xF107);
        assert_eq!(chip.v[1], 2);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx0a() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.keypad.down_key(Keycode::Num1);
        chip.run_op_code(0xF10A);
        assert_eq!(chip.v[1], 1);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx15() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 3;
        chip.run_op_code(0xF115);
        assert_eq!(chip.dt, 3);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx18() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 3;
        chip.run_op_code(0xF118);
        assert_eq!(chip.st, 3);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx1e() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 2;
        chip.run_op_code(0xF11E);
        assert_eq!(chip.i, 0x202);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx29() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 1;
        chip.run_op_code(0xF129);
        assert_eq!(chip.i, 5);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx33() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.v[1] = 123;
        chip.run_op_code(0xF133);
        assert_eq!(chip.memory[chip.i as usize], 1);
        assert_eq!(chip.memory[chip.i as usize + 1], 2);
        assert_eq!(chip.memory[chip.i as usize + 2], 3);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx55() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());
        let i = chip.i as usize;

        chip.v[0] = 0;
        chip.v[1] = 1;
        chip.v[2] = 2;
        chip.run_op_code(0xF255);
        assert_eq!(chip.memory[i], 0);
        assert_eq!(chip.memory[i + 1], 1);
        assert_eq!(chip.memory[i + 2], 2);
        assert_eq!(chip.pc, 0x202);
    }

    #[test]
    fn test_fx65() {
        let mut chip = Chip8::new(&sdl2::init().unwrap());

        chip.memory[chip.i as usize] = 0;
        chip.memory[chip.i as usize + 1] = 1;
        chip.memory[chip.i as usize + 2] = 2;
        chip.run_op_code(0xF265);
        assert_eq!(chip.v[0], 0);
        assert_eq!(chip.v[1], 1);
        assert_eq!(chip.v[2], 2);
        assert_eq!(chip.pc, 0x202);
    }
}
