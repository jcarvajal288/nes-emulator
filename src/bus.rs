#![allow(dead_code)]
extern crate hex;

use super::cartridge;

const BUS_RAM_SIZE: usize = 64 * 1024;

pub struct Bus {
    ram: [u8; BUS_RAM_SIZE],
    cartridge: Option<Box<cartridge::Cartridge>>,
}

impl PartialEq for Bus {
    fn eq(&self, other: &Bus) -> bool {
        self.ram[..] == other.ram[..]
    }
}

impl Bus {
    pub fn reset_ram(mut self) {
        for i in 0..self.ram.len() {
            self.ram[i] = 0x00;
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        if addr <= 0x1FFF { 
            // cpu bus has 8k addressable range but only 
            // 2k physical ram, so mirror 2k ram 4 times
            self.ram[usize::from(addr & 0x7FF)] = data;
        } else if addr <= 0x3FFF { // ppu flags
            self.write_to_ppu(addr & 0x0007, data);
        } else if addr >= 0x4020 { // program rom
            self.ram[usize::from(addr)] = data;
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let _read_only: bool = false; // this will be a parameter in the future
        if addr <= 0x1FFF { 
            // cpu bus has 8k addressable range but only 
            // 2k physical ram, so mirror 2k ram 4 times
            return self.ram[usize::from(addr & 0x7FF)];
        } else if addr <= 0x3FFF { // ppu flags
            return self.read_from_ppu(addr & 0x0007);
        } else if addr >= 0x4020 { 
            // program rom
            return self.ram[usize::from(addr)];
        } else {
            return 0x00;
        }
    }

    pub fn load_bytes_at(&mut self, addr: u16, data: String) {
        let bytes: Vec<u8> = data
            .split_whitespace()
            .map(|x| u8::from_str_radix(x, 16).unwrap())
            .collect();
        for (offset, byte) in bytes.iter().enumerate() {
            let abs_addr = addr + (offset as u16);
            self.write(abs_addr, *byte);
        }
    }

    pub fn read_bytes_at(&self, addr: u16, num_bytes: usize) -> String {
        let mut result: Vec<u8> = Vec::new();
        for offset in 0..num_bytes {
            let abs_addr = addr + (offset as u16);
            result.push(self.read(abs_addr));
        }
        return hex::encode_upper(result);
    }

    fn write_to_ppu(&mut self, addr: u16, _data: u8) {
        match addr {
            0x0000 => { // Control
                return;
            }
            0x0001 => { // Mask
                return;
            }
            0x0002 => { // Status
                return;
            }
            0x0003 => { // OAM Address
                return;
            }
            0x0004 => { // OAM Data
                return;
            }
            0x0005 => { // Scroll
                return;
            }
            0x0006 => { // PPU Address
                return;
            }
            0x0007 => { // PPU Data
                return;
            }
            _ => {
                panic!("Invalid addr in bus::write_to_ppu()");
            }
        }
    }

    fn read_from_ppu(&self, addr: u16) -> u8 {
        match addr {
            0x0000 => { // Control
                return 0x0;
            }
            0x0001 => { // Mask
                return 0x1;
            }
            0x0002 => { // Status
                return 0x2;
            }
            0x0003 => { // OAM Address
                return 0x3;
            }
            0x0004 => { // OAM Data
                return 0x4;
            }
            0x0005 => { // Scroll
                return 0x5;
            }
            0x0006 => { // PPU Address
                return 0x6;
            }
            0x0007 => { // PPU Data
                return 0x7;
            }
            _ => {
                panic!("Invalid addr in bus::read_from_ppu()");
            }
        }
    }

    pub fn connect_cartridge(&mut self, cartridge: Box<cartridge::Cartridge>) {
        self.cartridge = Some(cartridge);
    }
}

pub fn create_bus() -> Bus {
    return Bus {
        ram: [0x0; BUS_RAM_SIZE],
        cartridge: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write() {
        let mut b: Bus = create_bus();
        b.write(0x24, 0x20);
        assert!(b.ram[0x24] == 0x20);
    }

    #[test]
    fn test_read() {
        let mut b: Bus = create_bus();
        b.ram[0x24] = 0x20;
        assert!(b.read(0x24) == 0x20);
    }

    #[test]
    fn write_mirrored_address() {
        let mut b: Bus = create_bus();
        b.write(0x1111, 0xEA);
        assert!(b.ram[0x111] == 0xEA);
    }

    #[test]
    fn mirrored_read() {
        let mut b: Bus = create_bus();
        b.ram[0x0] = 0xEA;
        assert!(b.read(0x800) == 0xEA);
    }
}
