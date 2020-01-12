#![allow(dead_code)]
use super::cartridge;
use super::olc2C02;

pub struct Nes {
    ppu: olc2C02::Olc2C02,
}

impl Nes {
    // System Interface
    pub fn load_rom(&mut self, filename: &str) {
        let cartridge = cartridge::create_cartridge_from_file(filename).unwrap();
        self.ppu.cpu.bus.connect_cartridge(cartridge);
    }

    pub fn reset() {

    }

    pub fn clock() {

    }

    // test functions
    fn load_program(&mut self, program: String) {
        return self.ppu.cpu.load_program(program);
    }

    fn run_program(&mut self) {
        self.ppu.cpu.run_program();
    }

    fn read_cpu_address(self, addr: u16) -> u8 {
        return self.ppu.cpu.bus.read(addr);
    }
}

pub fn create_nes() -> Nes {
    let nes = Nes {
        ppu: olc2C02::create_olc2C02(),
    };
    return nes;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_and_run_program() {
        // duplicates short_loop test from olc6502.  This is to
        // test that the emulator's cpu and memory are wired up properly
        /* Program listing
          *=$8000
          LDX #$08
          decrement:
          DEX
          STX $0200
          CPX #$03
          BNE decrement
          STX $0201
          NOP
          NOP
          NOP
        */
        let assembled_source: String = "A2 08 CA 8E 00 02 E0 03 D0 F8 8E 01 02 EA EA EA".to_string();
        let mut nes = create_nes();
        nes.load_program(assembled_source);
        nes.run_program();
        assert!(nes.read_cpu_address(0x0201) == 0x03);
    }

    #[test]
    fn read_from_cartridge() {
        let mut nes = create_nes();
        nes.load_rom("./test_files/nestest.nes");
        let result = nes.read_cpu_address(0x8000);
        assert!(result == 0x4C);
    }

    #[test]
    fn run_nestest_regular_opcodes() {
        let mut nes = create_nes();
        nes.load_rom("./test_files/nestest.nes");
        nes.ppu.cpu.run_automation();
        let result = nes.read_cpu_address(0x02);
        if result != 0x00 {
            println!("Failure code: {}", result);
        }
        assert!(result == 0x00);
    }
}
