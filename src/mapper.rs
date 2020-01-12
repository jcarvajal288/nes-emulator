#![allow(dead_code)]

// Mapper documentation: http://wiki.nesdev.com/w/index.php/Mapper

pub trait Mapper {
    fn map_address(&self, input_addr: u16) -> u32;
}

pub struct NROM {
    num_prg_banks: u8,
    num_chr_banks: u8,
}

impl Mapper for NROM {

    fn map_address(&self, input_addr: u16) -> u32 {
        if self.num_prg_banks > 1 {
            return (input_addr & 0x7FFF) as u32;
        } else {
            return (input_addr & 0x3FFF) as u32;
        }
    }
}

pub fn create_mapper(mapper_id: u8, num_prg_banks: u8, num_chr_banks: u8) -> Box<dyn Mapper> {
    match mapper_id {
        0 => {
            return Box::new(NROM {
                num_prg_banks: num_prg_banks,
                num_chr_banks: num_chr_banks,
            });
        }
        _ => {
            panic!("Unimplemented mapper id {}", mapper_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod nrom {
        use super::*;

        #[test]
        fn map_16k() {
            let nrom = create_mapper(0, 1, 1);
            assert!(nrom.map_address(0x0000) == 0x0000);
            assert!(nrom.map_address(0x1000) == 0x1000);
            assert!(nrom.map_address(0x2000) == 0x2000);
            assert!(nrom.map_address(0x3000) == 0x3000);
            assert!(nrom.map_address(0x4000) == 0x0000);
            assert!(nrom.map_address(0x5000) == 0x1000);
            assert!(nrom.map_address(0x6000) == 0x2000);
            assert!(nrom.map_address(0x7000) == 0x3000);
            assert!(nrom.map_address(0x8000) == 0x0000);
            assert!(nrom.map_address(0x9000) == 0x1000);
            assert!(nrom.map_address(0xA000) == 0x2000);
            assert!(nrom.map_address(0xB000) == 0x3000);
            assert!(nrom.map_address(0xC000) == 0x0000);
            assert!(nrom.map_address(0xD000) == 0x1000);
            assert!(nrom.map_address(0xE000) == 0x2000);
            assert!(nrom.map_address(0xF000) == 0x3000);
        }

        #[test]
        fn map_32k() {
            let nrom = create_mapper(0, 2, 1);
            assert!(nrom.map_address(0x0000) == 0x0000);
            assert!(nrom.map_address(0x1000) == 0x1000);
            assert!(nrom.map_address(0x2000) == 0x2000);
            assert!(nrom.map_address(0x3000) == 0x3000);
            assert!(nrom.map_address(0x4000) == 0x4000);
            assert!(nrom.map_address(0x5000) == 0x5000);
            assert!(nrom.map_address(0x6000) == 0x6000);
            assert!(nrom.map_address(0x7000) == 0x7000);
            assert!(nrom.map_address(0x8000) == 0x0000);
            assert!(nrom.map_address(0x9000) == 0x1000);
            assert!(nrom.map_address(0xA000) == 0x2000);
            assert!(nrom.map_address(0xB000) == 0x3000);
            assert!(nrom.map_address(0xC000) == 0x4000);
            assert!(nrom.map_address(0xD000) == 0x5000);
            assert!(nrom.map_address(0xE000) == 0x6000);
            assert!(nrom.map_address(0xF000) == 0x7000);
        }
    }
}