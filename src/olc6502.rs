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
    prog_ctr: u16,
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

    fn populate_lookup_table(mut self) {
        let o = self;
        fn i(name: &str, operate: fn(&mut Olc6502) -> u8, addrmode: fn(&mut Olc6502) -> u8, cycles: u8) -> Instruction {
            return Instruction { name: String::from(name), operate, addrmode, cycles };
        }

        self.lookup = [
            i("BRK", BRK, IMP, 7), i("ORA", ORA, IZX, 6), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("ORA", ORA, ZP0, 3), i("ASL", ASL, ZP0, 5), i("???", XXX, IMP, 2), i("PHP", PHP, IMP, 3), i("ORA", ORA, IMM, 2), i("ASL", ASL, ACC, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("ORA", ORA, ABS, 4), i("ASL", ASL, ABS, 6), i("???", XXX, IMP, 2),
            i("BPL", BPL, REL, 2), i("ORA", ORA, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("ORA", ORA, ZPX, 4), i("ASL", ASL, ZPX, 6), i("???", XXX, IMP, 2), i("CLC", CLC, IMP, 2), i("ORA", ORA, ABY, 4), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("ORA", ORA, ABX, 4), i("ASL", ASL, ABX, 7), i("???", XXX, IMP, 2), 
            i("JSR", JSR, ABS, 6), i("AND", AND, IZX, 6), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("BIT", BIT, ZP0, 3), i("AND", AND, ZP0, 3), i("ROL", ROL, ZP0, 5), i("???", XXX, IMP, 2), i("PLP", PLP, IMP, 4), i("AND", AND, IMM, 2), i("ROL", ROL, ACC, 2), i("???", XXX, IMP, 2), i("BIT", BIT, ABS, 4), i("AND", AND, ABS, 4), i("ROL", ROL, ABS, 6), i("???", XXX, IMP, 2), 
            i("BMI", BMI, REL, 2), i("AND", AND, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("AND", AND, ZPX, 4), i("ROL", ROL, ZPX, 6), i("???", XXX, IMP, 2), i("SEC", SEC, IMP, 2), i("AND", AND, ABY, 4), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("AND", AND, ABX, 4), i("ROL", ROL, ABX, 7), i("???", XXX, IMP, 2), 
            i("RTI", RTI, IMP, 6), i("EOR", EOR, IZX, 6), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("EOR", EOR, ZP0, 3), i("LSR", LSR, ZP0, 5), i("???", XXX, IMP, 2), i("PHA", PHA, IMP, 3), i("EOR", EOR, IMM, 2), i("LSR", LSR, ACC, 2), i("???", XXX, IMP, 2), i("JMP", JMP, ABS, 3), i("EOR", EOR, ABS, 4), i("LSR", LSR, ABS, 6), i("???", XXX, IMP, 2), 
            i("BVC", BVC, REL, 2), i("EOR", EOR, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("EOR", EOR, ZPX, 4), i("LSR", LSR, ZPX, 6), i("???", XXX, IMP, 2), i("CLI", CLI, IMP, 2), i("EOR", EOR, ABY, 4), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("EOR", EOR, ABX, 4), i("LSR", LSR, ABX, 7), i("???", XXX, IMP, 2), 
            i("RTS", RTS, IMP, 6), i("ADC", ADC, IZX, 6), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("ADC", ADC, ZP0, 3), i("ROR", ROR, ZP0, 5), i("???", XXX, IMP, 2), i("PLA", PLA, IMP, 4), i("ADC", ADC, IMM, 2), i("ROR", ROR, ACC, 2), i("???", XXX, IMP, 2), i("JMP", JMP, IND, 5), i("ADC", ADC, ABS, 4), i("ROR", ROR, ABS, 6), i("???", XXX, IMP, 2), 
            i("BVS", BVS, REL, 2), i("ADC", ADC, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("ADC", ADC, ZPX, 4), i("ROR", ROR, ZPX, 6), i("???", XXX, IMP, 2), i("SEI", SEI, IMP, 2), i("ADC", ADC, ABY, 4), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("ADC", ADC, ABX, 4), i("ROR", ROR, ABX, 7), i("???", XXX, IMP, 2), 
            i("???", XXX, IMP, 2), i("STA", STA, IZX, 6), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("STY", STY, ZP0, 3), i("STA", STA, ZP0, 3), i("STX", STX, ZP0, 3), i("???", XXX, IMP, 2), i("DEY", DEY, IMP, 2), i("???", XXX, IMP, 2), i("TXA", TXA, IMP, 2), i("???", XXX, IMP, 2), i("STY", STY, ABS, 4), i("STA", STA, ABS, 4), i("STX", STX, ABS, 4), i("???", XXX, IMP, 2), 
            i("BCC", BCC, REL, 2), i("STA", STA, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("STY", STY, ZPX, 4), i("STA", STA, ZPX, 4), i("STX", STX, ZPY, 4), i("???", XXX, IMP, 2), i("TYA", TYA, IMP, 2), i("STA", STA, ABY, 5), i("TXS", TXS, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("STA", STA, ABX, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), 
            i("LDY", LDY, IMM, 2), i("LDA", LDA, IZX, 6), i("LDX", LDX, IMM, 2), i("???", XXX, IMP, 2), i("LDY", LDY, ZP0, 3), i("LDA", LDA, ZP0, 3), i("LDX", LDX, ZP0, 3), i("???", XXX, IMP, 2), i("TAY", TAY, IMP, 2), i("LDA", LDA, IMM, 2), i("TAX", TAX, IMP, 2), i("???", XXX, IMP, 2), i("LDY", LDY, ABS, 4), i("LDA", LDA, ABS, 4), i("LDX", LDX, ABS, 4), i("???", XXX, IMP, 2), 
            i("BCS", BCS, REL, 2), i("LDA", LDA, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("LDY", LDY, ZPX, 4), i("LDA", LDA, ZPX, 4), i("LDX", LDX, ZPY, 4), i("???", XXX, IMP, 2), i("CLV", CLV, IMP, 2), i("LDA", LDA, ABY, 4), i("TSX", TSX, IMP, 2), i("???", XXX, IMP, 2), i("LDY", LDY, ABX, 4), i("LDA", LDA, ABX, 4), i("LDX", LDX, ABY, 4), i("???", XXX, IMP, 2), 
            i("CPY", CPY, IMM, 2), i("CMP", CMP, IZX, 6), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("CPY", CPY, ZP0, 3), i("CMP", CMP, ZP0, 3), i("DEC", DEC, ZP0, 5), i("???", XXX, IMP, 2), i("INY", INY, IMP, 2), i("CMP", CMP, IMM, 2), i("DEX", DEX, IMP, 2), i("???", XXX, IMP, 2), i("CPY", CPY, ABS, 4), i("CMP", CMP, ABS, 4), i("DEC", DEC, ABS, 6), i("???", XXX, IMP, 2), 
            i("BNE", BNE, REL, 2), i("CMP", CMP, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("CMP", CMP, ZPX, 4), i("DEC", DEC, ZPX, 6), i("???", XXX, IMP, 2), i("CLD", CLD, IMP, 2), i("CMP", CMP, ABY, 4), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("CMP", CMP, ABX, 4), i("DEC", DEC, ABX, 7), i("???", XXX, IMP, 2), 
            i("CPX", CPX, IMM, 2), i("SBC", SBC, IZX, 6), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("CPX", CPX, ZP0, 3), i("SBC", SBC, ZP0, 3), i("INC", INC, ZP0, 5), i("???", XXX, IMP, 2), i("INX", INX, IMP, 2), i("SBC", SBC, IMM, 2), i("NOP", NOP, IMP, 2), i("???", XXX, IMP, 2), i("CPX", CPX, ABS, 4), i("SBC", SBC, ABS, 4), i("INC", INC, ABS, 6), i("???", XXX, IMP, 2), 
            i("BEQ", BEQ, REL, 2), i("SBC", SBC, IZY, 5), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("SBC", SBC, ZPX, 4), i("INC", INC, ZPX, 6), i("???", XXX, IMP, 2), i("SED", SED, IMP, 2), i("SBC", SBC, ABY, 4), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("???", XXX, IMP, 2), i("SBC", SBC, ABX, 4), i("INC", INC, ABX, 7), i("???", XXX, IMP, 2), 
        ];

    }

    fn clock(mut self) {
        if self.cycles == 0 {
            self.opcode = self.read(self.prog_ctr);
            self.prog_ctr += 1;

            // Get starting number of cycles
            let op_index = usize::from(self.opcode);
            self.cycles = self.lookup[op_index].cycles;

            // execute next instruction
            let additional_cycle1: u8 = (self.lookup[op_index].addrmode)(&mut self);
            let additional_cycle2: u8 = (self.lookup[op_index].operate)(&mut self);

            // add additional cycles if necessary
            self.cycles += additional_cycle1 & additional_cycle2;
        }

        self.cycles -= 1;
    }
    // fn reset() {}
    // fn irq() {}
    // fn nmi() {}

    // fn fetch -> u8 {}
}

struct Instruction {
    name: String,
    operate: fn(&mut Olc6502) -> u8,
    addrmode: fn(&mut Olc6502) -> u8,
    cycles: u8,
}

    // Addressing Modes
    // region
    fn ACC(o: &mut Olc6502) -> u8 { // Accumulator Addressing
        o.fetched_data = o.accumulator;
        return 0;
    }

    fn IMM(o: &mut Olc6502) -> u8 { // Immediate
        o.addr_abs = o.prog_ctr & 0x00FF;
        return 0; 
    }

    fn ABS(o: &mut Olc6502) -> u8 { // Absolute Addressing
        let lo: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        let hi: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        o.addr_abs = (hi << 8) | lo;
        return 0; 
    }

    fn ZP0(o: &mut Olc6502) -> u8 { // Zero Page Addressing
        o.addr_abs = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;
        o.addr_abs &= 0x00FF;
        return 0;    
    }

    fn ZPX(o: &mut Olc6502) -> u8 { // Indexed Zero Page Addressing X
        o.addr_abs = u16::from(o.read(o.prog_ctr) + o.x_reg);
        o.prog_ctr += 1;
        o.addr_abs &= 0x00FF;
        return 0; 
    }

    fn ZPY(o: &mut Olc6502) -> u8 { // Indexed Zero Page Addressing Y
        o.addr_abs = u16::from(o.read(o.prog_ctr) + o.y_reg);
        o.prog_ctr += 1;
        o.addr_abs &= 0x00FF;
        return 0; 
    }

    fn ABX(o: &mut Olc6502) -> u8 { // Indexed Absolute Addressing X
        let lo: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        let hi: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        o.addr_abs = (hi << 8) | lo;
        o.addr_abs += u16::from(o.x_reg);

        if (o.addr_abs & 0xFF00) != (hi << 8) {
            return 1;
        } else {
            return 0;
        }
    }

    fn ABY(o: &mut Olc6502) -> u8 { // Indexed Absolute Addressing Y
        let lo: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        let hi: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        o.addr_abs = (hi << 8) | lo;
        o.addr_abs += u16::from(o.y_reg);

        if (o.addr_abs & 0xFF00) != (hi << 8) {
            return 1;
        } else {
            return 0;
        }
    }

    fn IMP(o: &mut Olc6502) -> u8 { // Implied
        return 0; 
    }

    fn REL(o: &mut Olc6502) -> u8 { // Relative Adressing
        o.addr_rel = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;
        if o.addr_rel & 0x80 == 1 {
            o.addr_rel |= 0xFF00;
        }
        return 0;
    }

    fn IZX(o: &mut Olc6502) -> u8 { // Indexed Indirect Addressing X
        let t: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        let x: u16 = u16::from(o.x_reg);
        let lo_addr: u16 = u16::from((t + x) & 0x00FF);
        let hi_addr: u16 = u16::from((t + x + 1) & 0x00FF);
        let lo: u16 = u16::from(o.read(lo_addr));
        let hi: u16 = u16::from(o.read(hi_addr));

        o.addr_abs = (hi << 8) | lo;
        return 0;
    }

    fn IZY(o: &mut Olc6502) -> u8 { // Indirect Indexed Addressing Y
        let t: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        let lo: u16 = u16::from(o.read(t & 0x00FF));
        let hi: u16 = u16::from(o.read((t + 1) & 0x00FF));
        
        o.addr_abs = (hi << 8) | lo;
        o.addr_abs += u16::from(o.y_reg);

        if (o.addr_abs & 0xFF00) != (hi << 8) {
            return 1;
        } else {
            return 0;
        }
    }

    fn IND(o: &mut Olc6502) -> u8 { // Absolute Indirect
        let ptr_lo: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;
        let ptr_hi: u16 = u16::from(o.read(o.prog_ctr));
        o.prog_ctr += 1;

        let ptr: u16 = (ptr_hi << 8) | ptr_lo;

        if ptr_lo == 0x00FF { // Simulate page boundary hardware bug
            o.addr_abs = (u16::from(o.read(ptr & 0xFF00)) << 8) | u16::from(o.read(ptr + 0));
        } else {
            o.addr_abs = (u16::from(o.read(ptr + 1)) << 8) | u16::from(o.read(ptr + 0));
        }

        return 0;
    }
    // endregion

    // Opcodes
    // region
    fn ADC(o: &mut Olc6502) -> u8 { // Add Memory to Accumulator with Carry
        return 0x0; 
    }

    fn AND(o: &mut Olc6502) -> u8 { // "AND" Memory with Accumulator
        return 0x0; 
    }

    fn ASL(o: &mut Olc6502) -> u8 { // Shift Left One Bit (Memory or Accumulator)
        return 0x0; 
    }

    fn BCC(o: &mut Olc6502) -> u8 { // Branch on Carry Clear
        return 0x0; 
    }

    fn BCS(o: &mut Olc6502) -> u8 { // Branch on Carry Set
        return 0x0; 
    }

    fn BEQ(o: &mut Olc6502) -> u8 { // Branch on Result Zero
        return 0x0; 
    }

    fn BIT(o: &mut Olc6502) -> u8 { // Test Bits in Memory with Accumulator
        return 0x0; 
    }

    fn BMI(o: &mut Olc6502) -> u8 { // Branch on Result Minus
        return 0x0; 
    }

    fn BNE(o: &mut Olc6502) -> u8 { // Branch on Result not Zero
        return 0x0; 
    }

    fn BPL(o: &mut Olc6502) -> u8 { // Branch on Result Plus
        return 0x0; 
    }

    fn BRK(o: &mut Olc6502) -> u8 { // Force Break
        return 0x0; 
    }

    fn BVC(o: &mut Olc6502) -> u8 { // Branch on Overflow Clear
        return 0x0; 
    }

    fn BVS(o: &mut Olc6502) -> u8 { // Branch on Overflow Set
        return 0x0; 
    }

    fn CLC(o: &mut Olc6502) -> u8 { // Clear Carry Flag
        return 0x0; 
    }

    fn CLD(o: &mut Olc6502) -> u8 { // Clear Decimal Mode
        return 0x0; 
    }

    fn CLI(o: &mut Olc6502) -> u8 { // Clear Interrupt Disable Bit
        return 0x0; 
    }

    fn CLV(o: &mut Olc6502) -> u8 { // Clear Overflow Flag
        return 0x0; 
    }

    fn CMP(o: &mut Olc6502) -> u8 { // Compare Memory And Accumulator
        return 0x0; 
    }

    fn CPX(o: &mut Olc6502) -> u8 { // Compare Memory and Index X
        return 0x0; 
    }

    fn CPY(o: &mut Olc6502) -> u8 { // Compare Memory And Index Y
        return 0x0; 
    }

    fn DEC(o: &mut Olc6502) -> u8 { // Decrement Memory by One
        return 0x0; 
    }

    fn DEX(o: &mut Olc6502) -> u8 { // Decrement Index X by One
        return 0x0; 
    }

    fn DEY(o: &mut Olc6502) -> u8 { // Decrement Index Y by One
        return 0x0; 
    }

    fn EOR(o: &mut Olc6502) -> u8 { // "Exclusive-OR" Memory with Accumulator
        return 0x0; 
    }

    fn INC(o: &mut Olc6502) -> u8 { // Increment Memory by One
        return 0x0; 
    }

    fn INX(o: &mut Olc6502) -> u8 { // Increment Index X by One
        return 0x0; 
    }

    fn INY(o: &mut Olc6502) -> u8 { // Increment Index Y by One
        return 0x0; 
    }

    fn JMP(o: &mut Olc6502) -> u8 { // Jump to New Location
        return 0x0; 
    }

    fn JSR(o: &mut Olc6502) -> u8 { // Jump to New Location Saving Return Address
        return 0x0; 
    }

    fn LDA(o: &mut Olc6502) -> u8 { // Load Accumulator with Memory
        return 0x0; 
    }

    fn LDX(o: &mut Olc6502) -> u8 { // Load Index X with Memory
        return 0x0; 
    }

    fn LDY(o: &mut Olc6502) -> u8 { // Load Index Y with Memory
        return 0x0; 
    }

    fn LSR(o: &mut Olc6502) -> u8 { // Shift One Bit Right (Memory or Accumulator)
        return 0x0; 
    }

    fn NOP(o: &mut Olc6502) -> u8 { // No Operation
        return 0x0; 
    }

    fn ORA(o: &mut Olc6502) -> u8 { // "OR" Memory with Accumulator
        return 0x0; 
    }

    fn PHA(o: &mut Olc6502) -> u8 { // Push Accumulator on Stack
        return 0x0; 
    }

    fn PHP(o: &mut Olc6502) -> u8 { // Push Processor Status on Stack
        return 0x0; 
    }

    fn PLA(o: &mut Olc6502) -> u8 { // Pull Accumulator from Stack
        return 0x0; 
    }

    fn PLP(o: &mut Olc6502) -> u8 { // Pull Processor Status from Stack
        return 0x0; 
    }

    fn ROL(o: &mut Olc6502) -> u8 { // Rotate One Bit Left (Memory or Accumulator)
        return 0x0; 
    }

    fn ROR(o: &mut Olc6502) -> u8 { // Rotate One Bit Right (Memory or Accumulator)
        return 0x0; 
    }

    fn RTI(o: &mut Olc6502) -> u8 { // Return from Interrupt
        return 0x0; 
    }

    fn RTS(o: &mut Olc6502) -> u8 { // Return from Subroutine
        return 0x0; 
    }

    fn SBC(o: &mut Olc6502) -> u8 { // Subtract Memory from Accumulator with Borrow
        return 0x0; 
    }

    fn SEC(o: &mut Olc6502) -> u8 { // Set Carry Flag
        return 0x0; 
    }

    fn SED(o: &mut Olc6502) -> u8 { // Set Decimal Mode (unused)
        return 0x0; 
    }

    fn SEI(o: &mut Olc6502) -> u8 { // Set Interrupt Disable Status
        return 0x0; 
    }

    fn STA(o: &mut Olc6502) -> u8 { // Store Accumulator in Memory
        return 0x0; 
    }

    fn STX(o: &mut Olc6502) -> u8 { // Store Index X in Memory
        return 0x0; 
    }

    fn STY(o: &mut Olc6502) -> u8 { // Store Index Y in Memory
        return 0x0; 
    }

    fn TAX(o: &mut Olc6502) -> u8 { // Transfer Accumulator to Index X
        return 0x0; 
    }

    fn TAY(o: &mut Olc6502) -> u8 { // Transfer Accumulator to Index Y
        return 0x0; 
    }

    fn TSX(o: &mut Olc6502) -> u8 { // Transfer Stack Pointer to Index X
        return 0x0; 
    }

    fn TXA(o: &mut Olc6502) -> u8 { // Transfer Index X to Accumulator
        return 0x0; 
    }

    fn TXS(o: &mut Olc6502) -> u8 { // Transfer Index X to Stack Register
        return 0x0; 
    }

    fn TYA(o: &mut Olc6502) -> u8 { // Transfer Index Y to Accumulator
        return 0x0; 
    }

    fn XXX(o: &mut Olc6502) -> u8 { // Undefined Instruction
        return 0x0; 
    }
    // endregion


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
