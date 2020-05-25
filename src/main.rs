mod bus;
mod cartridge;
mod mapper;
mod logline;
mod nes;
#[allow(non_snake_case)]
mod olc2C02;
mod olc6502;
mod renderer;

#[macro_use] extern crate lazy_static;

fn main() {
	let mut nes = nes::create_nes();
	loop {
		nes.clock();
	}
}
