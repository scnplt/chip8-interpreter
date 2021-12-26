use crate::chip8::Chip8;

mod chip8;
mod keypad;

fn main() {
    let path = "./roms/Tetris [Fran Dachille, 1991].ch8";
    let sdl = sdl2::init().expect("Could not create SDL!");
    let mut chip8 = Chip8::new(&sdl);
    chip8.load_rom(path);
    chip8.start_cycle(&mut sdl.event_pump().expect("Event Issue"));
}
