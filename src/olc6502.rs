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

    lookup: [Instruction; 256],
}


impl Olc6502 {
    fn read(&self, addr: u16) -> u8 {
        return self.bus.read(addr);
    }

    fn write(mut self, addr: u16, data: u8) {
        self.bus.write(addr, data);
    }

    //fn get_flag(flag: Flags6502) -> u8 { }

    fn set_flag(flag: Flags6502, v: bool) {

    }

    // Addressing Modes
    // region
    fn ACC() -> u8 { // Accumulator Addressing
        return 0x0;
    }

    fn IMM() -> u8 { // Immediate
        return 0x0; 
    }

    fn ABS() -> u8 { // Absolute Addressing
        return 0x0; 
    }

    fn ZP0() -> u8 { // Zero Page Addressing
        return 0x0; 
    }

    fn ZPX() -> u8 { // Indexed Zero Page Addressing X
        return 0x0; 
    }

    fn ZPY() -> u8 { // Indexed Zero Page Addressing Y
        return 0x0; 
    }

    fn ABX() -> u8 { // Indexed Absolute Adressing X
        return 0x0; 
    }

    fn ABY() -> u8 { // Indexed Absolute Adressing Y
        return 0x0; 
    }

    fn IMP() -> u8 { // Implied
        return 0x0; 
    }

    fn REL() -> u8 { // Relative Adressing
        return 0x0; 
    }

    fn IZX() -> u8 { // Indexed Indirect Addressing X
        return 0x0; 
    }

    fn IZY() -> u8 { // Indirect Indexed Addressing Y
        return 0x0; 
    }

    fn IND() -> u8 { // Absolute Indirect
        return 0x0; 
    }
    // endregion

    // Opcodes
    // region
    fn ADC() -> u8 { // Add Memory to Accumulator with Carry
        return 0x0; 
    }

    fn AND() -> u8 { // "AND" Memory with Accumulator
        return 0x0; 
    }

    fn ASL() -> u8 { // Shift Left One Bit (Memory or Accumulator)
        return 0x0; 
    }

    fn BCC() -> u8 { // Branch on Carry Clear
        return 0x0; 
    }

    fn BCS() -> u8 { // Branch on Carry Set
        return 0x0; 
    }

    fn BEQ() -> u8 { // Branch on Result Zero
        return 0x0; 
    }

    fn BIT() -> u8 { // Test Bits in Memory with Accumulator
        return 0x0; 
    }

    fn BMI() -> u8 { // Branch on Result Minus
        return 0x0; 
    }

    fn BNE() -> u8 { // Branch on Result not Zero
        return 0x0; 
    }

    fn BPL() -> u8 { // Branch on Result Plus
        return 0x0; 
    }

    fn BRK() -> u8 { // Force Break
        return 0x0; 
    }

    fn BVC() -> u8 { // Branch on Overflow Clear
        return 0x0; 
    }

    fn BVS() -> u8 { // Branch on Overflow Set
        return 0x0; 
    }

    fn CLC() -> u8 { // Clear Carry Flag
        return 0x0; 
    }

    fn CLD() -> u8 { // Clear Decimal Mode
        return 0x0; 
    }

    fn CLI() -> u8 { // Clear Interrupt Disable Bit
        return 0x0; 
    }

    fn CLV() -> u8 { // Clear Overflow Flag
        return 0x0; 
    }

    fn CMP() -> u8 { // Compare Memory And Accumulator
        return 0x0; 
    }

    fn CPX() -> u8 { // Compare Memory and Index X
        return 0x0; 
    }

    fn CPY() -> u8 { // Compare Memory And Index Y
        return 0x0; 
    }

    fn DEC() -> u8 { // Decrement Memory by One
        return 0x0; 
    }

    fn DEX() -> u8 { // Decrement Index X by One
        return 0x0; 
    }

    fn DEY() -> u8 { // Decrement Index Y by One
        return 0x0; 
    }

    fn EOR() -> u8 { // "Exclusive-OR" Memory with Accumulator
        return 0x0; 
    }

    fn INC() -> u8 { // Increment Memory by One
        return 0x0; 
    }

    fn INX() -> u8 { // Increment Index X by One
        return 0x0; 
    }

    fn INY() -> u8 { // Increment Index Y by One
        return 0x0; 
    }

    fn JMP() -> u8 { // Jump to New Location
        return 0x0; 
    }

    fn JSR() -> u8 { // Jump to New Location Saving Return Address
        return 0x0; 
    }

    fn LDA() -> u8 { // Load Accumulator with Memory
        return 0x0; 
    }

    fn LDX() -> u8 { // Load Index X with Memory
        return 0x0; 
    }

    fn LDY() -> u8 { // Load Index Y with Memory
        return 0x0; 
    }

