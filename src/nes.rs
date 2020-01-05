use super::olc6502;
use super::bus;

pub struct Nes {
    cpu_bus: bus::Bus,
    cpu: olc6502::Olc6502,
}

pub fn create_nes() -> Nes {
    let bus = bus::create_bus();
    let nes = Nes {
        cpu_bus: bus,
        cpu: olc6502::create_olc6502(&bus),
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
        nes.cpu_bus.load_bytes_at(0x8000, assembled_source);
        nes.cpu.run_program();
        assert!(nes.cpu_bus.read(0x0201) == 0x03);
    }
}
