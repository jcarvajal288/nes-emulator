use super::olc6502;
use super::bus;

pub struct Nes {
    olc6502: olc6502::Olc6502;
    cpu_bus: bus::Bus;
}