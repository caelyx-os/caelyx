use crate::drvs::{
    e9::print_fmt as e9_print_fmt, serial::print_fmt as serial_print_fmt,
    vga::print_fmt as vga_print_fmt,
};
use core::fmt::Arguments;

pub const LINE_ENDING: &str = "\r\n";

pub fn print_fmt(args: Arguments<'_>) {
    e9_print_fmt(args);
    serial_print_fmt(args);
    vga_print_fmt(args);
}

#[macro_export]
macro_rules! print {
    () => {};
    ($($arg:tt)*) => ($crate::misc::raw_print::print_fmt(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::misc::raw_print::print_fmt(format_args!("{}", $crate::misc::raw_print::LINE_ENDING))
    };
    ($($arg:tt)*) => {
        $crate::misc::raw_print::print_fmt(format_args!($($arg)*));
        $crate::println!();
    };
}
