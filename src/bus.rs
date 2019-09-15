//use super::olc6502;

pub struct Bus {
    //cpu: olc6502::Olc6502,
    ram: [u8; 64 * 1024],
}

impl Bus {
    pub fn reset_ram(mut self) {
        for i in 0..self.ram.len() {
            self.ram[i] = 0x00;
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        if addr >= 0x0000 && addr <= 0xFFFF {
            self.ram[usize::from(addr)] = data;
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let read_only: bool = false; // this will be a parameter in the future

        if addr >= 0x0000 && addr <= 0xFFFF {
            return self.ram[usize::from(addr)];
        }
        return 0x00;
    }
}

pub fn create_Bus() -> Bus {
    return Bus {
        ram: [0x0; 64 * 1024],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write() {
        let mut b: Bus = create_Bus();
        b.write(0x24, 0x20);
        assert!(b.ram[0x24] == 0x20);
    }
}
