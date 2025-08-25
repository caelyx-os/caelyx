use crate::{
    misc::str_writer::StrWriter,
    x86::ioport::{inb, outb},
};

use core::fmt::{Arguments, Write};

pub fn init() {}

pub fn print_fmt(args: Arguments<'_>) {
    let _ = StrWriter {
        write: |s| {
            if inb(0xE9) == 0xE9 {
                for c in s.chars() {
                    outb(0xE9, (if c.is_ascii() { c } else { '.' }) as u8);
                }
            }
        },
    }
    .write_fmt(args);
}