    fn LSR() -> u8 { // Shift One Bit Right (Memory or Accumulator)
        return 0x0; 
    }

    fn NOP() -> u8 { // No Operation
        return 0x0; 
    }

    fn ORA() -> u8 { // "OR" Memory with Accumulator
        return 0x0; 
    }

    fn PHA() -> u8 { // Push Accumulator on Stack
        return 0x0; 
    }

    fn PHP() -> u8 { // Push Processor Status on Stack
        return 0x0; 
    }

    fn PLA() -> u8 { // Pull Accumulator from Stack
        return 0x0; 
    }

    fn PLP() -> u8 { // Pull Processor Status from Stack
        return 0x0; 
    }

    fn ROL() -> u8 { // Rotate One Bit Left (Memory or Accumulator)
        return 0x0; 
    }

    fn ROR() -> u8 { // Rotate One Bit Right (Memory or Accumulator)
        return 0x0; 
    }

    fn RTI() -> u8 { // Return from Interrupt
        return 0x0; 
    }

    fn RTS() -> u8 { // Return from Subroutine
        return 0x0; 
    }

    fn SBC() -> u8 { // Subtract Memory from Accumulator with Borrow
        return 0x0; 
    }

    fn SEC() -> u8 { // Set Carry Flag
        return 0x0; 
    }

    fn SED() -> u8 { // Set Decimal Mode (unused)
        return 0x0; 
    }

    fn SEI() -> u8 { // Set Interrupt Disable Status
        return 0x0; 
    }

    fn STA() -> u8 { // Store Accumulator in Memory
        return 0x0; 
    }

    fn STX() -> u8 { // Store Index X in Memory
        return 0x0; 
    }

    fn STY() -> u8 { // Store Index Y in Memory
        return 0x0; 
    }

    fn TAX() -> u8 { // Transfer Accumulator to Index X
        return 0x0; 
    }

    fn TAY() -> u8 { // Transfer Accumulator to Index Y
        return 0x0; 
    }

    fn TSX() -> u8 { // Transfer Stack Pointer to Index X
        return 0x0; 
    }

    fn TXA() -> u8 { // Transfer Index X to Accumulator
        return 0x0; 
    }

    fn TXS() -> u8 { // Transfer Index X to Stack Register
        return 0x0; 
    }

    fn TYA() -> u8 { // Transfer Index Y to Accumulator
        return 0x0; 
    }

    fn XXX() -> u8 { // Undefined Instruction
        return 0x0; 
    }
    // endregion

    fn populate_lookup_table(&self) {
        type A = Olc6502;
        fn i(name: &str, operate: fn() -> u8, addrmode: fn() -> u8, cycles: u8) -> Instruction {
            return Instruction { name: String::from(name), operate, addrmode, cycles };
        }

        self.lookup = [
            i("BRK", A::BRK, A::IMP, 7), i("ORA", A::ORA, A::IZX, 6), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("ORA", A::ORA, A::ZP0, 3), i("ASL", A::ASL, A::ZP0, 5), i("???", A::XXX, A::IMP, 2),  i("PHP" A::PHP, A::IMP, 3), i("ORA", A::ORA, A::IMM, 2), i("ASL", A::ASL, A::ACC, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("ORA", A::ORA, A::ABS, 4), i("ASL", A::XXX, A::ABS, 6), i("???", A::XXX, A::IMP, 2),
            i("BPL", A::BPL, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("JSR", A::JSR, A::ABS, 6), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("BMI", A::BMI, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("RTI", A::RTI, A::IMP, 6), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("BVC", A::BVC, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("RTS", A::RTS, A::IMP, 6), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("BVS", A::BVS, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("BCC", A::BCC, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("LDY", A::LDY, A::IMM, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("BCS", A::BCS, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("CPY", A::CPY, A::IMM, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("BNE", A::BNE, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("CPX", A::CPX, A::IMM, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
            i("BEQ", A::BEQ, A::REL, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), i("???", A::XXX, A::IMP, 2), 
        ];

    }

    fn clock(&self) {
        if self.cycles == 0 {
            self.opcode = self.read(u16::from(self.prog_ctr));
            self.prog_ctr += 1;

            // Get starting number of cycles
            let op_index = usize::from(self.opcode);
            self.cycles = self.lookup[op_index].cycles;
            (self.lookup[op_index].addrmode)();
            (self.lookup[op_index].operate)();
            // at 29:10
        }
    }
    // fn reset() {}
    // fn irq() {}
    // fn nmi() {}

    // fn fetch -> u8 {}
}

struct Instruction {
    name: String,
    operate: fn() -> u8,
    addrmode: fn() -> u8,
    cycles: u8,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
