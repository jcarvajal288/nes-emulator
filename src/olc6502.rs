#![allow(dead_code)]
extern crate hex;

use std::fs::File;
use std::io::prelude::*;
use std::io::LineWriter;

use super::bus;

static STACK_BASE: u16 = 0x0100;

enum Flags6502 {
    C = 1 << 0, // Carry Bit
    Z = 1 << 1, // Zero
    I = 1 << 2, // Disable Interrupts
    D = 1 << 3, // Decimal Mode (unused in nes)
    B = 1 << 4, // Break
    U = 1 << 5, // Unused
    V = 1 << 6, // Overflow
    N = 1 << 7, // Negative
}

pub struct Olc6502 {
    accumulator: u8,
    x_reg: u8,
    y_reg: u8,
    stack_ptr: u8,
    prog_ctr: u16,
    status_reg: u8,

    pub bus: bus::Bus,

    fetched_data: u8,
    addr_abs: u16,
    addr_rel: u16,
    opcode: u8,
    cycles: u8,

    lines_of_code: u32,

    lookup: [Instruction; 256],

    program_complete: bool,

    log_file: LineWriter<File>,
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

    pub fn clock(&mut self) {
        if self.cycles == 0 {
            self.lines_of_code += 1; // debug variable
            self.opcode = self.read(self.prog_ctr);
            let op_index = usize::from(self.opcode);
            if self.lookup[op_index].name == "BRK" && self.stack_ptr == 0 {
                self.program_complete = true;
                return;
            }             
            if self.lookup[op_index].name == "???" {
                println!("Invalid opcode: {:2X}.  Ending program.", op_index);
                self.program_complete = true;
                return;
            }
            self.log_state();
            self.prog_ctr += 1;

            // Get starting number of cycles
            self.cycles = self.lookup[op_index].cycles;

            // execute next instruction
            let additional_cycle1: u8 = (self.lookup[op_index].addrmode)(self);
            let additional_cycle2: u8 = (self.lookup[op_index].operate)(self);

            // add additional cycles if necessary
            self.cycles += additional_cycle1 & additional_cycle2;
        }

        self.cycles -= 1;
    }

