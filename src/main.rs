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
                .required(true),
            Arg::with_name("delay")
                .short("d")
                .long("delay")
                .value_name("DELAY (ms)")
                .help("Inter-cycle delay time")
                .default_value("2")
                .takes_value(true)
                .empty_values(false)
                .multiple(false)
        ]).get_matches();

    let rom_path = matches.value_of("rom_path").expect("Args error!").trim();
    let delay = matches.value_of("delay").expect("Args error!").parse::<u64>();

    if delay.is_err() {
        eprint!("Error: That's not a number!");
        return;
    }

    let sdl = sdl2::init().expect("Could not create SDL!");
    let mut chip = Chip8::new(&sdl);
    chip.load_rom(rom_path);
    chip.start_cycle(delay.unwrap());
}
