use super::olc6502;

pub struct Bus {
    cpu: olc6502::Olc6502,
    ram: [u8; 64 * 1024],
}

impl Bus {
    fn write(&self, addr: u16, data: u8) {
        if addr >= 0x0000 && addr <= 0xFFFF {
            self.ram[usize::from(addr)] = data;
        }
    }

    fn read(&self, addr: u16) -> u8 {
        let read_only: bool = false; // this will be a parameter in the future

        if addr >= 0x0000 && addr <= 0xFFFF {
            return self.ram[usize::from(addr)];
        }
        return 0x00;
    }
}