    fn reset(&mut self) {
        // set program counter to known location
        self.addr_abs = 0xFFFC;
        let lo: u16 = self.bus.read(self.addr_abs) as u16;
        let hi: u16 = self.bus.read(self.addr_abs + 1) as u16;

        self.accumulator = 0;
        self.x_reg = 0;
        self.y_reg = 0;
        self.stack_ptr = 0xFD;
        self.prog_ctr = (hi << 8) | lo;
        self.status_reg = 0x24;

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
        return if (self.status_reg & f) > 0 {
            1
        } else {
            0
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

    fn log_state(&mut self) {
        let instr = &self.lookup[self.opcode as usize];
        let op = &instr.name;
        let mut args = ["  "; 2];
        let arg0 = format!("{:02X}", self.bus.read(self.prog_ctr + 1));
        let arg1 = format!("{:02X}", self.bus.read(self.prog_ctr + 2));
        if instr.num_bytes >= 2 {
            args[0] = &arg0;
        } 
        if instr.num_bytes == 3 {
            args[1] = &arg1;
        }
        let prog_ctr = format!("{:04X}", self.prog_ctr);
        let accumulator = format!("{:02X}", self.accumulator);
        let x_reg = format!("{:02X}", self.x_reg);
        let y_reg = format!("{:02X}", self.y_reg);
        let stack_ptr = format!("{:02X}", self.stack_ptr);
        let status_reg = format!("{:02X}", self.status_reg);
        let logline = format!("{} {} {} {}\t\tA:{} X:{} Y:{} P:{} SP:{}\n", prog_ctr, op, args[0], args[1], accumulator, x_reg, y_reg, status_reg, stack_ptr);
        self.log_file.write_all(logline.as_bytes()).expect("Unable to write to log file");
    }

    pub fn load_program(&mut self, program: String) {
        let program_address = 0x8000;
        self.bus.load_bytes_at(program_address, program);
    }

    pub fn run_program(&mut self) {
        // set reset vector
        self.bus.write(0xFFFC, 0x00);
        self.bus.write(0xFFFD, 0x80);
        self.reset();
        while self.program_complete == false {
            self.clock();
        }
    }

    pub fn run_automation(&mut self) {
        // set reset vector
        self.reset();
        self.prog_ctr = 0xC000;
        while self.program_complete == false{
            self.clock();
        }
    }

    pub fn set_log_file(&mut self, filename: &str) {
        let file = File::create(filename).unwrap();
        self.log_file = LineWriter::new(file);
    }

    fn stack_top(&self) -> u16 {
        return STACK_BASE | self.stack_ptr as u16;
    }

    fn run_interrupt(&mut self, inter_addr: u16, cycles: u8, b_flag: bool) {
        self.push_to_stack((self.prog_ctr >> 8) as u8);
        self.push_to_stack(self.prog_ctr as u8);
        let mut sr_copy = self.status_reg;
        if b_flag == true {
            sr_copy |= 0x30; 
        } else {
            sr_copy |= 0b0010_0000; // turn on bit 5
            sr_copy &= 0b1110_1111; // turn off bit 4
        }
        self.push_to_stack(sr_copy);

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
            self.run_interrupt(0xFFFE, 7, false);
        }
     }

    fn nmi(&mut self, b_flag: bool) {
        self.run_interrupt(0xFFFA, 8, b_flag);
    }

    fn push_to_stack(&mut self, data: u8) {
        let current_stack_location = STACK_BASE | self.stack_ptr as u16;
        self.bus.write(current_stack_location, data);
        self.stack_ptr = u8::wrapping_sub(self.stack_ptr, 1);
    }

    fn pop_from_stack(&mut self) -> u8 {
        // XXX - stack pointer overflow might be a bug?
        self.stack_ptr = u8::wrapping_add(self.stack_ptr, 1);
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

pub fn create_olc6502() -> Olc6502 {
    let path = std::path::Path::new("./log/olc6502.log");
    let prefix = path.parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap(); // create log directory if it doesn't exist
    let file = File::create(path).unwrap();
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
        lines_of_code: 0,
        lookup: populate_lookup_table(),
        program_complete: false,
        log_file: LineWriter::new(file),
    };
    o.reset();
    return o;
}

struct Instruction {
    name: String,
    operate: fn(&mut Olc6502) -> u8,
    addrmode: fn(&mut Olc6502) -> u8,
    num_bytes: u8,
    cycles: u8,
}

fn populate_lookup_table() -> [Instruction; 256] {
    fn i(name: &str, operate: fn(&mut Olc6502) -> u8, addrmode: fn(&mut Olc6502) -> u8, num_bytes: u8, cycles: u8) -> Instruction {
        return Instruction { name: String::from(name), operate, addrmode, num_bytes, cycles };
    }

    return [
        i("BRK", BRK, IMP, 1, 7), i("ORA", ORA, IZX, 2, 6), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("ORA", ORA, ZP0, 2, 3), i("ASL", ASL, ZP0, 2, 5), i("???", XXX, IMP, 0, 2), i("PHP", PHP, IMP, 1, 3), i("ORA", ORA, IMM, 2, 2), i("ASL", ASL, ACC, 1, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("ORA", ORA, ABS, 3, 4), i("ASL", ASL, ABS, 3, 6), i("???", XXX, IMP, 0, 2),
        i("BPL", BPL, REL, 2, 2), i("ORA", ORA, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("ORA", ORA, ZPX, 2, 4), i("ASL", ASL, ZPX, 2, 6), i("???", XXX, IMP, 0, 2), i("CLC", CLC, IMP, 1, 2), i("ORA", ORA, ABY, 3, 4), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("ORA", ORA, ABX, 3, 4), i("ASL", ASL, ABX, 3, 7), i("???", XXX, IMP, 0, 2), 
        i("JSR", JSR, ABS, 3, 6), i("AND", AND, IZX, 2, 6), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("BIT", BIT, ZP0, 2, 3), i("AND", AND, ZP0, 2, 3), i("ROL", ROL, ZP0, 2, 5), i("???", XXX, IMP, 0, 2), i("PLP", PLP, IMP, 1, 4), i("AND", AND, IMM, 2, 2), i("ROL", ROL, ACC, 1, 2), i("???", XXX, IMP, 0, 2), i("BIT", BIT, ABS, 3, 4), i("AND", AND, ABS, 3, 4), i("ROL", ROL, ABS, 3, 6), i("???", XXX, IMP, 0, 2), 
        i("BMI", BMI, REL, 2, 2), i("AND", AND, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("AND", AND, ZPX, 2, 4), i("ROL", ROL, ZPX, 2, 6), i("???", XXX, IMP, 0, 2), i("SEC", SEC, IMP, 1, 2), i("AND", AND, ABY, 3, 4), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("AND", AND, ABX, 3, 4), i("ROL", ROL, ABX, 3, 7), i("???", XXX, IMP, 0, 2), 
        i("RTI", RTI, IMP, 1, 6), i("EOR", EOR, IZX, 2, 6), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("EOR", EOR, ZP0, 2, 3), i("LSR", LSR, ZP0, 2, 5), i("???", XXX, IMP, 0, 2), i("PHA", PHA, IMP, 1, 3), i("EOR", EOR, IMM, 2, 2), i("LSR", LSR, ACC, 1, 2), i("???", XXX, IMP, 0, 2), i("JMP", JMP, ABS, 3, 3), i("EOR", EOR, ABS, 3, 4), i("LSR", LSR, ABS, 3, 6), i("???", XXX, IMP, 0, 2), 
        i("BVC", BVC, REL, 2, 2), i("EOR", EOR, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("EOR", EOR, ZPX, 2, 4), i("LSR", LSR, ZPX, 2, 6), i("???", XXX, IMP, 0, 2), i("CLI", CLI, IMP, 1, 2), i("EOR", EOR, ABY, 3, 4), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("EOR", EOR, ABX, 3, 4), i("LSR", LSR, ABX, 3, 7), i("???", XXX, IMP, 0, 2), 
        i("RTS", RTS, IMP, 1, 6), i("ADC", ADC, IZX, 2, 6), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("ADC", ADC, ZP0, 2, 3), i("ROR", ROR, ZP0, 2, 5), i("???", XXX, IMP, 0, 2), i("PLA", PLA, IMP, 1, 4), i("ADC", ADC, IMM, 2, 2), i("ROR", ROR, ACC, 1, 2), i("???", XXX, IMP, 0, 2), i("JMP", JMP, IND, 3, 5), i("ADC", ADC, ABS, 3, 4), i("ROR", ROR, ABS, 3, 6), i("???", XXX, IMP, 0, 2), 
        i("BVS", BVS, REL, 2, 2), i("ADC", ADC, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("ADC", ADC, ZPX, 2, 4), i("ROR", ROR, ZPX, 2, 6), i("???", XXX, IMP, 0, 2), i("SEI", SEI, IMP, 1, 2), i("ADC", ADC, ABY, 3, 4), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("ADC", ADC, ABX, 3, 4), i("ROR", ROR, ABX, 3, 7), i("???", XXX, IMP, 0, 2), 
        i("???", XXX, IMP, 0, 2), i("STA", STA, IZX, 2, 6), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("STY", STY, ZP0, 2, 3), i("STA", STA, ZP0, 2, 3), i("STX", STX, ZP0, 2, 3), i("???", XXX, IMP, 0, 2), i("DEY", DEY, IMP, 1, 2), i("???", XXX, IMP, 0, 2), i("TXA", TXA, IMP, 1, 2), i("???", XXX, IMP, 0, 2), i("STY", STY, ABS, 3, 4), i("STA", STA, ABS, 3, 4), i("STX", STX, ABS, 3, 4), i("???", XXX, IMP, 0, 2), 
        i("BCC", BCC, REL, 2, 2), i("STA", STA, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("STY", STY, ZPX, 2, 4), i("STA", STA, ZPX, 2, 4), i("STX", STX, ZPY, 2, 4), i("???", XXX, IMP, 0, 2), i("TYA", TYA, IMP, 1, 2), i("STA", STA, ABY, 3, 5), i("TXS", TXS, IMP, 1, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("STA", STA, ABX, 3, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), 
        i("LDY", LDY, IMM, 2, 2), i("LDA", LDA, IZX, 2, 6), i("LDX", LDX, IMM, 2, 2), i("???", XXX, IMP, 0, 2), i("LDY", LDY, ZP0, 2, 3), i("LDA", LDA, ZP0, 2, 3), i("LDX", LDX, ZP0, 2, 3), i("???", XXX, IMP, 0, 2), i("TAY", TAY, IMP, 1, 2), i("LDA", LDA, IMM, 2, 2), i("TAX", TAX, IMP, 1, 2), i("???", XXX, IMP, 0, 2), i("LDY", LDY, ABS, 3, 4), i("LDA", LDA, ABS, 3, 4), i("LDX", LDX, ABS, 3, 4), i("???", XXX, IMP, 0, 2), 
        i("BCS", BCS, REL, 2, 2), i("LDA", LDA, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("LDY", LDY, ZPX, 2, 4), i("LDA", LDA, ZPX, 2, 4), i("LDX", LDX, ZPY, 2, 4), i("???", XXX, IMP, 0, 2), i("CLV", CLV, IMP, 1, 2), i("LDA", LDA, ABY, 3, 4), i("TSX", TSX, IMP, 1, 2), i("???", XXX, IMP, 0, 2), i("LDY", LDY, ABX, 3, 4), i("LDA", LDA, ABX, 3, 4), i("LDX", LDX, ABY, 3, 4), i("???", XXX, IMP, 0, 2), 
        i("CPY", CPY, IMM, 2, 2), i("CMP", CMP, IZX, 2, 6), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("CPY", CPY, ZP0, 2, 3), i("CMP", CMP, ZP0, 2, 3), i("DEC", DEC, ZP0, 2, 5), i("???", XXX, IMP, 0, 2), i("INY", INY, IMP, 1, 2), i("CMP", CMP, IMM, 2, 2), i("DEX", DEX, IMP, 1, 2), i("???", XXX, IMP, 0, 2), i("CPY", CPY, ABS, 3, 4), i("CMP", CMP, ABS, 3, 4), i("DEC", DEC, ABS, 3, 6), i("???", XXX, IMP, 0, 2), 
        i("BNE", BNE, REL, 2, 2), i("CMP", CMP, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("CMP", CMP, ZPX, 2, 4), i("DEC", DEC, ZPX, 2, 6), i("???", XXX, IMP, 0, 2), i("CLD", CLD, IMP, 1, 2), i("CMP", CMP, ABY, 3, 4), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("CMP", CMP, ABX, 3, 4), i("DEC", DEC, ABX, 3, 7), i("???", XXX, IMP, 0, 2), 
        i("CPX", CPX, IMM, 2, 2), i("SBC", SBC, IZX, 2, 6), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("CPX", CPX, ZP0, 2, 3), i("SBC", SBC, ZP0, 2, 3), i("INC", INC, ZP0, 2, 5), i("???", XXX, IMP, 0, 2), i("INX", INX, IMP, 1, 2), i("SBC", SBC, IMM, 2, 2), i("NOP", NOP, IMP, 1, 2), i("???", XXX, IMP, 0, 2), i("CPX", CPX, ABS, 3, 4), i("SBC", SBC, ABS, 3, 4), i("INC", INC, ABS, 3, 6), i("???", XXX, IMP, 0, 2), 
        i("BEQ", BEQ, REL, 2, 2), i("SBC", SBC, IZY, 2, 5), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("SBC", SBC, ZPX, 2, 4), i("INC", INC, ZPX, 2, 6), i("???", XXX, IMP, 0, 2), i("SED", SED, IMP, 1, 2), i("SBC", SBC, ABY, 3, 4), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("???", XXX, IMP, 0, 2), i("SBC", SBC, ABX, 3, 4), i("INC", INC, ABX, 3, 7), i("???", XXX, IMP, 0, 2), 
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
    let fetched_addr = u16::from(o.read(o.prog_ctr));
    o.addr_abs = u16::wrapping_add(fetched_addr, o.x_reg as u16);
    o.prog_ctr += 1;
    o.addr_abs &= 0x00FF;
    return 0; 
}

#[allow(non_snake_case)]
fn ZPY(o: &mut Olc6502) -> u8 { // Indexed Zero Page Addressing Y
    let fetched_addr = u16::from(o.read(o.prog_ctr));
    o.addr_abs = u16::wrapping_add(fetched_addr, o.y_reg as u16);
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

    return if (o.addr_abs & 0xFF00) != (hi << 8) {
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
fn ABY(o: &mut Olc6502) -> u8 { // Indexed Absolute Addressing Y
    let lo: u16 = u16::from(o.read(o.prog_ctr));
    o.prog_ctr += 1;

    let hi: u16 = u16::from(o.read(o.prog_ctr));
    o.prog_ctr += 1;

    o.addr_abs = (hi << 8) | lo;
    o.addr_abs = u16::wrapping_add(o.addr_abs, o.y_reg as u16);

    return if (o.addr_abs & 0xFF00) != (hi << 8) {
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
fn IMP(_: &mut Olc6502) -> u8 { // Implied
    //o.fetched_data = o.accumulator;
    return 0; 
}

#[allow(non_snake_case)]
fn REL(o: &mut Olc6502) -> u8 { // Relative Addressing
    let mut argument: u16 = o.read(o.prog_ctr) as u16;
    o.prog_ctr += 1;
    if argument & 0x80 > 1 {
        argument |= 0xFF00;
    }
    o.addr_abs = u16::wrapping_add(o.prog_ctr, argument);
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
    o.addr_abs = u16::wrapping_add(o.addr_abs, o.y_reg as u16);

    return if (o.addr_abs & 0xFF00) != (hi << 8) {
        1
    } else {
        0
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
    let temp = u16::wrapping_add(a, data16);
    let sum: u16 = u16::wrapping_add(temp, flag_c);
    o.set_flag(Flags6502::C, sum & 0xFF00 > 0);
    o.set_flag(Flags6502::Z, (sum & 0x00FF) == 0);
    o.set_flag(Flags6502::N, (sum & 0x80) > 0);
    o.set_flag(Flags6502::V, !((a ^ data16) & 0x80 > 0) && ((a ^ sum) & 0x80) > 0);
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
    o.set_flag(Flags6502::N, fetched & (1 << 7) >= 1);
    o.set_flag(Flags6502::V, fetched & (1 << 6) >= 1);
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
// TODO: add test!
fn BRK(o: &mut Olc6502) -> u8 { // Force Break
    o.nmi(true);
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
    let sub_result = u8::wrapping_sub(o.accumulator, data);
    o.set_flag(Flags6502::C, o.accumulator >= data);
    o.set_flag(Flags6502::Z, o.accumulator == data);
    o.set_flag(Flags6502::N, sub_result >= 0x80);
    return 1; 
}

#[allow(non_snake_case)]
fn CPX(o: &mut Olc6502) -> u8 { // Compare Memory and Index X
    let data = o.fetch();
    let sub_result = u8::wrapping_sub(o.x_reg, data);
    o.set_flag(Flags6502::C, o.x_reg >= data);
    o.set_flag(Flags6502::Z, o.x_reg == data);
    o.set_flag(Flags6502::N, sub_result >= 0x80);
    return 1; 
}

#[allow(non_snake_case)]
fn CPY(o: &mut Olc6502) -> u8 { // Compare Memory And Index Y
    let data = o.fetch();
    let sub_result = u8::wrapping_sub(o.y_reg, data);
    o.set_flag(Flags6502::C, o.y_reg >= data);
    o.set_flag(Flags6502::Z, o.y_reg == data);
    o.set_flag(Flags6502::N, sub_result >= 0x80);
    return 1; 
}

#[allow(non_snake_case)]
fn DEC(o: &mut Olc6502) -> u8 { // Decrement Memory by One
    let data = o.fetch();
    let result = u8::wrapping_sub(data, 1);
    o.set_flag(Flags6502::N, (result & 0x80) > 1);
    o.set_flag(Flags6502::Z, result == 0x00);
    o.bus.write(o.addr_abs, result);
    return 0;
}

#[allow(non_snake_case)]
fn DEX(o: &mut Olc6502) -> u8 { // Decrement Index X by One
    o.x_reg = u8::wrapping_sub(o.x_reg, 1);
    o.set_flag(Flags6502::N, (o.x_reg & 0x80) > 1);
    o.set_flag(Flags6502::Z, o.x_reg == 0x00);
    return 0;
}

#[allow(non_snake_case)]
fn DEY(o: &mut Olc6502) -> u8 { // Decrement Index Y by One
    o.y_reg = u8::wrapping_sub(o.y_reg, 1);
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
    let temp = o.prog_ctr - 1;
    let hi = (temp >> 8) as u8;
    let lo = (temp & 0x00FF) as u8;
    o.push_to_stack(hi);
    o.push_to_stack(lo);
    o.prog_ctr = o.addr_abs;
    return 0;
}

#[allow(non_snake_case)]
fn LDA(o: &mut Olc6502) -> u8 { // Load Accumulator with Memory
    o.accumulator = o.fetch();
    o.set_flag(Flags6502::Z, o.accumulator == 0x00);
    o.set_flag(Flags6502::N, (o.accumulator & 0x80) > 1);
    return 1;
}

#[allow(non_snake_case)]
fn LDX(o: &mut Olc6502) -> u8 { // Load Index X with Memory
    o.x_reg = o.fetch();
    o.set_flag(Flags6502::Z, o.x_reg == 0x00);
    o.set_flag(Flags6502::N, (o.x_reg & 0x80) > 1);
    return 1;
}

#[allow(non_snake_case)]
fn LDY(o: &mut Olc6502) -> u8 { // Load Index Y with Memory
    o.y_reg = o.fetch();
    o.set_flag(Flags6502::Z, o.y_reg == 0x00);
    o.set_flag(Flags6502::N, (o.y_reg & 0x80) > 1);
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
    let data: u8 = o.fetch();
    o.accumulator |= data;
    o.set_flag(Flags6502::Z, o.accumulator == 0x00);
    o.set_flag(Flags6502::N, o.accumulator & 0x80 >= 1);
    return 1;
}

#[allow(non_snake_case)]
fn PHA(o: &mut Olc6502) -> u8 { // Push Accumulator on Stack
    o.push_to_stack(o.accumulator);
    return 0;
}

#[allow(non_snake_case)]
fn PHP(o: &mut Olc6502) -> u8 { // Push Processor Status on Stack
    let sr_copy = o.status_reg | 0x30; // set bits 5 and 4 (B flag)
    o.push_to_stack(sr_copy);
    return 0;
}

#[allow(non_snake_case)]
fn PLA(o: &mut Olc6502) -> u8 { // Pull Accumulator from Stack
    o.accumulator = o.pop_from_stack();
    o.set_flag(Flags6502::Z, o.accumulator == 0);
    o.set_flag(Flags6502::N, o.accumulator & 0x80 >= 1);
    return 0;
}

#[allow(non_snake_case)]
fn PLP(o: &mut Olc6502) -> u8 { // Pull Processor Status from Stack
    let temp = o.pop_from_stack();
    o.set_flag(Flags6502::C, temp & 0x1 > 0);
    o.set_flag(Flags6502::Z, temp & 0x2 > 0);
    o.set_flag(Flags6502::I, temp & 0x4 > 0);
    o.set_flag(Flags6502::D, temp & 0x8 > 0);
    // ignore bits 4 and 5
    o.set_flag(Flags6502::V, temp & 0x40 > 0);
    o.set_flag(Flags6502::N, temp & 0x80 > 0);
    return 0;
}

#[allow(non_snake_case)]
fn ROL(o: &mut Olc6502) -> u8 { // Rotate One Bit Left (Memory or Accumulator)
    let data = o.fetch();
    let flag_c = o.get_flag(Flags6502::C);
    o.set_flag(Flags6502::C, data & 0x80 > 0);
    let result: u8 = data << 1 | flag_c;
    o.set_flag(Flags6502::Z, result == 0);
    o.set_flag(Flags6502::N, result & 0x80 > 0);
    if o.lookup[o.opcode as usize].addrmode as usize == ACC as usize {
        o.accumulator = result;
    } else {
        o.bus.write(o.addr_abs, result);
    }
    return 0;
}

#[allow(non_snake_case)]
fn ROR(o: &mut Olc6502) -> u8 { // Rotate One Bit Right (Memory or Accumulator)
    let data = o.fetch();
    let flag_c = o.get_flag(Flags6502::C);
    o.set_flag(Flags6502::C, data & 0x1 > 0);
    let result: u8 = data >> 1 | flag_c << 7;
    o.set_flag(Flags6502::Z, result == 0);
    o.set_flag(Flags6502::N, result & 0x80 > 0);
    if o.lookup[o.opcode as usize].addrmode as usize == ACC as usize {
        o.accumulator = result;
    } else {
        o.bus.write(o.addr_abs, result);
    }
    return 0;
}

#[allow(non_snake_case)]
fn RTI(o: &mut Olc6502) -> u8 { // Return from Interrupt
    let temp = o.pop_from_stack();
    o.set_flag(Flags6502::C, temp & 0x1 > 0);
    o.set_flag(Flags6502::Z, temp & 0x2 > 0);
    o.set_flag(Flags6502::I, temp & 0x4 > 0);
    o.set_flag(Flags6502::D, temp & 0x8 > 0);
    // ignore bits 4 and 5
    o.set_flag(Flags6502::V, temp & 0x40 > 0);
    o.set_flag(Flags6502::N, temp & 0x80 > 0);

    o.prog_ctr = o.pop_from_stack() as u16;
    o.prog_ctr |= (o.pop_from_stack() as u16) << 8;
    return 0;
}

#[allow(non_snake_case)]
fn RTS(o: &mut Olc6502) -> u8 { // Return from Subroutine
    let mut temp = o.pop_from_stack() as u16;
    temp |= (o.pop_from_stack() as u16) << 8;
    o.prog_ctr = temp + 1;
    return 0;
}

#[allow(non_snake_case)]
fn SBC(o: &mut Olc6502) -> u8 { // Subtract Memory from Accumulator with Borrow
    let data: u8 = o.fetch();
    let inverted_data: u16 = (data as u16) ^ 0x00FF;
    add(o, inverted_data); 
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
    o.bus.write(o.addr_abs, o.accumulator);
    return 0;
}

#[allow(non_snake_case)]
fn STX(o: &mut Olc6502) -> u8 { // Store Index X in Memory
    o.bus.write(o.addr_abs, o.x_reg);
    return 0;
}

#[allow(non_snake_case)]
fn STY(o: &mut Olc6502) -> u8 { // Store Index Y in Memory
    o.bus.write(o.addr_abs, o.y_reg);
    return 0;
}

#[allow(non_snake_case)]
fn TAX(o: &mut Olc6502) -> u8 { // Transfer Accumulator to Index X
    o.x_reg = o.accumulator;
    o.set_flag(Flags6502::Z, o.x_reg == 0);
    o.set_flag(Flags6502::N, o.x_reg & 0x80 > 0);
    return 0;
}

#[allow(non_snake_case)]
fn TAY(o: &mut Olc6502) -> u8 { // Transfer Accumulator to Index Y
    o.y_reg = o.accumulator;
    o.set_flag(Flags6502::Z, o.y_reg == 0);
    o.set_flag(Flags6502::N, o.y_reg & 0x80 > 0);
    return 0;
}

#[allow(non_snake_case)]
fn TSX(o: &mut Olc6502) -> u8 { // Transfer Stack Pointer to Index X
    o.x_reg = o.stack_ptr;
    o.set_flag(Flags6502::Z, o.x_reg == 0);
    o.set_flag(Flags6502::N, o.x_reg & 0x80 > 0);
    return 0;
}

#[allow(non_snake_case)]
fn TXA(o: &mut Olc6502) -> u8 { // Transfer Index X to Accumulator
    o.accumulator = o.x_reg;
    o.set_flag(Flags6502::Z, o.accumulator == 0);
    o.set_flag(Flags6502::N, o.accumulator & 0x80 > 0);
    return 0;
}

#[allow(non_snake_case)]
fn TXS(o: &mut Olc6502) -> u8 { // Transfer Index X to Stack Register
    o.stack_ptr = o.x_reg;
    return 0;
}

#[allow(non_snake_case)]
fn TYA(o: &mut Olc6502) -> u8 { // Transfer Index Y to Accumulator
    o.accumulator = o.y_reg;
    o.set_flag(Flags6502::Z, o.accumulator == 0);
    o.set_flag(Flags6502::N, o.accumulator & 0x80 > 0);
    return 0;
}

#[allow(non_snake_case)]
fn XXX(_: &mut Olc6502) -> u8 { // Undefined Instruction
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
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::I), 1);
        assert_eq!(o.get_flag(Flags6502::D), 0);
        assert_eq!(o.get_flag(Flags6502::B), 1);
        assert_eq!(o.get_flag(Flags6502::U), 0);
        assert_eq!(o.get_flag(Flags6502::V), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    fn test_status_reg_write() {
        let mut o: Olc6502 = create_olc6502();
        o.status_reg = 0x00;
        o.set_flag(Flags6502::C, true);
        assert_eq!(o.get_flag(Flags6502::C), 1);

        o.set_flag(Flags6502::Z, true);
        assert_eq!(o.get_flag(Flags6502::Z), 1);

        o.set_flag(Flags6502::I, true);
        assert_eq!(o.get_flag(Flags6502::I), 1);

        o.set_flag(Flags6502::D, true);
        assert_eq!(o.get_flag(Flags6502::D), 1);

        o.set_flag(Flags6502::B, true);
        assert_eq!(o.get_flag(Flags6502::B), 1);

        o.set_flag(Flags6502::U, true);
        assert_eq!(o.get_flag(Flags6502::U), 1);

        o.set_flag(Flags6502::V, true);
        assert_eq!(o.get_flag(Flags6502::V), 1);

        o.set_flag(Flags6502::N, true);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_irq_and_RTI() {
        let mut o: Olc6502 = create_olc6502();
        o.prog_ctr = 0x11EC;
        o.status_reg = 0x0B;
        o.bus.write(0xFFFE, 0xAD);
        o.bus.write(0xFFFF, 0xDE);
        let old_pc = o.prog_ctr;
        let old_stack = o.stack_top();
        o.irq();
        assert_eq!(o.bus.read(0x01FD), 0x11);
        assert_eq!(o.bus.read(0x01FC), 0xEC);
        assert_eq!(o.bus.read(0x01FB), 0x2B);
        assert_eq!(o.prog_ctr, 0xDEAD);
        assert_eq!(o.get_flag(Flags6502::I), 1);
        assert_eq!(o.get_flag(Flags6502::B), 0);
        assert_eq!(o.get_flag(Flags6502::U), 1);
        RTI(&mut o);
        assert_eq!(o.prog_ctr, old_pc);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::I), 0);
        assert_eq!(o.get_flag(Flags6502::D), 1);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.stack_top(), old_stack);
    }

    #[test]
    fn test_nmi() {
        let mut o: Olc6502 = create_olc6502();
        o.prog_ctr = 0x11EC;
        o.status_reg = 0x28;
        o.bus.write(0xFFFA, 0xAD);
        o.bus.write(0xFFFB, 0xDE);
        o.nmi(false);
        assert_eq!(o.bus.read(0x01FD), 0x11);
        assert_eq!(o.bus.read(0x01FC), 0xEC);
        assert_eq!(o.bus.read(0x01FB), 0x28);
        assert_eq!(o.prog_ctr, 0xDEAD);
        assert_eq!(o.get_flag(Flags6502::I), 1);
        assert_eq!(o.get_flag(Flags6502::B), 0);
        assert_eq!(o.get_flag(Flags6502::U), 1);
    }

    // addressing mode tests
    // region
    #[test]
    #[allow(non_snake_case)]
    fn am_ACC_test() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x65;
        ACC(&mut o);
        assert_eq!(o.fetched_data, 0x65);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IMM_test() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = o.prog_ctr;
        IMM(&mut o);
        assert_eq!(o.addr_abs, current_addr);
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
        let new_addr: u16 = current_addr + u16::from(offset) + 1;
        assert_eq!(o.addr_abs, new_addr);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_REL_negative() {
        let mut o: Olc6502 = create_olc6502();
        let offset: u8 = 0xFA; 
        let current_addr: u16 = 0x8015;
        o.prog_ctr = current_addr;
        o.bus.write(current_addr, offset);
        REL(&mut o);
        assert_eq!(o.addr_abs, 0x8010);
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
        assert_eq!(o.addr_abs, 0x4032);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ZP0() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.bus.write(current_addr, 0x32);
        o.prog_ctr = current_addr;
        ZP0(&mut o);
        assert_eq!(o.addr_abs, 0x32);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ZPX_with_overflow() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.x_reg = 0x60;
        o.bus.write(current_addr, 0xC0);
        o.prog_ctr = current_addr;
        ZPX(&mut o);
        assert_eq!(o.addr_abs, 0x0020);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ZPY_with_overflow() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.y_reg = 0x60;
        o.bus.write(current_addr, 0xC0);
        o.prog_ctr = current_addr;
        ZPY(&mut o);
        assert_eq!(o.addr_abs, 0x0020);
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
        assert_eq!(o.addr_abs, 0x3A52);
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
        assert_eq!(o.addr_abs, 0xAA3A);
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
        assert_eq!(o.addr_abs, 0x4036);
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
        assert_eq!(o.addr_abs, 0x4036);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_ABY_overflow() {
        let mut o: Olc6502 = create_olc6502();
        let current_addr: u16 = 0x1000;
        o.y_reg = 0x01;
        o.bus.write(current_addr, 0xFF);
        o.bus.write(current_addr+1, 0xFF);
        o.prog_ctr = current_addr;
        ABY(&mut o);
        assert_eq!(o.addr_abs, 0x0000);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IZX_normal() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x24;
        o.bus.write(0x24, 0x74);
        o.bus.write(0x25, 0x20);
        IZX(&mut o);
        assert_eq!(o.addr_abs, 0x2074);
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
        assert_eq!(o.addr_abs, 0x2074);
    }

    #[test]
    #[allow(non_snake_case)]
    fn am_IZY() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x01;
        o.prog_ctr = 0x24;
        o.bus.write(0x24, 0x86);
        o.bus.write(0x86, 0xFF);
        o.bus.write(0x87, 0xFF);
        IZY(&mut o);
        assert_eq!(o.addr_abs, 0x0000);
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
        assert_eq!(o.accumulator, 0x41);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_AND_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xFF;
        o.fetched_data = 0x00;
        AND(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_AND_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xFF;
        o.fetched_data = 0xF0;
        AND(&mut o);
        assert_eq!(o.accumulator, 0xF0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ASL_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x80;
        o.addr_abs = 0x100;
        o.opcode = 0x0A; // to get an ASL with the Accum addressing mode
        ASL(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ASL_non_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x45;
        o.addr_abs = 0x100;
        ASL(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x8A);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_pos_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x04;
        o.fetched_data = 0x14;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0x18);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_pos_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x78;
        o.fetched_data = 0x78;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0xF0);
        assert_eq!(o.get_flag(Flags6502::V), 1);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_neg_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x78;
        o.fetched_data = 0xEC;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0x64);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_pos_neg_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x04;
        o.fetched_data = 0xF0;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0xF4);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_pos_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xFC;
        o.fetched_data = 0x60;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0x5C);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_pos_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x88;
        o.fetched_data = 0x04;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0x8C);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_neg_pos() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xD0;
        o.fetched_data = 0x90;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0x60);
        assert_eq!(o.get_flag(Flags6502::V), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_neg_neg_neg() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xF6;
        o.fetched_data = 0xF6;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0xEC);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ADC_zero_out() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x14;
        o.fetched_data = 0xEC;
        ADC(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BIT() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xF0;
        o.fetched_data = 0x0F;
        BIT(&mut o);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::V), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BIT_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xF0;
        o.fetched_data = 0xFF;
        BIT(&mut o);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::V), 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_BNE_short_jump_forward() {
        let mut o: Olc6502 = create_olc6502();
        let addr: u16 = 0x1000;
        let current_cycles: u8 = 2;
        o.prog_ctr = addr;
        o.addr_abs = addr + 0x0F;
        o.cycles = current_cycles;
        o.set_flag(Flags6502::Z, false);
        BNE(&mut o);
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
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
        assert_eq!(o.prog_ctr, addr); // no jump
        assert_eq!(o.cycles, current_cycles);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 1);
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
        assert_eq!(o.prog_ctr, o.addr_abs);
        assert_eq!(o.cycles, current_cycles + 2);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLC() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::C, true);
        CLC(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLD() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::D, true);
        CLD(&mut o);
        assert_eq!(o.get_flag(Flags6502::D), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLI() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::I, true);
        CLI(&mut o);
        assert_eq!(o.get_flag(Flags6502::I), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CLV() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::V, true);
        CLV(&mut o);
        assert_eq!(o.get_flag(Flags6502::V), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CMP_GT() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x81;
        o.fetched_data = 0x70;
        CMP(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CMP_Zero() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x70;
        o.fetched_data = 0x70;
        CMP(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CMP_LT() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x70;
        o.fetched_data = 0x81;
        CMP(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CMP_N_flag_example() {
        // taken from http://www.6502.org/tutorials/compare_beyond.html
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x01;
        o.fetched_data = 0xFF;
        CMP(&mut o);
        assert_eq!(o.accumulator, 0x01);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CMP_N_flag_example_2() {
        // taken from same page above
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x7F;
        o.fetched_data = 0x80;
        CMP(&mut o);
        assert_eq!(o.accumulator, 0x7F);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPX_GT() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x81;
        o.fetched_data = 0x70;
        CPX(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPX_Zero() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x70;
        o.fetched_data = 0x70;
        CPX(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPX_LT() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x70;
        o.fetched_data = 0x80;
        CPX(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPX_N_flag_example() {
        // taken from http://www.6502.org/tutorials/compare_beyond.html
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x01;
        o.fetched_data = 0xFF;
        CPX(&mut o);
        assert_eq!(o.x_reg, 0x01);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPX_N_flag_example_2() {
        // taken from same page above
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x7F;
        o.fetched_data = 0x80;
        CPX(&mut o);
        assert_eq!(o.x_reg, 0x7F);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }


    #[test]
    #[allow(non_snake_case)]
    fn op_CPY_GT() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x81;
        o.fetched_data = 0x70;
        CPY(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPY_Zero() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x70;
        o.fetched_data = 0x70;
        CPY(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPY_LT() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x70;
        o.fetched_data = 0x80;
        CPY(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPY_N_flag_example() {
        // taken from http://www.6502.org/tutorials/compare_beyond.html
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x01;
        o.fetched_data = 0xFF;
        CPY(&mut o);
        assert_eq!(o.y_reg, 0x01);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_CPY_N_flag_example_2() {
        // taken from same page above
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x7F;
        o.fetched_data = 0x80;
        CPY(&mut o);
        assert_eq!(o.y_reg, 0x7F);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }


    #[test]
    #[allow(non_snake_case)]
    fn op_DEC_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x70;
        DEC(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x6F);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEC_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x01;
        DEC(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x0);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEC_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x8F;
        DEC(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x8E);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEC_underflow() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x00;
        DEC(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0xFF);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEX_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x70;
        DEX(&mut o);
        assert_eq!(o.x_reg, 0x6F);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEX_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x01;
        DEX(&mut o);
        assert_eq!(o.x_reg, 0x0);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEX_underflow() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x00;
        DEX(&mut o);
        assert_eq!(o.x_reg, 0xFF);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEX_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x8F;
        DEX(&mut o);
        assert_eq!(o.x_reg, 0x8E);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEY_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x70;
        DEY(&mut o);
        assert_eq!(o.y_reg, 0x6F);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEY_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x01;
        DEY(&mut o);
        assert_eq!(o.y_reg, 0x0);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEY_underflow() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x00;
        DEY(&mut o);
        assert_eq!(o.y_reg, 0xFF);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_DEY_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x8F;
        DEY(&mut o);
        assert_eq!(o.y_reg, 0x8E);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_EOR_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x45;
        o.accumulator = 0x30;
        EOR(&mut o);
        assert_eq!(o.accumulator, 0x75);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_EOR_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x80;
        o.accumulator = 0x45;
        EOR(&mut o);
        assert_eq!(o.accumulator, 0xC5);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_EOR_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFF;
        o.accumulator = 0xFF;
        EOR(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INC_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x70;
        INC(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x71);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INC_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x10;
        o.fetched_data = 0xFF;
        INC(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x0);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INC_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x10;
        o.fetched_data = 0x80;
        INC(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x81);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INX_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x70;
        INX(&mut o);
        assert_eq!(o.x_reg, 0x71);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INX_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0xFF;
        INX(&mut o);
        assert_eq!(o.x_reg, 0x0);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INX_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0x80;
        INX(&mut o);
        assert_eq!(o.x_reg, 0x81);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INY_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x70;
        INY(&mut o);
        assert_eq!(o.y_reg, 0x71);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INY_memory_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0xFF;
        INY(&mut o);
        assert_eq!(o.y_reg, 0x0);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_INY_memory_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0x80;
        INY(&mut o);
        assert_eq!(o.y_reg, 0x81);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SBC_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x14;
        o.fetched_data = 0x04;
        o.set_flag(Flags6502::C, true);
        SBC(&mut o);
        assert_eq!(o.accumulator, 0x10);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SBC_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x40;
        o.fetched_data = 0x40;
        o.set_flag(Flags6502::C, true);
        SBC(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SBC_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x14;
        o.fetched_data = 0x15;
        o.set_flag(Flags6502::C, true);
        SBC(&mut o);
        assert_eq!(o.accumulator, 0xFF);
        assert_eq!(o.get_flag(Flags6502::V), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 0);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SBC_40_minus_0_carry_clear() {
        // taken from line 542 in nestest.log
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x80;
        o.fetched_data = 0x00;
        o.status_reg = 0xA4;
        SBC(&mut o);
        assert_eq!(o.accumulator, 0x7F);
        assert_eq!(o.status_reg, 0x65);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ORA_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xF9;
        o.fetched_data = 0x45;
        ORA(&mut o);
        assert_eq!(o.accumulator, 0xFD);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ORA_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x00;
        o.fetched_data = 0x00;
        ORA(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ORA_positive() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x0F;
        o.fetched_data = 0x00;
        ORA(&mut o);
        assert_eq!(o.accumulator, 0x0F);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PHA() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        let old_stack_ptr = o.stack_ptr;
        o.accumulator = 0x14;
        PHA(&mut o);
        assert_eq!(o.bus.read(stack_end), 0x14);
        assert_eq!(o.stack_ptr, old_stack_ptr - 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PHP() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        let old_stack_ptr = o.stack_ptr;
        o.status_reg = 0x14;
        PHP(&mut o);
        assert_eq!(o.bus.read(stack_end), 0x34); // account for 'B flag' pushed as side effect
        assert_eq!(o.stack_ptr, old_stack_ptr - 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLA_positive() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.bus.write(stack_end, 0x14);
        o.stack_ptr = (stack_end as u8) - 1;
        PLA(&mut o);
        assert_eq!(o.accumulator, 0x14);
        assert_eq!(o.stack_ptr, stack_end as u8);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLA_zero() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.bus.write(stack_end, 0x00);
        o.stack_ptr = (stack_end as u8) - 1;
        PLA(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.stack_ptr, stack_end as u8);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLA_negative() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.bus.write(stack_end, 0xF0);
        o.stack_ptr = (stack_end as u8) - 1;
        PLA(&mut o);
        assert_eq!(o.accumulator, 0xF0);
        assert_eq!(o.stack_ptr, stack_end as u8);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLP() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.bus.write(stack_end, 0x14);
        o.stack_ptr = (stack_end as u8) - 1;
        PLP(&mut o);
        assert_eq!(o.status_reg, 0x24); // account for ignoring bits 5 and 4
        assert_eq!(o.stack_ptr, stack_end as u8);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLP_flag_by_flag_all_positive() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.status_reg = 0x0;
        o.bus.write(stack_end, 0xFF);
        o.stack_ptr = (stack_end as u8) - 1;
        PLP(&mut o);
        assert_eq!(o.status_reg, 0xCF); // account for ignoring bits 5 and 4
        assert_eq!(o.stack_ptr, stack_end as u8);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_PLP_flag_by_flag_all_negative() {
        let mut o: Olc6502 = create_olc6502();
        let stack_end: u16 = STACK_BASE | o.stack_ptr as u16;
        o.status_reg = 0xFF;
        o.bus.write(stack_end, 0x00);
        o.stack_ptr = (stack_end as u8) - 1;
        PLP(&mut o);
        assert_eq!(o.status_reg, 0x30); // account for ignoring bits 5 and 4
        assert_eq!(o.stack_ptr, stack_end as u8);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_JMP() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x100;
        o.prog_ctr = 0xF;
        JMP(&mut o);
        assert_eq!(o.prog_ctr, 0x100);
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
        assert_eq!(o.prog_ctr, 0x1000);
        assert_eq!(lo, 0xAC);
        assert_eq!(hi, 0xDE);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_RTS() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x1000;
        o.prog_ctr = 0xDEAD;
        JSR(&mut o);
        assert_eq!(o.prog_ctr, 0x1000);
        RTS(&mut o);
        assert_eq!(o.prog_ctr, 0xDEAD);
    }
    #[test]
    #[allow(non_snake_case)]
    fn op_LDA() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x04;
        LDA(&mut o);
        assert_eq!(o.accumulator, 0x04);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }


    #[test]
    #[allow(non_snake_case)]
    fn op_LDA_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFA;
        LDA(&mut o);
        assert_eq!(o.accumulator, 0xFA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDA_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x00;
        LDA(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDX() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x04;
        LDX(&mut o);
        assert_eq!(o.x_reg, 0x04);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDX_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFA;
        LDX(&mut o);
        assert_eq!(o.x_reg, 0xFA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDX_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x00;
        LDX(&mut o);
        assert_eq!(o.x_reg, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDY() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x04;
        LDY(&mut o);
        assert_eq!(o.y_reg, 0x04);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDY_negative() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0xFA;
        LDY(&mut o);
        assert_eq!(o.y_reg, 0xFA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LDY_zero() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x00;
        LDY(&mut o);
        assert_eq!(o.y_reg, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }
    #[test]
    #[allow(non_snake_case)]
    fn op_LSR_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x01;
        o.addr_abs = 0x100;
        o.opcode = 0x0A; // to get an ASL with the Accum addressing mode
        LSR(&mut o);
        assert_eq!(o.accumulator, 0x00);
        assert_eq!(o.get_flag(Flags6502::Z), 1);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_LSR_non_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x44;
        o.addr_abs = 0x100;
        LSR(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x22);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ROL_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x80;
        o.addr_abs = 0x100;
        o.opcode = 0x0A; // to get an opcode with the Accum addressing mode
        o.set_flag(Flags6502::C, true);
        ROL(&mut o);
        assert_eq!(o.accumulator, 0x01);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
        assert_eq!(o.get_flag(Flags6502::C), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ROL_non_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x45;
        o.addr_abs = 0x100;
        o.set_flag(Flags6502::C, true);
        ROL(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0x8B);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ROR_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0x01;
        o.addr_abs = 0x100;
        o.opcode = 0x0A; // to get an opcode with the Accum addressing mode
        o.set_flag(Flags6502::C, true);
        ROR(&mut o);
        assert_eq!(o.accumulator, 0x80);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_ROR_non_ACC() {
        let mut o: Olc6502 = create_olc6502();
        o.fetched_data = 0x44;
        o.addr_abs = 0x100;
        o.set_flag(Flags6502::C, true);
        ROR(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), 0xA2);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
        assert_eq!(o.get_flag(Flags6502::C), 0);
    }


    #[test]
    #[allow(non_snake_case)]
    fn op_SEC() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::C, false);
        SEC(&mut o);
        assert_eq!(o.get_flag(Flags6502::C), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SED() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::D, false);
        SED(&mut o);
        assert_eq!(o.get_flag(Flags6502::D), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_SEI() {
        let mut o: Olc6502 = create_olc6502();
        o.set_flag(Flags6502::I, false);
        SEI(&mut o);
        assert_eq!(o.get_flag(Flags6502::I), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_STA() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x100;
        o.accumulator = 0xDE;
        STA(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), o.accumulator);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_STX() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x100;
        o.x_reg = 0xDE;
        STX(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), o.x_reg);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_STY() {
        let mut o: Olc6502 = create_olc6502();
        o.addr_abs = 0x100;
        o.y_reg = 0xDE;
        STY(&mut o);
        assert_eq!(o.bus.read(o.addr_abs), o.y_reg);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_TAX() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xEA;
        TAX(&mut o);
        assert_eq!(o.x_reg, 0xEA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_TAY() {
        let mut o: Olc6502 = create_olc6502();
        o.accumulator = 0xEA;
        TAY(&mut o);
        assert_eq!(o.y_reg, 0xEA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_TSX() {
        let mut o: Olc6502 = create_olc6502();
        o.stack_ptr = 0xEA;
        TSX(&mut o);
        assert_eq!(o.x_reg, 0xEA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_TXA() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0xEA;
        TXA(&mut o);
        assert_eq!(o.accumulator, 0xEA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_TXS() {
        let mut o: Olc6502 = create_olc6502();
        o.x_reg = 0xEA;
        TXS(&mut o);
        assert_eq!(o.stack_ptr, 0xEA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 0);
    }

    #[test]
    #[allow(non_snake_case)]
    fn op_TYA() {
        let mut o: Olc6502 = create_olc6502();
        o.y_reg = 0xEA;
        TYA(&mut o);
        assert_eq!(o.accumulator, 0xEA);
        assert_eq!(o.get_flag(Flags6502::Z), 0);
        assert_eq!(o.get_flag(Flags6502::N), 1);
    }
    // endregion

    // Functional tests
    // region
    #[test]
    fn load_program_into_memory() {
		let assembled_source: String = "A2 0A 8E 00 00 A2 03 8E 01 00 AC 00 00 A9 00 18 6D 01 00 88 D0 FA 8D 02 00 EA EA EA".to_string();
        let program_length: usize = assembled_source.split_whitespace().count();
        let mut o: Olc6502 = create_olc6502();
        o.bus.load_bytes_at(0x8000, assembled_source.clone());
        let read_program = o.bus.read_bytes_at(0x8000, program_length); 
        println!("{}", read_program);
        assert_eq!(read_program, assembled_source.replace(" ", ""));
    }

    #[test]
    fn multiply_10_by_3() {
        // Load Program (assembled at https://www.masswerk.at/6502/assembler.html)
		/*
			*=$8000
			LDX #10
			STX $0000
			LDX #3
			STX $0001
			LDY $0000
			LDA #0
			CLC
			loop
			ADC $0001
			DEY
			BNE loop
			STA $0002
			NOP
			NOP
			NOP
		*/
		let assembled_source: String = "A2 0A 8E 00 00 A2 03 8E 01 00 AC 00 00 A9 00 18 6D 01 00 88 D0 FA 8D 02 00 EA EA EA".to_string();
        let mut o: Olc6502 = create_olc6502();
        o.set_log_file("./log/multiply_10_by_3.log");
        o.load_program(assembled_source);
        o.run_program();
        assert_eq!(o.bus.read(0x0002), 0x1E);
    }

    #[test]
    fn short_loop() {
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
        let mut o: Olc6502 = create_olc6502();
        o.set_log_file("./log/short_loop.log");
        o.load_program(assembled_source);
        o.run_program();
        assert_eq!(o.bus.read(0x0201), 0x03);
    }
//endregion
}
