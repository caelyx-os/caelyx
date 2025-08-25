use crate::{
    misc::{isituninit::IsItUninit, str_writer::StrWriter},
    sync::mutex::Mutex,
    x86::ioport::{inb, outb},
};

use core::fmt::{Arguments, Write};

pub struct SerialPort {
    num: u8,
    port: u16,
}

const MAX_SERIAL_PORTS: usize = 4;
type SerialPortArr = [IsItUninit<SerialPort>; MAX_SERIAL_PORTS];
static PORTS: Mutex<IsItUninit<SerialPortArr>> = Mutex::new(IsItUninit::uninit());

impl SerialPort {
    pub const fn new(num: u8, port: u16) -> Self {
        Self { num, port }
    }

    pub fn write(&self, c: char) {
        let c = if c.is_ascii() { c } else { '.' };
        let c = c as u8;

        while (inb(self.port + 5) & 0x20) == 0 {}

        outb(self.port, c);
    }
}

pub struct SerialPortIterator<'a> {
    current_idx: usize,
    serial_ports: &'a [(u8, u16)],
}

impl<'a> SerialPortIterator<'a> {
    pub fn new(serial_ports: &'a [(u8, u16)]) -> SerialPortIterator<'a> {
        if serial_ports.len() > MAX_SERIAL_PORTS {
            panic!(
                "SerialPortIterator::new({:?}): {:?}.len() ({}) > MAX_SERIAL_PORTS ({})",
                serial_ports,
                serial_ports,
                serial_ports.len(),
                MAX_SERIAL_PORTS
            );
        }

        SerialPortIterator::<'a> {
            current_idx: 0,
            serial_ports,
        }
    }
}

impl<'a> Iterator for SerialPortIterator<'a> {
    type Item = (u8, u16);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx >= self.serial_ports.len() {
            None
        } else {
            let (num, port) = self.serial_ports[self.current_idx];

            outb(port + 1, 0x00); // disable irqs
            outb(port + 3, 0x80); // enable dlab
            outb(port, 0x03); // 115200 / 3(0x0003) = 38400 (bits 0-7  of divisor)
            outb(port + 1, 0x00); // (bits 8-15 of divisor)
            outb(port + 3, 0x03); // 8 bits, no parity
            outb(port + 2, 0xC7); // enable fifo
            outb(port + 4, 0b10010); // enable loopback

            // test loopback
            outb(port, 69);
            if inb(port) != 69 {
                self.current_idx += 1;
                return self.next();
            }

            outb(port + 4, 0b11); // disable loopback

            self.current_idx += 1;
            Some((num, port))
        }
    }
}

pub fn init() {
    let iter = SerialPortIterator::new(&[(1, 0x3F8), (2, 0x2F8), (3, 0x3E8), (4, 0x2E8)]);
    let mut lock = PORTS.lock();
    lock.write(SerialPortArr::default());

    for (i, (idx, port)) in iter.enumerate() {
        lock.get_mut()[i] = IsItUninit::init(SerialPort::new(idx, port));
    }
}

fn write(c: char) {
    let lock = PORTS.lock();
    if !lock.initialized() {
        return;
    }

    for port in lock.get_ref() {
        if let Some(port) = port.try_get_ref() {
            port.write(c);
        }
    }
}

pub fn print_fmt(args: Arguments<'_>) {
    let _ = StrWriter {
        write: |s| {
            for c in s.chars() {
                write(c);
            }
        },
    }
    .write_fmt(args);
}
