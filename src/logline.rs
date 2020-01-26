#![allow(dead_code)]

extern crate regex;

use regex::Regex;

/*
 * Test helper file used for comparing my log vs Nintendulator's log
 */
pub struct Logline<'a> {
    prog_ctr: &'a str,
    accumulator: &'a str,
    x_reg: &'a str,
    y_reg: &'a str,
    status_reg: &'a str,
    stack_ptr: &'a str,
}

impl<'a> PartialEq for Logline<'a> {
    fn eq(&self, other: &Logline) -> bool {
        self.prog_ctr == other.prog_ctr &&
        self.accumulator == other.accumulator &&
        self.x_reg == other.x_reg &&
        self.y_reg == other.y_reg &&
        self.status_reg == other.status_reg &&
        self.stack_ptr == other.stack_ptr
    }
}

pub fn parse_my_line<'a>(line: &'a String) -> Logline<'a> {
    let splits: Vec<&str> = line.split_whitespace().collect();
    let len = splits.len();
    return Logline {
        prog_ctr: splits[0],
        stack_ptr: splits[len-1],
        status_reg: splits[len-2],
        y_reg: splits[len-3],
        x_reg: splits[len-4],
        accumulator: splits[len-5],
    }
}

pub fn parse_their_line<'a>(line: &'a String) -> Logline<'a> {
    lazy_static! {
        static ref A_REG:  Regex = Regex::new("A:.{2}").unwrap();
        static ref X_REG:  Regex = Regex::new("X:.{2}").unwrap();
        static ref Y_REG:  Regex = Regex::new("Y:.{2}").unwrap();
        static ref P_REG:  Regex = Regex::new("P:.{2}").unwrap();
        static ref SP_REG: Regex = Regex::new("SP:.{2}").unwrap();
    }
    let splits: Vec<&str> = line.split_whitespace().collect();
    return Logline {
        prog_ctr: splits[0],
        accumulator: A_REG.find(line).unwrap().as_str(),
        x_reg: X_REG.find(line).unwrap().as_str(),
        y_reg: Y_REG.find(line).unwrap().as_str(),
        status_reg: P_REG.find(line).unwrap().as_str(),
        stack_ptr: SP_REG.find(line).unwrap().as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_my_log_line() {
        let line = "C000 JMP F5 C5		A:00 X:00 Y:00 P:24 SP:FD".to_string();
        let logline = parse_my_line(&line);
        assert!(logline.prog_ctr == "C000");
        assert!(logline.stack_ptr == "SP:FD");
        assert!(logline.status_reg == "P:24");
        assert!(logline.y_reg == "Y:00");
        assert!(logline.x_reg == "X:00");
        assert!(logline.accumulator == "A:00");
    }

    #[test]
    fn parse_their_log_line_separate_ppu() {
        let line = "C7A6  70 03     BVS $C7AB                       A:00 X:00 Y:00 P:26 SP:FB PPU: 34,  1 CYC:132".to_string();
        let logline = parse_their_line(&line);
        assert!(logline.prog_ctr == "C7A6");
        assert!(logline.stack_ptr == "SP:FB");
        assert!(logline.status_reg == "P:26");
        assert!(logline.y_reg == "Y:00");
        assert!(logline.x_reg == "X:00");
        assert!(logline.accumulator == "A:00");
    }

    #[test]
    fn parse_their_log_line_joined_ppu() {
        let line = "C740  EA        NOP                             A:00 X:00 Y:00 P:26 SP:FB PPU:108,  0 CYC:43".to_string();
        let logline = parse_their_line(&line);
        assert!(logline.prog_ctr == "C740");
        assert!(logline.stack_ptr == "SP:FB");
        assert!(logline.status_reg == "P:26");
        assert!(logline.y_reg == "Y:00");
        assert!(logline.x_reg == "X:00");
        assert!(logline.accumulator == "A:00");
    }

    #[test]
    fn parse_their_log_line_over_100_frame_buffer() {
        let line = "DD1F  F5 00     SBC $00,X @ 78 = 40             A:40 X:78 Y:1D P:65 SP:FB PPU: 10,100 CYC:11377".to_string();
        let logline = parse_their_line(&line);
        assert!(logline.prog_ctr == "DD1F");
        assert!(logline.stack_ptr == "SP:FB");
        assert!(logline.status_reg == "P:65");
        assert!(logline.y_reg == "Y:1D");
        assert!(logline.x_reg == "X:78");
        assert!(logline.accumulator == "A:40");
    }
}