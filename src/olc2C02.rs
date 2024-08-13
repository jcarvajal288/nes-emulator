#![allow(dead_code)]
use super::olc6502;

const NAMETABLE_SIZE: usize = 1024;
const NUM_NAMETABLES: usize = 2;
const NUM_PALLETES: usize = 32;

pub struct Olc2C02 {
    pub cpu: olc6502::Olc6502,

    nametables: [[u8; NAMETABLE_SIZE]; NUM_NAMETABLES],
    palettes: [u8; NUM_PALLETES],

    scanline: i16,
    cycle: i16,
    frame_complete: bool,
}

impl Olc2C02 {
    
    pub fn clock(&mut self) {
        
        // TODO: set pixel here

        self.cycle += 1;
        if self.cycle >= 341 {
            self.cycle = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                self.scanline = -1;
                self.frame_complete = true;
            }
        }
    }

    fn cpu_read(&self, addr: u16) -> u8 {
        return self.cpu.bus.read(addr);
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        self.cpu.bus.write(addr, data);
    }
}

#[allow(non_snake_case)]
pub fn create_olc2C02() -> Olc2C02 {
    return Olc2C02 {
        cpu: olc6502::create_olc6502(),
        nametables: [[0x0; NAMETABLE_SIZE]; NUM_NAMETABLES],
        palettes: [0x0; NUM_PALLETES],
        scanline: 0,
        cycle: 0,
        frame_complete: false,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_to_and_read_from_cpu_bus() {
        let mut ppu = create_olc2C02();
        ppu.cpu_write(0x24, 0x20);
        assert_eq!(ppu.cpu_read(0x24), 0x20);
    }
}