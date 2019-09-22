#![allow(dead_code)]
use super::bus;

static STACK_BASE: u16 = 0x0100;

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

impl PartialEq for Olc6502 {
    fn eq(&self, other: &Olc6502) -> bool {
        self.accumulator == other.accumulator &&
        self.x_reg == other.x_reg &&
        self.y_reg == other.y_reg &&
        self.stack_ptr == other.stack_ptr &&
        self.prog_ctr == other.prog_ctr &&
        self.status_reg == other.status_reg &&
        self.bus == other.bus &&
        self.fetched_data == other.fetched_data &&
        self.addr_abs == other.addr_abs &&
        self.addr_rel == other.addr_rel &&
        self.opcode == other.opcode &&
        self.cycles == other.cycles 
    }
}


impl Olc6502 {
    fn reset(&mut self) {
        // set program counter to known location
        self.addr_abs = 0xFFFC;
        let lo: u16 = self.bus.read(self.addr_abs) as u16;
        let hi: u16 = self.bus.read(self.addr_abs + 1) as u16;

        self.accumulator = 0;
        self.x_reg = 0;
        self.y_reg = 0;
        self.stack_ptr = 0xFF;
        self.prog_ctr = (hi << 8) | lo;
        self.status_reg = 0;

        self.fetched_data = 0;
        self.addr_abs = 0;
        self.addr_rel = 0;
        self.opcode = 0;
        self.cycles = 8; // reset takes time
        self.lookup = populate_lookup_table();
    }

    fn read(&self, addr: u16) -> u8 {
        return self.bus.read(addr);
    }

    fn write(mut self, addr: u16, data: u8) {
        self.bus.write(addr, data);
    }

    fn get_flag(&self, flag: Flags6502) -> u8 { 
        let f = flag as u8;
        if (self.status_reg & f) > 0 {
            return 1;
        } else {
            return 0;
        }
    }

