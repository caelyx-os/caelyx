use core::fmt::Arguments;

use crate::{
    misc::{
        isituninit::IsItUninit,
        output::raw_print::{print_fmt, print_line_ending},
    },
    sync::mutex::Mutex,
    x86::halt,
};

static LOGGER: Mutex<IsItUninit<Logger>> = Mutex::new(IsItUninit::uninit());

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    pub fn should_display(&self, level: &LogLevel) -> bool {
        self >= level
    }
}

impl core::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let txt = match self {
            LogLevel::Trace => "\x1b[34mTRACE\x1b[0m",
            LogLevel::Debug => "\x1b[1;34mDEBUG\x1b[0m",
            LogLevel::Info => "\x1b[36mINFO\x1b[0m ",
            LogLevel::Warning => "\x1b[1;33mWARN\x1b[0m ",
            LogLevel::Error => "\x1b[31mERROR\x1b[0m",
            LogLevel::Fatal => "\x1b[1;31mFATAL\x1b[0m",
        };

        write!(f, "{txt}")
    }
}

#[derive(Clone, Copy)]
pub struct Log<'a> {
    level: LogLevel,
    line: u32,
    file: &'static str,
    args: Arguments<'a>,
}

#[derive(Clone, Copy)]
pub struct Logger {
    level: LogLevel,
}

impl Logger {
    pub const fn new(level: LogLevel) -> Self {
        Self { level }
    }

    pub fn log(&mut self, log: Log) {
        if log.level.should_display(&self.level) {
            print_fmt(format_args!("{} {}:{} ", log.level, log.file, log.line));
            print_fmt(log.args);
            print_line_ending();
        }
    }
}

pub fn init() {
    let mut lock = LOGGER.lock();
    lock.write(Logger::new(LogLevel::Trace));
}

pub fn log(level: LogLevel, file: &'static str, line: u32, args: Arguments<'_>) {
    let mut lock = LOGGER.lock();
    if let Some(logger) = lock.try_get_mut() {
        logger.log(Log {
            file,
            line,
            level,
            args,
        });
    }
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ($crate::misc::output::logger::log($crate::misc::output::logger::LogLevel::Trace, file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::misc::output::logger::log($crate::misc::output::logger::LogLevel::Debug, file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::misc::output::logger::log($crate::misc::output::logger::LogLevel::Info, file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => ($crate::misc::output::logger::log($crate::misc::output::logger::LogLevel::Warning, file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::misc::output::logger::log($crate::misc::output::logger::LogLevel::Error, file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => ($crate::misc::output::logger::log($crate::misc::output::logger::LogLevel::Fatal, file!(), line!(), format_args!($($arg)*)));
}
