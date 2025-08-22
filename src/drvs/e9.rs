use crate::{
    misc::str_writer::StrWriter,
    x86::ioport::{inb, outb},
};

use core::fmt::{Arguments, Write};

pub fn find_e9() -> Option<u16> {
    if inb(0xE9) != 0xE9 { None } else { Some(0xE9) }
}

pub fn init() {
    if find_e9().is_some() {}
}

pub fn print_fmt(args: Arguments<'_>) {
    let _ = StrWriter {
        write: |s| {
            for c in s.chars() {
                outb(0xE9, (if c.is_ascii() { c } else { '.' }) as u8);
            }
        },
    }
    .write_fmt(args);
}
