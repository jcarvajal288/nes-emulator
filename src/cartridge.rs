#![allow(dead_code)]
use std::fs::File;
use std::io::Read;
use std::convert::TryInto;

const PROGRAM_ROM_CHUNK_SIZE: usize = 16384;
const CHARACTER_ROM_CHUNK_SIZE: usize = 8192;

pub struct Cartridge {
    header: Header,
    mapper_id: u8,
    program_rom: Vec<u8>,
    character_rom: Vec<u8>,
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
    let file_buffer: Vec<u8> = match read_rom_file(filename) {
        Ok(file_buffer) => file_buffer,
        Err(e) => { 
            println!("{}", e);
            return None
        }
    };
    let header = read_header(&file_buffer);
    // the mapper id is (upper nybble of mapper2 | lower nybble of mapper1)
    let mapper_id = ((header.mapper2 >> 4) << 4) | (header.mapper1 >> 4);

    let has_trainer_block = header.mapper1 & 0x04 > 1;
    let prg_starting_index = if has_trainer_block { 528 } else { 16 };

    // read in program memory and character memory
    let mut program_rom: Vec<u8> = Vec::new();
    let mut character_rom: Vec<u8> = Vec::new();
    let file_type: u8 = 1; // three types of file types.  only concerned with 1 so far
    match file_type {
        0 => { /* placeholder */ }
        1 => {
            let program_rom_len = (header.prg_rom_chunks as usize) * PROGRAM_ROM_CHUNK_SIZE;
            let prg_ending_index = prg_starting_index + program_rom_len as usize;
            program_rom = file_buffer[prg_starting_index..prg_ending_index].try_into().unwrap();

            let character_rom_len = (header.chr_rom_chunks as usize) * CHARACTER_ROM_CHUNK_SIZE;
            let chr_ending_index = prg_ending_index + character_rom_len as usize;
            character_rom = file_buffer[prg_ending_index..chr_ending_index].try_into().unwrap();
        }
        2 => { /* placeholder */ }
        _ => { panic!("ERROR: Unrecognized file type in rom read.")}
    }

    return Some(Box::new(Cartridge {
        header: header,
        mapper_id: mapper_id,
        program_rom: program_rom,
        character_rom: character_rom,
    }))
}

fn read_rom_file(filename: &str) -> Result<Vec<u8>, String> {
    let mut file_buffer = Vec::new();
    let mut file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            let error_message = format!("ERROR: File not found: '{}'", filename);
            return Err(error_message);
        }
    };
    match file.read_to_end(&mut file_buffer) {
        Ok(_) => {}
        Err(e) => {
            let error_message = format!("{}", e);
            return Err(error_message);
        }
    }
    return Ok(file_buffer);
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
        let file_buffer = read_rom_file(filename).unwrap();
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

    #[test]
    fn verify_program_rom_read() {
        let filename = "./test_files/nestest.nes";
        let cartridge: Box<Cartridge> = create_cartridge_from_file(filename).unwrap();

        assert!(cartridge.program_rom.len() == PROGRAM_ROM_CHUNK_SIZE);
        assert!(cartridge.program_rom.first() == Some(&0x4C));
        assert!(cartridge.program_rom.last() == Some(&0xC5));
    }

    #[test]
    fn verify_character_rom_read() {
        let filename = "./test_files/nestest.nes";
        let cartridge: Box<Cartridge> = create_cartridge_from_file(filename).unwrap();

        assert!(cartridge.character_rom.len() == CHARACTER_ROM_CHUNK_SIZE);
        assert!(cartridge.character_rom.first() == Some(&0x00));
        assert!(cartridge.character_rom.last() == Some(&0x00));
    }
}