    fn set_flag(&mut self, flag: Flags6502, v: bool) {
        let f = flag as u8;
        if v == true {
            self.status_reg |= f;
        } else {
            self.status_reg &= !f;
        }
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

    fn stack_top(&self) -> u16 {
        return STACK_BASE | self.stack_ptr as u16;
    }

    fn run_interrupt(&mut self, inter_addr: u16, cycles: u8) {
        self.push_to_stack((self.prog_ctr >> 8) as u8);
        self.push_to_stack(self.prog_ctr as u8);
        self.push_to_stack(self.status_reg);

        self.set_flag(Flags6502::B, false);
        self.set_flag(Flags6502::U, true);
        self.set_flag(Flags6502::I, true);

        self.addr_abs = inter_addr; // hardcoded interrupt address
        let lo = self.bus.read(self.addr_abs) as u16;
        let hi = self.bus.read(self.addr_abs + 1) as u16;
        self.prog_ctr = (hi << 8) | lo;

        self.cycles = cycles;
    }

    fn irq(&mut self) {
        if self.get_flag(Flags6502::I) == 0 {
            self.run_interrupt(0xFFFE, 7);
        }
     }

    fn nmi(&mut self) {
        self.run_interrupt(0xFFFA, 8);
    }

    fn push_to_stack(&mut self, data: u8) {
        let current_stack_location = STACK_BASE | self.stack_ptr as u16;
        self.bus.write(current_stack_location, data);
        self.stack_ptr -= 1;
    }

    fn pop_from_stack(&mut self) -> u8 {
        self.stack_ptr += 1;
        let current_stack_location = STACK_BASE | self.stack_ptr as u16;
        return self.bus.read(current_stack_location);
    }

    fn fetch(&mut self) -> u8 {
        let i: usize = usize::from(self.opcode);
        // cast function pointers to usize to compare
        let addrmode: usize = self.lookup[i].addrmode as usize;
        let imp: usize = IMP as usize;
        let acc: usize = ACC as usize;
        if addrmode == acc {
            self.fetched_data = self.accumulator;
        }
        else if !(addrmode == imp) {
            self.fetched_data = self.read(self.addr_abs);
        }
        return self.fetched_data
    }
}

fn create_olc6502() -> Olc6502 {
    let mut o = Olc6502 {
        accumulator: 0,
        x_reg: 0,
        y_reg: 0,
        stack_ptr: 0,
        prog_ctr: 0,
        status_reg: 0,
        bus: bus::create_bus(),
        fetched_data: 0,
        addr_abs: 0,
        addr_rel: 0,
        opcode: 0,
        cycles: 0,
        lookup: populate_lookup_table(),
    };
    o.reset();
    return o;
}

struct Instruction {
    name: String,
    operate: fn(&mut Olc6502) -> u8,
    addrmode: fn(&mut Olc6502) -> u8,
    cycles: u8,
}

fn populate_lookup_table() -> [Instruction; 256] {
    fn i(name: &str, operate: fn(&mut Olc6502) -> u8, addrmode: fn(&mut Olc6502) -> u8, cycles: u8) -> Instruction {
        return Instruction { name: String::from(name), operate, addrmode, cycles };
    }

    return [
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


// Addressing Modes
// region
#[allow(non_snake_case)]
fn ACC(o: &mut Olc6502) -> u8 { // Accumulator Addressing
    o.fetched_data = o.accumulator;
    return 0;
}

#[allow(non_snake_case)]
fn IMM(o: &mut Olc6502) -> u8 { // Immediate
    o.addr_abs = o.prog_ctr;
    o.prog_ctr += 1;
    return 0; 
}

#[allow(non_snake_case)]
fn ABS(o: &mut Olc6502) -> u8 { // Absolute Addressing
    let lo: u16 = u16::from(o.read(o.prog_ctr));
    o.prog_ctr += 1;

    let hi: u16 = u16::from(o.read(o.prog_ctr));
    o.prog_ctr += 1;

    o.addr_abs = (hi << 8) | lo;
    return 0; 
}

#[allow(non_snake_case)]
fn ZP0(o: &mut Olc6502) -> u8 { // Zero Page Addressing
    o.addr_abs = u16::from(o.read(o.prog_ctr));
    o.prog_ctr += 1;
    o.addr_abs &= 0x00FF;
    return 0;    
}

#[allow(non_snake_case)]
fn ZPX(o: &mut Olc6502) -> u8 { // Indexed Zero Page Addressing X
    o.addr_abs = u16::from(o.read(o.prog_ctr) + o.x_reg);
    o.prog_ctr += 1;
    o.addr_abs &= 0x00FF;
    return 0; 
}

#[allow(non_snake_case)]
fn ZPY(o: &mut Olc6502) -> u8 { // Indexed Zero Page Addressing Y
    o.addr_abs = u16::from(o.read(o.prog_ctr) + o.y_reg);
    o.prog_ctr += 1;
    o.addr_abs &= 0x00FF;
    return 0; 
}

#[allow(non_snake_case)]
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

#[allow(non_snake_case)]
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

#[allow(non_snake_case)]
fn IMP(_: &mut Olc6502) -> u8 { // Implied
    //o.fetched_data = o.accumulator;
    return 0; 
}

#[allow(non_snake_case)]
fn REL(o: &mut Olc6502) -> u8 { // Relative Addressing
    // javidx9 saves the offset as its own 'addr_rel' variable and
    // computes the final address later, probably in the opcode instruction
    /* 
    o.addr_rel = u16::from(o.read(o.prog_ctr));
    o.prog_ctr += 1;
    if o.addr_rel & 0x80 == 1 {
        o.addr_rel |= 0xFF00;
    }
    */
    let offset: u8 = o.read(o.prog_ctr);
    o.addr_abs = o.prog_ctr + u16::from(offset);
    o.addr_abs &= 0x00FF;
    o.prog_ctr += 1;
    return 0;
}

#[allow(non_snake_case)]
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

#[allow(non_snake_case)]
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

#[allow(non_snake_case)]
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
#[allow(non_snake_case)]
fn ADC(o: &mut Olc6502) -> u8 { // Add Memory to Accumulator with Carry
    let data: u8 = o.fetch();
    add(o, data as u16);
    return 1;
}

fn add(o: &mut Olc6502, data16: u16) {
    let a: u16 = o.accumulator as u16;
    let flag_c: u16 = o.get_flag(Flags6502::C) as u16;
    let sum: u16 = a + data16 + flag_c;
    o.set_flag(Flags6502::C, sum > 0xFF);
    o.set_flag(Flags6502::Z, (sum & 0x00FF) == 0);
    o.set_flag(Flags6502::N, (sum & 0x80) > 0);
    o.set_flag(Flags6502::V, ((!(a ^ data16) & (a ^ sum)) & 0x0080) > 0);
    o.accumulator = (sum & 0x00FF) as u8;
}

#[allow(non_snake_case)]
fn AND(o: &mut Olc6502) -> u8 { // "AND" Memory with Accumulator
    let data: u8 = o.fetch();
    o.accumulator &= data;
    o.set_flag(Flags6502::Z, o.accumulator == 0x00);
    o.set_flag(Flags6502::N, o.accumulator & 0x80 >= 1);
    return 1;
}

#[allow(non_snake_case)]
fn ASL(o: &mut Olc6502) -> u8 { // Shift Left One Bit (Memory or Accumulator)
    let data = o.fetch();
    let temp = (data as u16) << 1;
    o.set_flag(Flags6502::C, temp > 0xFF);
    o.set_flag(Flags6502::N, (temp & 0x80) > 1);
    o.set_flag(Flags6502::Z, (temp as u8) == 0x00);
    let result = temp as u8;
    if o.lookup[o.opcode as usize].addrmode as usize == ACC as usize {
        o.accumulator = result;
    } else {
        o.bus.write(o.addr_abs, result);
    }
    return 0;
}

fn perform_jump(o: &mut Olc6502) {
    o.cycles += 1;
    if (o.addr_abs & 0xFF00) != (o.prog_ctr & 0xFF00) {
        o.cycles += 1;
    }
    o.prog_ctr = o.addr_abs;
}

#[allow(non_snake_case)]
fn BCC(o: &mut Olc6502) -> u8 { // Branch on Carry Clear
    if o.get_flag(Flags6502::C) == 0 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn BCS(o: &mut Olc6502) -> u8 { // Branch on Carry Set
    if o.get_flag(Flags6502::C) == 1 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn BEQ(o: &mut Olc6502) -> u8 { // Branch on Result Zero
    if o.get_flag(Flags6502::Z) == 1 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn BIT(o: &mut Olc6502) -> u8 { // Test Bits in Memory with Accumulator
    let fetched = o.fetch();
    let data = o.accumulator & fetched;
    o.set_flag(Flags6502::Z, data == 0);
    o.set_flag(Flags6502::N, fetched & (1 << 7) == 1);
    o.set_flag(Flags6502::V, fetched & (1 << 6) == 1);
    return 0;
}

#[allow(non_snake_case)]
fn BMI(o: &mut Olc6502) -> u8 { // Branch on Result Minus
    if o.get_flag(Flags6502::N) == 1 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn BNE(o: &mut Olc6502) -> u8 { // Branch on Result not Zero
    if o.get_flag(Flags6502::Z) == 0 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn BPL(o: &mut Olc6502) -> u8 { // Branch on Result Plus
    if o.get_flag(Flags6502::N) == 0 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn BRK(o: &mut Olc6502) -> u8 { // Force Break
    o.nmi();
    o.prog_ctr += 1;
    return 0;
}

#[allow(non_snake_case)]
fn BVC(o: &mut Olc6502) -> u8 { // Branch on Overflow Clear
    if o.get_flag(Flags6502::V) == 0 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn BVS(o: &mut Olc6502) -> u8 { // Branch on Overflow Set
    if o.get_flag(Flags6502::V) == 1 {
        perform_jump(o);
    }
    return 0;
}

#[allow(non_snake_case)]
fn CLC(o: &mut Olc6502) -> u8 { // Clear Carry Flag
    o.set_flag(Flags6502::C, false);
    return 0; 
}

#[allow(non_snake_case)]
fn CLD(o: &mut Olc6502) -> u8 { // Clear Decimal Mode
    o.set_flag(Flags6502::D, false);
    return 0; 
}

#[allow(non_snake_case)]
fn CLI(o: &mut Olc6502) -> u8 { // Clear Interrupt Disable Bit
    o.set_flag(Flags6502::I, false);
    return 0; 
}

#[allow(non_snake_case)]
fn CLV(o: &mut Olc6502) -> u8 { // Clear Overflow Flag
    o.set_flag(Flags6502::V, false);
    return 0; 
}

#[allow(non_snake_case)]
fn CMP(o: &mut Olc6502) -> u8 { // Compare Memory And Accumulator
    let data = o.fetch();
    o.set_flag(Flags6502::C, o.accumulator >= data);
    o.set_flag(Flags6502::Z, o.accumulator == data);
    o.set_flag(Flags6502::N, o.accumulator >= 0x80);
    return 1; 
}

#[allow(non_snake_case)]
fn CPX(o: &mut Olc6502) -> u8 { // Compare Memory and Index X
    let data = o.fetch();
    o.set_flag(Flags6502::C, o.x_reg >= data);
    o.set_flag(Flags6502::Z, o.x_reg == data);
    o.set_flag(Flags6502::N, o.x_reg >= 0x80);
    return 1; 
}

#[allow(non_snake_case)]
fn CPY(o: &mut Olc6502) -> u8 { // Compare Memory And Index Y
    let data = o.fetch();
    o.set_flag(Flags6502::C, o.y_reg >= data);
    o.set_flag(Flags6502::Z, o.y_reg == data);
    o.set_flag(Flags6502::N, o.y_reg >= 0x80);
    return 1; 
}

#[allow(non_snake_case)]
fn DEC(o: &mut Olc6502) -> u8 { // Decrement Memory by One
    let data = o.fetch();
    let result = data - 1;
    o.set_flag(Flags6502::N, (result & 0x80) > 1);
    o.set_flag(Flags6502::Z, result == 0x00);
    o.bus.write(o.addr_abs, result);
    return 0;
}

#[allow(non_snake_case)]
fn DEX(o: &mut Olc6502) -> u8 { // Decrement Index X by One
    o.x_reg -= 1;
    o.set_flag(Flags6502::N, (o.x_reg & 0x80) > 1);
    o.set_flag(Flags6502::Z, o.x_reg == 0x00);
    return 0;
}

#[allow(non_snake_case)]
fn DEY(o: &mut Olc6502) -> u8 { // Decrement Index Y by One
    o.y_reg -= 1;
    o.set_flag(Flags6502::N, (o.y_reg & 0x80) > 1);
    o.set_flag(Flags6502::Z, o.y_reg == 0x00);
    return 0;
}

#[allow(non_snake_case)]
fn EOR(o: &mut Olc6502) -> u8 { // "Exclusive-OR" Memory with Accumulator
    let data = o.fetch();
    o.accumulator ^= data;
    o.set_flag(Flags6502::N, (o.accumulator & 0x80) > 1);
    o.set_flag(Flags6502::Z, o.accumulator == 0x00);
    return 1; 
}

#[allow(non_snake_case)]
fn INC(o: &mut Olc6502) -> u8 { // Increment Memory by One
    let data = o.fetch();
    let result = ((data as u16) + 1) as u8; // cast to u16 to handle incrementing 0xFF
    o.set_flag(Flags6502::N, (result & 0x80) > 1);
    o.set_flag(Flags6502::Z, result == 0x00);
    o.bus.write(o.addr_abs, result);
    return 0;
}

#[allow(non_snake_case)]
fn INX(o: &mut Olc6502) -> u8 { // Increment Index X by One
    o.x_reg = ((o.x_reg as u16) + 1) as u8; // cast to u16 to handle incrementing 0xFF
    o.set_flag(Flags6502::N, (o.x_reg & 0x80) > 1);
    o.set_flag(Flags6502::Z, o.x_reg == 0x00);
    return 0;
}

#[allow(non_snake_case)]
fn INY(o: &mut Olc6502) -> u8 { // Increment Index Y by One
    o.y_reg = ((o.y_reg as u16) + 1) as u8; // cast to u16 to handle incrementing 0xFF
    o.set_flag(Flags6502::N, (o.y_reg & 0x80) > 1);
    o.set_flag(Flags6502::Z, o.y_reg == 0x00);
    return 0;
}

#[allow(non_snake_case)]
fn JMP(o: &mut Olc6502) -> u8 { // Jump to New Location
    o.prog_ctr = o.addr_abs;
    return 0;
}

#[allow(non_snake_case)]
fn JSR(o: &mut Olc6502) -> u8 { // Jump to New Location Saving Return Address
    let hi = (o.prog_ctr >> 8) as u8;
    let lo = (o.prog_ctr & 0x00FF) as u8;
    o.push_to_stack(hi);
    o.push_to_stack(lo);
    o.prog_ctr = o.addr_abs;
    return 0;
}

#[allow(non_snake_case)]
fn LDA(o: &mut Olc6502) -> u8 { // Load Accumulator with Memory
    o.accumulator = o.fetch();
    return 1;
}

#[allow(non_snake_case)]
fn LDX(o: &mut Olc6502) -> u8 { // Load Index X with Memory
    o.x_reg = o.fetch();
    return 1;
}

#[allow(non_snake_case)]
fn LDY(o: &mut Olc6502) -> u8 { // Load Index Y with Memory
    o.y_reg = o.fetch();
    return 1;
}

#[allow(non_snake_case)]
fn LSR(o: &mut Olc6502) -> u8 { // Shift One Bit Right (Memory or Accumulator)
    let data = o.fetch();
    let temp = (data as u16) >> 1;
    o.set_flag(Flags6502::C, (data & 0x1) == 1);
    o.set_flag(Flags6502::N, (temp & 0x80) > 1);
    o.set_flag(Flags6502::Z, (temp as u8) == 0x00);
    let result = temp as u8;
    if o.lookup[o.opcode as usize].addrmode as usize == ACC as usize {
        o.accumulator = result;
    } else {
        o.bus.write(o.addr_abs, result);
    }
    return 0;
}

#[allow(non_snake_case)]
fn NOP(_: &mut Olc6502) -> u8 { // No Operation
    return 0x0; 
}

#[allow(non_snake_case)]
fn ORA(o: &mut Olc6502) -> u8 { // "OR" Memory with Accumulator
    return 0x0; 
}

#[allow(non_snake_case)]
fn PHA(o: &mut Olc6502) -> u8 { // Push Accumulator on Stack
    o.push_to_stack(o.accumulator);
    return 0;
}

#[allow(non_snake_case)]
fn PHP(o: &mut Olc6502) -> u8 { // Push Processor Status on Stack
    o.push_to_stack(o.status_reg);
    return 0;
}

#[allow(non_snake_case)]
fn PLA(o: &mut Olc6502) -> u8 { // Pull Accumulator from Stack
    o.accumulator = o.pop_from_stack();
    return 0;
}

#[allow(non_snake_case)]
fn PLP(o: &mut Olc6502) -> u8 { // Pull Processor Status from Stack
    o.status_reg = o.pop_from_stack();
    return 0;
}

#[allow(non_snake_case)]
fn ROL(o: &mut Olc6502) -> u8 { // Rotate One Bit Left (Memory or Accumulator)
    return 0x0; 
}

#[allow(non_snake_case)]
fn ROR(o: &mut Olc6502) -> u8 { // Rotate One Bit Right (Memory or Accumulator)
    return 0x0; 
}

#[allow(non_snake_case)]
fn RTI(o: &mut Olc6502) -> u8 { // Return from Interrupt
    o.status_reg = o.pop_from_stack();

    o.prog_ctr = o.pop_from_stack() as u16;
    o.prog_ctr |= (o.pop_from_stack() as u16) << 8;
    return 0;
}

#[allow(non_snake_case)]
fn RTS(o: &mut Olc6502) -> u8 { // Return from Subroutine
    return 0x0; 
}

#[allow(non_snake_case)]
fn SBC(o: &mut Olc6502) -> u8 { // Subtract Memory from Accumulator with Borrow
    let data: u8 = o.fetch();
    let inverted_data: u16 = ((data as u16) ^ 0x00FF) + 1;
    add(o, inverted_data); // make the data negative and add it
    return 1;
}

#[allow(non_snake_case)]
fn SEC(o: &mut Olc6502) -> u8 { // Set Carry Flag
    o.set_flag(Flags6502::C, true);
    return 0; 
}

#[allow(non_snake_case)]
fn SED(o: &mut Olc6502) -> u8 { // Set Decimal Mode (unused)
    o.set_flag(Flags6502::D, true);
    return 0; 
}

#[allow(non_snake_case)]
fn SEI(o: &mut Olc6502) -> u8 { // Set Interrupt Disable Status
    o.set_flag(Flags6502::I, true);
    return 0; 
}

#[allow(non_snake_case)]
fn STA(o: &mut Olc6502) -> u8 { // Store Accumulator in Memory
    return 0x0; 
}

#[allow(non_snake_case)]
fn STX(o: &mut Olc6502) -> u8 { // Store Index X in Memory
    return 0x0; 
}

#[allow(non_snake_case)]
fn STY(o: &mut Olc6502) -> u8 { // Store Index Y in Memory
    return 0x0; 
}

#[allow(non_snake_case)]
fn TAX(o: &mut Olc6502) -> u8 { // Transfer Accumulator to Index X
    return 0x0; 
}

#[allow(non_snake_case)]
fn TAY(o: &mut Olc6502) -> u8 { // Transfer Accumulator to Index Y
    return 0x0; 
}

#[allow(non_snake_case)]
fn TSX(o: &mut Olc6502) -> u8 { // Transfer Stack Pointer to Index X
    return 0x0; 
}

#[allow(non_snake_case)]
fn TXA(o: &mut Olc6502) -> u8 { // Transfer Index X to Accumulator
    return 0x0; 
}

#[allow(non_snake_case)]
fn TXS(o: &mut Olc6502) -> u8 { // Transfer Index X to Stack Register
    return 0x0; 
}

#[allow(non_snake_case)]
fn TYA(o: &mut Olc6502) -> u8 { // Transfer Index Y to Accumulator
    return 0x0; 
}

#[allow(non_snake_case)]
fn XXX(o: &mut Olc6502) -> u8 { // Undefined Instruction
    return 0x0; 
}
// endregion


// Tests
// region
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_reg_read() {
        let mut o: Olc6502 = create_olc6502();
        o.status_reg = 0x55;
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::I) == 1);
        assert!(o.get_flag(Flags6502::D) == 0);
        assert!(o.get_flag(Flags6502::B) == 1);
        assert!(o.get_flag(Flags6502::U) == 0);
        assert!(o.get_flag(Flags6502::V) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    fn test_status_reg_write() {
        let mut o: Olc6502 = create_olc6502();
        o.status_reg = 0x00;
        o.set_flag(Flags6502::C, true);
        assert!(o.get_flag(Flags6502::C) == 1);

        o.set_flag(Flags6502::Z, true);
        assert!(o.get_flag(Flags6502::Z) == 1);

        o.set_flag(Flags6502::I, true);
        assert!(o.get_flag(Flags6502::I) == 1);

        o.set_flag(Flags6502::D, true);
        assert!(o.get_flag(Flags6502::D) == 1);

        o.set_flag(Flags6502::B, true);
        assert!(o.get_flag(Flags6502::B) == 1);

        o.set_flag(Flags6502::U, true);
        assert!(o.get_flag(Flags6502::U) == 1);

        o.set_flag(Flags6502::V, true);
        assert!(o.get_flag(Flags6502::V) == 1);

        o.set_flag(Flags6502::N, true);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_irq_and_RTI() {
        let mut o: Olc6502 = create_olc6502();
        o.prog_ctr = 0x11EC;
        o.status_reg = 0x28;
        o.bus.write(0xFFFE, 0xAD);
        o.bus.write(0xFFFF, 0xDE);
        let old_pc = o.prog_ctr;
        let old_status = o.status_reg;
        let old_stack = o.stack_top();
        o.irq();
        assert!(o.bus.read(0x01FF) == 0x11);
        assert!(o.bus.read(0x01FE) == 0xEC);
        assert!(o.bus.read(0x01FD) == 0x28);
        assert!(o.prog_ctr == 0xDEAD);
        assert!(o.get_flag(Flags6502::I) == 1);
        assert!(o.get_flag(Flags6502::B) == 0);
        assert!(o.get_flag(Flags6502::U) == 1);
        RTI(&mut o);
        assert!(o.prog_ctr == old_pc);
        assert!(o.status_reg == old_status);
        assert!(o.stack_top() == old_stack);
    }

    fn test_nmi() {
        let mut o: Olc6502 = create_olc6502();
        o.prog_ctr = 0x11EC;
        o.status_reg = 0x28;
        o.bus.write(0xFFFA, 0xAD);
        o.bus.write(0xFFFB, 0xDE);
        o.set_flag(Flags6502::I, true);
        o.nmi();
        assert!(o.bus.read(0x01FF) == 0x11);
        assert!(o.bus.read(0x01FE) == 0xEC);
        assert!(o.bus.read(0x01FD) == 0x28);
        assert!(o.prog_ctr == 0xDEAD);
        assert!(o.get_flag(Flags6502::I) == 1);
        assert!(o.get_flag(Flags6502::B) == 0);
        assert!(o.get_flag(Flags6502::U) == 1);
    }

    // addressing mode tests
    // region
    #[test]
    #[allow(non_snake_case)]
    fn am_ACC_test() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x65;
        ACC(&mut o);
        assert!(o.fetched_data == 0x65);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IMM_test() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = o.prog_ctr;
        IMM(&mut o);
        assert!(o.addr_abs == current_addr);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IMP_test() {
        let mut test: Olc6502 = create_olc6502();
        let reference: Olc6502 = create_olc6502();
        IMP(&mut test);
        assert!(test == reference);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_REL_positive() {
        let mut o: Olc6502 = create_olc6502();
        let offset: u8 = 0x20;
        let current_addr: u16 = 0x24;
        o.prog_ctr = current_addr;
        o.bus.write(current_addr, offset);
        REL(&mut o);
        let new_addr: u16 = current_addr + u16::from(offset);
        assert!(o.addr_abs == new_addr);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_REL_negative() {
        let mut o: Olc6502 = create_olc6502();
        let offset: u8 = 0xE0; // -0x20
        let current_addr: u16 = 0x24;
        o.prog_ctr = current_addr;
        o.bus.write(current_addr, offset);
        REL(&mut o);
        assert!(o.addr_abs == 0x04);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ABS() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.bus.write(current_addr, 0x32);
        o.bus.write(current_addr+1, 0x40);
        o.prog_ctr = current_addr;
        ABS(&mut o);
        assert!(o.addr_abs == 0x4032);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ZP0() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.bus.write(current_addr, 0x32);
        o.prog_ctr = current_addr;
        ZP0(&mut o);
        assert!(o.addr_abs == 0x32);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IND() {
        let mut o: Olc6502 = create_olc6502();
        o.bus.write(0x24, 0x00);
        o.bus.write(0x25, 0x10);
        o.bus.write(0x1000, 0x52);
        o.bus.write(0x1001, 0x3A);
        o.prog_ctr = 0x24;
        IND(&mut o);
        assert!(o.addr_abs == 0x3A52);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IND_FF_page_bug() {
        let mut o: Olc6502 = create_olc6502();
        o.bus.write(0x24, 0xFF);
        o.bus.write(0x25, 0x10);
        o.bus.write(0x1000, 0xAA);
        o.bus.write(0x10FF, 0x3A);
        o.bus.write(0x1100, 0xEE);
        o.prog_ctr = 0x24;
        IND(&mut o);
        assert!(o.addr_abs == 0xAA3A);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ABX() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.x_reg = 0x04;
        o.bus.write(current_addr, 0x32);
        o.bus.write(current_addr+1, 0x40);
        o.prog_ctr = current_addr;
        ABX(&mut o);
        assert!(o.addr_abs == 0x4036);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ABY() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.y_reg = 0x04;
        o.bus.write(current_addr, 0x32);
        o.bus.write(current_addr+1, 0x40);
        o.prog_ctr = current_addr;
        ABY(&mut o);
        assert!(o.addr_abs == 0x4036);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IZX_normal() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x24;
        o.bus.write(0x24, 0x74);
        o.bus.write(0x25, 0x20);
        IZX(&mut o);
        assert!(o.addr_abs == 0x2074);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IZX_wrapped() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0xFF;
        o.prog_ctr = 0x10;
        o.bus.write(0xFF, 0x74);
        o.bus.write(0x00, 0x20);
        IZX(&mut o);
        assert!(o.addr_abs == 0x2074);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IZY() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x10;
        o.prog_ctr = 0x24;
        o.bus.write(0x24, 0x86);
        o.bus.write(0x86, 0x28);
        o.bus.write(0x87, 0x40);
        IZY(&mut o);
        assert!(o.addr_abs == 0x4038);
    }
    // endregion

    // Instruction tests
    // region
    #[test]
    #[allow(non_snake_case)]
    fn op_AND() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xF9;
        o.fetched_data = 0x45;
        AND(&mut o);
        assert!(o.accumulator == 0x41);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ASL_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x80;
        o.addr_abs = 0x100;
        o.opcode = 0x0A; // to get an ASL with the Accum addressing mode
        ASL(&mut o);
        assert!(o.accumulator == 0x00);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ASL_non_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x45;
        o.addr_abs = 0x100;
        ASL(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x8A);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
        assert!(o.get_flag(Flags6502::C) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADD_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xFF;
        o.fetched_data = 0x00;
        AND(&mut o);
        assert!(o.accumulator == 0x00);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADD_negative_result() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xFF;
        o.fetched_data = 0xF0;
        AND(&mut o);
        assert!(o.accumulator == 0xF0);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_pos_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x04;
        o.fetched_data = 0x14;
        ADC(&mut o);
        assert!(o.accumulator == 0x18);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 0);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_pos_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x78;
        o.fetched_data = 0x78;
        ADC(&mut o);
        assert!(o.accumulator == 0xF0);
        assert!(o.get_flag(Flags6502::V) == 1);
        assert!(o.get_flag(Flags6502::N) == 1);
        assert!(o.get_flag(Flags6502::C) == 0);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_neg_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x78;
        o.fetched_data = 0xEC;
        ADC(&mut o);
        assert!(o.accumulator == 0x64);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_neg_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x04;
        o.fetched_data = 0x90;
        ADC(&mut o);
        assert!(o.accumulator == 0x94);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
        assert!(o.get_flag(Flags6502::C) == 0);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_pos_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xFC;
        o.fetched_data = 0x60;
        ADC(&mut o);
        assert!(o.accumulator == 0x5C);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_pos_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x88;
        o.fetched_data = 0x04;
        ADC(&mut o);
        assert!(o.accumulator == 0x8C);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
        assert!(o.get_flag(Flags6502::C) == 0);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_neg_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x88;
        o.fetched_data = 0x88;
        ADC(&mut o);
        assert!(o.accumulator == 0x10);
        assert!(o.get_flag(Flags6502::V) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_neg_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xF6;
        o.fetched_data = 0xF6;
        ADC(&mut o);
        assert!(o.accumulator == 0xEC);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_zero_out() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x14;
        o.fetched_data = 0xEC;
        ADC(&mut o);
        assert!(o.accumulator == 0x00);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 1);
    }


    #[test]
    #[allow(non_snake_case)]
    fn op_BCS_carry_unset() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::C, false);
        BCS(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BCS_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::C, true);
        BCS(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BCS_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::C, true);
        BCS(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BCC_carry_set() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::C, true);
        BCC(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BCC_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::C, false);
        BCC(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BCC_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::C, false);
        BCC(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BEQ_zero_unset() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::Z, false);
        BEQ(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BIT() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xF0;
        o.fetched_data = 0x0F;
        BIT(&mut o);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::V) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BEQ_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::Z, true);
        BEQ(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BEQ_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::Z, true);
        BEQ(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BNE_zero_set() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::Z, true);
        BNE(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BNE_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::Z, false);
        BNE(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BNE_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::Z, false);
        BNE(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BMI_negative_unset() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::N, false);
        BMI(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BMI_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::N, true);
        BMI(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BMI_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::N, true);
        BMI(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BPL_negative_set() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::N, true);
        BPL(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BPL_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::N, false);
        BPL(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BPL_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::N, false);
        BPL(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BVC_overflow_set() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::V, true);
        BVC(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BVC_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::V, false);
        BVC(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BVC_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::V, false);
        BVC(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BVS_overflow_unset() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 50;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::V, false);
        BVS(&mut o);
        assert!(o.prog_ctr == addr); // no jump
        assert!(o.cycles == current_cycles); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BVS_short_jump() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::V, true);
        BVS(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 1); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BVS_jump_page() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F00;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::V, true);
        BVS(&mut o);
        assert!(o.prog_ctr == o.addr_abs);
        assert!(o.cycles == current_cycles + 2); 
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLC() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::C, true);
        CLC(&mut o);
        assert!(o.get_flag(Flags6502::C) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLD() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::D, true);
        CLD(&mut o);
        assert!(o.get_flag(Flags6502::D) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLI() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::I, true);
        CLI(&mut o);
        assert!(o.get_flag(Flags6502::I) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLV() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::V, true);
        CLV(&mut o);
        assert!(o.get_flag(Flags6502::V) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CMP_GT() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x81;
        o.fetched_data = 0x70;
        CMP(&mut o);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CMP_Zero() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x70;
        o.fetched_data = 0x70;
        CMP(&mut o);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPX_GT() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x81;
        o.fetched_data = 0x70;
        CPX(&mut o);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPX_Zero() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x70;
        o.fetched_data = 0x70;
        CPX(&mut o);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPY_GT() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x81;
        o.fetched_data = 0x70;
        CPY(&mut o);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPY_Zero() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x70;
        o.fetched_data = 0x70;
        CPY(&mut o);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEC_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x70;
        DEC(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x6F);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEC_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x10;
        o.fetched_data = 0x01;
        DEC(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x0);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEC_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x10;
        o.fetched_data = 0x8F;
        DEC(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x8E);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEX_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x70;
        DEX(&mut o);
        assert!(o.x_reg == 0x6F);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEX_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x01;
        DEX(&mut o);
        assert!(o.x_reg == 0x0);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEX_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x8F;
        DEX(&mut o);
        assert!(o.x_reg == 0x8E);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEY_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x70;
        DEY(&mut o);
        assert!(o.y_reg == 0x6F);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEY_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x01;
        DEY(&mut o);
        assert!(o.y_reg == 0x0);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEY_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x8F;
        DEY(&mut o);
        assert!(o.y_reg == 0x8E);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_EOR_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x45;
        o.accumulator = 0x30;
        EOR(&mut o);
        assert!(o.accumulator == 0x75);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_EOR_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x80;
        o.accumulator = 0x45;
        EOR(&mut o);
        assert!(o.accumulator == 0xC5);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_EOR_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFF;
        o.accumulator = 0xFF;
        EOR(&mut o);
        assert!(o.accumulator == 0x00);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INC_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x70;
        INC(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x71);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INC_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x10;
        o.fetched_data = 0xFF;
        INC(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x0);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INC_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x10;
        o.fetched_data = 0x80;
        INC(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x81);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INX_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x70;
        INX(&mut o);
        assert!(o.x_reg == 0x71);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INX_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0xFF;
        INX(&mut o);
        assert!(o.x_reg == 0x0);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INX_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x80;
        INX(&mut o);
        assert!(o.x_reg == 0x81);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INY_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x70;
        INY(&mut o);
        assert!(o.y_reg == 0x71);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INY_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0xFF;
        INY(&mut o);
        assert!(o.y_reg == 0x0);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INY_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x80;
        INY(&mut o);
        assert!(o.y_reg == 0x81);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SBC() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x14;
        o.fetched_data = 0x04;
        SBC(&mut o);
        assert!(o.accumulator == 0x10);
        assert!(o.get_flag(Flags6502::V) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 1);
        assert!(o.get_flag(Flags6502::Z) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PHA() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        let old_stack_ptr = o.stack_ptr;
        o.accumulator = 0x14;
        PHA(&mut o);
        assert!(o.bus.read(stack_end) == 0x14);
        assert!(o.stack_ptr == old_stack_ptr - 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PHP() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        let old_stack_ptr = o.stack_ptr;
        o.status_reg = 0x14;
        PHP(&mut o);
        assert!(o.bus.read(stack_end) == 0x14);
        assert!(o.stack_ptr == old_stack_ptr - 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLA() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.bus.write(stack_end, 0x14);
        o.stack_ptr = (stack_end as u8) - 1;
        PLA(&mut o);
        assert!(o.accumulator == 0x14);
        assert!(o.stack_ptr == stack_end as u8);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLP() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.bus.write(stack_end, 0x14);
        o.stack_ptr = (stack_end as u8) - 1;
        PLP(&mut o);
        assert!(o.status_reg == 0x14);
        assert!(o.stack_ptr == stack_end as u8);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_JMP() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x100;
        o.prog_ctr = 0xF;
        JMP(&mut o);
        assert!(o.prog_ctr == 0x100);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_JSR() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x1000;
        o.prog_ctr = 0xDEAD;
        JSR(&mut o);
        let lo = o.pop_from_stack();
        let hi = o.pop_from_stack();
        assert!(o.prog_ctr == 0x1000);
        assert!(lo == 0xAD);
        assert!(hi == 0xDE);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDA() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFA;
        LDA(&mut o);
        assert!(o.accumulator == 0xFA);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDX() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFA;
        LDX(&mut o);
        assert!(o.x_reg == 0xFA);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDY() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFA;
        LDY(&mut o);
        assert!(o.y_reg == 0xFA);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LSR_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x01;
        o.addr_abs = 0x100;
        o.opcode = 0x0A; // to get an ASL with the Accum addressing mode
        LSR(&mut o);
        assert!(o.accumulator == 0x00);
        assert!(o.get_flag(Flags6502::Z) == 1);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LSR_non_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x44;
        o.addr_abs = 0x100;
        LSR(&mut o);
        assert!(o.bus.read(o.addr_abs) == 0x22);
        assert!(o.get_flag(Flags6502::Z) == 0);
        assert!(o.get_flag(Flags6502::N) == 0);
        assert!(o.get_flag(Flags6502::C) == 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SEC() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::C, false);
        SEC(&mut o);
        assert!(o.get_flag(Flags6502::C) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SED() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::D, false);
        SED(&mut o);
        assert!(o.get_flag(Flags6502::D) == 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SEI() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::I, false);
        SEI(&mut o);
        assert!(o.get_flag(Flags6502::I) == 1);
    }
    // endregion
//endregion
}
