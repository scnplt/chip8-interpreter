use clap::{App, Arg};

use crate::chip8::Chip8;

mod chip8;
mod keypad;

fn main() {
    let matches = App::new(chip8::WINDOW_TITLE)
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .usage("chip8-interpreter [OPTIONS]")
        .args(&[
            Arg::with_name("rom_path")
                .short("r")
                .long("rom")
                .value_name("ROM_PATH")
                .help("Sets a custom ch8 rom")
                .takes_value(true)
                .empty_values(false)
                .multiple(false)
                .required(true)
        ]).get_matches();

    let rom_path = matches.value_of("rom_path").expect("Args error!").trim();
    let sdl = sdl2::init().expect("Could not create SDL!");
    let mut chip = Chip8::new(&sdl);
    
    chip.load_rom(rom_path);
    chip.start_cycle();
}
