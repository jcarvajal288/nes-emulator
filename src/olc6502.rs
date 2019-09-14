use super::bus;

enum Flags6502 {
    C = (1 << 0), // Carry Bit
    Z = (1 << 1), // Zero
    I = (1 << 2), // Disable Interrupts
    D = (1 << 3), // Decimal Mode (unused in nes)
    B = (1 << 4), // Break
    U = (1 << 5), // Unused
    V = (1 << 6), // Overflow
    N = (1 << 7), // Negative
}

pub struct Olc6502 {
    flags: Flags6502,
    accumulator: u8,
    x_reg: u8,
    y_reg: u8,
    stack_ptr: u8,
    prog_ctr: u8,
    status_reg: u8,

    bus: bus::Bus,

    fetched_data: u8,
    addr_abs: u16,
    addr_rel: u16,
    opcode: u8,
    cycles: u8,

    lookup: Vec<Instruction>,
}

struct Instruction {
    name: String,
    operate: fn(),
    addrmode: fn(),
    cycles: u8,
}

impl Olc6502 {
    fn read(&self, addr: u16) -> u8 {
        return self.bus.read(addr);
    }

    fn write(mut self, addr: u16, data: u8) {
        self.bus.write(addr, data);
    }

    fn get_flag(flag: Flags6502) -> u8 {

    }

    fn set_flag(flag: Flags6502, v: bool) {

    }

    // Addressing Modes
//    fn IMP() -> u8 {}
//    fn IMM() -> u8 {}
//    fn ZP0() -> u8 {}
//    fn ZPX() -> u8 {}
//    fn ZPY() -> u8 {}
//    fn REL() -> u8 {}
//    fn ABS() -> u8 {}
//    fn ABX() -> u8 {}
//    fn ABY() -> u8 {}
//    fn IND() -> u8 {}
//    fn IZX() -> u8 {}
//    fn IZY() -> u8 {}

    // Opcodes
    // ...

    fn clock(&self) {
        if self.cycles == 0 {
            self.opcode = self.read(u16::from(self.prog_ctr));
            self.prog_ctr += 1;

            // Get starting number of cycles
            let op_index = usize::from(self.opcode);
            self.cycles = self.lookup[op_index].cycles;
            (self.lookup[op_index].addrmode)();
            (self.lookup[op_index].operate)();
        }
    }
    // fn reset() {}
    // fn irq() {}
    // fn nmi() {}

    // fn fetch -> u8 {}
}

// fn populate_lookup_table() -> Vec<Instruction> {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
