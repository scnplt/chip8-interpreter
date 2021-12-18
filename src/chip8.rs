use std::io;

use rand::Rng;

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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Chip8 {
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
}

impl Chip8 {
    pub fn new() -> Chip8 {
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
        }
    }

    pub fn run_op_code(&mut self, code: u16) {
        let (op1, op2, op3, op4) = (
            ((code & 0xF000) >> 12) as u8,
            ((code & 0x0F00) >> 8) as u8,
            ((code & 0x00F0) >> 4) as u8,
            (code & 0x000F) as u8
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

    // 00EE
    // The interpreter sets the program counter to the address at the top of the stack,
    // then subtracts 1 from the stack pointer.
    fn ret(&mut self) {
        println!("00EE - RET");
        self.pc = self.stack[self.sp as usize] as u16;
        if self.sp != 0 { self.sp -= 1; }
    }

    // 00E0
    // Clear the display.
    fn cls(&mut self) {
        println!("00E0 - CLS");
        self.display = [[false; 64]; 32];
        self.pc += 1;
    }

    // 1nnn
    // The interpreter sets the program counter to nnn.
    fn jp_addr(&mut self, nnn: u16) {
        println!("1nnn - JP addr");
        self.pc = nnn;
    }

    // 2nnn
    // The interpreter increments the stack pointer, then puts the current PC on the top
    // of the stack. The PC is then set to nnn.
    fn call_addr(&mut self, nnn: u16) {
        println!("2nnn - CALL addr");
        self.sp += 1;
        self.stack[self.sp as usize] = self.pc as u8;
        self.pc = nnn;
    }

    // 3xkk
    // The interpreter compares register Vx to kk, and if they are equal,
    // increments the program counter by 2.
    fn se_vx_byte(&mut self, x: u8, kk: u8) {
        println!("3xkk - SE Vx, byte");
        if self.v[x as usize] == kk { self.pc += 2; }
    }

    // 4xkk
    // The interpreter compares register Vx to kk, and if they are not equal,
    // increments the program counter by 2.
    fn sne_vx_byte(&mut self, x: u8, kk: u8) {
        println!("4xkk - SNE Vx, byte");
        if self.v[x as usize] != kk { self.pc += 2; }
    }

    // 5xy0
    // The interpreter compares register Vx to register Vy, and if they are equal,
    // increments the program counter by 2.
    fn se_vx_vy(&mut self, x: u8, y: u8) {
        println!("5xy0 - SE Vx, Vy");
        if self.v[x as usize] == self.v[y as usize] { self.pc += 2; }
    }

    // 6xkk
    // The interpreter puts the value kk into register Vx
    fn ld_vx_byte(&mut self, x: u8, kk: u8) {
        println!("6xkk - LD Vx, byte");
        self.v[x as usize] = kk;
        self.pc += 1;
    }

    // 7xkk
    // Adds the value kk to the value of register Vx, then stores the result in Vx
    fn add_vx_byte(&mut self, x: u8, kk: u8) {
        println!("7xkk - ADD Vx, byte");
        self.v[x as usize] += kk;
        self.pc += 1;
    }

    // 8xy0
    // Stores the value of register Vy in register Vx.
    fn ld_vx_vy(&mut self, x: u8, y: u8) {
        println!("8xy0 - LD Vx, Vy");
        self.v[x as usize] = self.v[y as usize];
        self.pc += 1;
    }

    // 8xy1
    // Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    fn or_vx_vy(&mut self, x: u8, y: u8) {
        println!("8xy1 - OR Vx, Vy");
        self.v[x as usize] |= self.v[y as usize];
        self.pc += 1;
    }

    // 8xy2
    // Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
    fn and_vx_vy(&mut self, x: u8, y: u8) {
        println!("8xy2 - AND Vx, Vy");
        self.v[x as usize] &= self.v[y as usize];
        self.pc += 1;
    }

    // 8xy3
    // Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx.
    fn xor_vx_vy(&mut self, x: u8, y: u8) {
        println!("8xy3 - XOR Vx, Vy");
        self.v[x as usize] ^= self.v[y as usize];
        self.pc += 1;
    }

    // 8xy4
    // The values of Vx and Vy are added together. If the result is greater
    // than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.
    fn add_vx_vy(&mut self, x: u8, y: u8) {
        println!("8xy4 - ADD Vx, Vy");
        let sum = self.v[x as usize] as u16 + self.v[y as usize] as u16;
        self.v[x as usize] = sum as u8;
        self.v[0xF] = (sum > 255) as u8;
        self.pc += 1;
    }

    // 8xy5
    // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is
    // subtracted from Vx, and the results stored in Vx
    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        println!("8xy5 - SUB Vx, Vy");
        self.v[0xf] = (self.v[x as usize] > self.v[y as usize]) as u8;
        self.v[x as usize] -= self.v[y as usize];
        self.pc += 1;
    }

    // 8xy6
    // If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is
    // divided by 2.
    fn shr_vx_vy(&mut self, x: u8) {
        println!("8xy6 - SHR Vx");
        self.v[0xf] = (self.v[x as usize] & 0b1 == 1) as u8;
        self.v[x as usize] >>= 1;
        self.pc += 1;
    }

    // 8xy7
    // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is
    // subtracted from Vy, and the results stored in Vx.
    fn subn_vx_vy(&mut self, x: u8, y: u8) {
        println!("8xy7 - SUBN Vx, Vy");
        self.v[0xf] = (self.v[y as usize] > self.v[x as usize]) as u8;
        self.v[x as usize] = self.v[y as usize] - self.v[x as usize];
        self.pc += 1;
    }

    // 8xyE
    // If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is
    // multiplied by 2.
    fn shl_vx_vy(&mut self, x: u8) {
        println!("8xyE - SHL Vx");
        self.v[0xf] = (self.v[x as usize] >> 7 == 1) as u8;
        self.v[x as usize] <<= 1;
        self.pc += 1;
    }

    // 9xy0
    // The values of Vx and Vy are compared, and if they are not equal, the
    // program counter is increased by 2.
    fn sne_vx_vy(&mut self, x: u8, y: u8) {
        println!("9xy0 - SNE Vx, Vy");
        if self.v[x as usize] != self.v[y as usize] { self.pc += 2; }
    }

    // Annn
    // The value of register I is set to nnn
    fn ld_i_addr(&mut self, nnn: u16) {
        println!("Annn - LD I, addr");
        self.i = nnn;
        self.pc += 1;
    }

    // Bnnn
    // The program counter is set to nnn plus the value of V0.
    fn jp_v0_addr(&mut self, nnn: u16) {
        println!("Bnnn - JP V0, addr");
        self.pc = self.v[0] as u16 + nnn;
    }

    // Cxkk
    // The interpreter generates a random number from 0 to 255, which is then
    // ANDed with the value kk. The results are stored in Vx.
    fn rnd_vx_byte(&mut self, x: u8, kk: u8) {
        println!("Cxkk - RND Vx, byte");
        let random_byte: u8 = rand::thread_rng().gen_range(0..255);
        self.v[x as usize] = random_byte & kk;
        self.pc += 1;
    }

    // Dxyn
    // The interpreter reads n
    // bytes from memory, starting at the address stored in I. These bytes are then displayed as sprites on screen
    // at coordinates (Vx, Vy). Sprites are XOR’d onto the existing screen. If this causes any pixels to be erased,
    // VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it is outside the coordinates of
    // the display, it wraps around to the opposite side of the screen.
    fn drw_vx_vy_nibble(&mut self, x: u8, y: u8, n: u8) {
        println!("Dxyn - DRW Vx, Vy, nibble");
        // let data = self.memory[self.i..self.i+n];
        todo!("Dxyn - drw_vx_vy_nibble")
    }

    // Ex9E
    // Checks the keyboard, and if the key corresponding
    // to the value of Vx is currently in the down position, PC is increased by 2.
    fn skp_vx(&mut self, x: u8) {
        println!("Ex9E - SKP Vx");
        if self.keyboard[self.v[x as usize] as usize] { self.pc += 2; }
    }

    // ExA1
    // Checks the keyboard, and if the key
    // corresponding to the value of Vx is currently in the up position, PC is increased by 2.
    fn sknp_vx(&mut self, x: u8) {
        println!("ExA1 - SKNP Vx");
        if !self.keyboard[self.v[x as usize] as usize] { self.pc += 1; }
    }

    // Fx07
    // The value of DT is placed into Vx.
    fn ld_vx_dt(&mut self, x: u8) {
        println!("Fx07 - LD Vx, DT");
        self.v[x as usize] = self.dt;
        self.pc += 1;
    }

    // Fx0A
    // All execution stops until a key is pressed, then the
    // value of that key is stored in Vx.
    fn ld_vx_k(&mut self, x: u8) {
        let mut key = String::new();
        loop {
            io::stdin().read_line(&mut key).expect("Invalid input");
            let key_value = get_key_value(key.to_uppercase().as_str());
            if key_value != None {
                self.v[x as usize] = key_value.unwrap();
                break;
            }
        }
        self.pc += 2;
    }

    // Fx15
    // Delay Timer is set equal to the value of Vx.
    fn ld_dt_vx(&mut self, x: u8) {
        println!("Fx15 - LD DT, Vx");
        self.dt = self.v[x as usize];
        self.pc += 1;
    }

    // Fx18
    // Sound Timer is set equal to the value of Vx.
    fn ld_st_vx(&mut self, x: u8) {
        println!("Fx18 - LD ST, Vx");
        self.st = self.v[x as usize];
        self.pc += 1;
    }

    // Fx1E
    // The values of I and Vx are added, and the results are stored in I.
    fn add_i_vx(&mut self, x: u8) {
        println!("Fx1E - ADD I, Vx");
        self.i += self.v[x as usize] as u16;
        self.pc += 1;
    }

    // Fx29
    // The value of I is set to the location for the hexadecimal sprite
    // corresponding to the value of Vx.
    fn ld_f_vx(&mut self, x: u8) {
        println!("Fx29 - LD F, Vx");
        todo!("Fx29 - ld_f_vx")
    }

    // Fx33
    // The interpreter takes the decimal
    // value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and
    // the ones digit at location I+2.
    fn ld_b_vx(&mut self, x: u8) {
        println!("Fx33 - LD B, Vx");
        let data = self.v[x as usize];
        self.memory[self.i as usize] = data / 100;
        self.memory[(self.i + 1) as usize] = (data % 100) / 10;
        self.memory[(self.i + 2) as usize] = data % 10;
        self.pc += 1;
    }

    // Fx55
    // Stores V0 to VX in memory starting at address I. I is then set to I + x + 1.
    fn ld_i_vx(&mut self, x: u8) {
        println!("Fx55 - LD [I], Vx");
        for j in 0..x { self.memory[(self.i + j as u16) as usize] = self.v[j as usize]; }
        self.pc += 1;
    }

    // Fx65
    // Fills V0 to VX with values from memory starting at address I. I is then set to I + x + 1.
    fn ld_vx_i(&mut self, x: u8) {
        println!("Fx65 - LD Vx, [I]");
        for j in 0..x { self.v[j as usize] = self.memory[(self.i + j as u16) as usize]; }
        self.pc += 1;
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
fn get_key_value(input: &str) -> Option<u8> {
    match input {
        "1" => Some(1),
        "2" => Some(2),
        "3" => Some(3),
        "4" => Some(0xC),
        "Q" => Some(4),
        "W" => Some(5),
        "E" => Some(6),
        "R" => Some(0xD),
        "A" => Some(7),
        "S" => Some(8),
        "D" => Some(9),
        "F" => Some(0xE),
        "Z" => Some(0xA),
        "X" => Some(0),
        "C" => Some(0xB),
        "V" => Some(0xF),
        _ => None,
    }
}
