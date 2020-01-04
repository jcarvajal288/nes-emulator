#![allow(dead_code)]
extern crate hex;

pub struct Bus {
    ram: [u8; 64 * 1024],
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
        self.ram[usize::from(addr)] = data;
    }

    pub fn read(&self, addr: u16) -> u8 {
        let _read_only: bool = false; // this will be a parameter in the future
        return self.ram[usize::from(addr)];
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
}

pub fn create_bus() -> Bus {
    return Bus {
        ram: [0x0; 64 * 1024],
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
}
