#![allow(dead_code)]
use std::fs::File;
use std::io::Read;
use std::convert::TryInto;

pub struct Cartridge {
    header: Header,
}

struct Header {
    name: [u8; 4],
    prg_rom_chunks: u8, // in 16kb units
    chr_rom_chunks: u8, // in 8 kb units (0 means the board uses CHR RAM)
    mapper1: u8,
    mapper2: u8,
    prg_ram_size: u8,
    tv_system1: u8,
    tv_system2: u8,
    unused: [u8; 5], 
}

pub fn create_cartridge_from_file(filename: &str) -> Option<Box<Cartridge>> {
    let mut file_buffer = Vec::new();
    let mut file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            println!("ERROR: File not found: '{}'", filename);
            return None;
        }
    };
    match file.read_to_end(&mut file_buffer) {
        Ok(_) => {}
        Err(_) => {
            println!("ERROR: File unreadable: '{}'", filename);
            return None;
        }
    }

    return Some(Box::new(Cartridge {
        header: read_header(&file_buffer)
    }))
}

fn read_header(file_buffer: &Vec<u8>) -> Header {
    return Header {
        name: file_buffer[0..4].try_into().unwrap(),
        prg_rom_chunks: file_buffer[4],
        chr_rom_chunks: file_buffer[5],
        mapper1: file_buffer[6],
        mapper2: file_buffer[7],
        prg_ram_size: file_buffer[8],
        tv_system1: file_buffer[9],
        tv_system2: file_buffer[10],
        unused: file_buffer[11..16].try_into().unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_header_read() {
        let filename = "./test_files/nestest.nes";
        let mut file_buffer = Vec::new();
        let mut file = File::open(filename).unwrap();
        file.read_to_end(&mut file_buffer).unwrap();
        let header: Header = read_header(&file_buffer);

        assert!(header.name == [0x4E, 0x45, 0x53, 0x1A]);
        assert!(header.prg_rom_chunks == 0x01);
        assert!(header.chr_rom_chunks == 0x01);
        assert!(header.mapper1 == 0x00);
        assert!(header.mapper2 == 0x00);
        assert!(header.prg_ram_size == 0x00);
        assert!(header.tv_system1 == 0x00);
        assert!(header.tv_system2 == 0x00);
        assert!(header.unused == [0x00, 0x00, 0x00, 0x00, 0x00]);
    }
}