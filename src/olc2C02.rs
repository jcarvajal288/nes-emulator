#![allow(dead_code)]
use super::olc6502;

pub struct Olc2C02 {
    pub cpu: olc6502::Olc6502,
}

impl Olc2C02 {
    fn cpu_read(self, addr: u16) -> u8 {
        return self.cpu.bus.read(addr);
    }

    fn cpu_write(mut self, addr: u16, data: u8) {
        self.cpu.bus.write(addr, data);
    }
}

#[allow(non_snake_case)]
pub fn create_olc2C02() -> Olc2C02 {
    return Olc2C02 {
        cpu: olc6502::create_olc6502(),
    };
}