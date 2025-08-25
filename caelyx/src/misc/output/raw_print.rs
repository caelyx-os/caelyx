use crate::drvs::{
    e9::print_fmt as e9_print_fmt, serial::print_fmt as serial_print_fmt,
    vga::print_fmt as vga_print_fmt,
};
use core::fmt::Arguments;

const LINE_ENDING: &str = "\r\n";

pub fn print_fmt(args: Arguments<'_>) {
    e9_print_fmt(args);
    serial_print_fmt(args);
    vga_print_fmt(args);
}

pub fn print_line_ending() {
    print_fmt(format_args!("{LINE_ENDING}"));
}
