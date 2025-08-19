use crate::{sync::mutex::Mutex, util::isituninit::IsItUninit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VGAColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VGAColorPair {
    pub fg: VGAColor,
    pub bg: VGAColor,
}

impl VGAColorPair {
    pub fn to_u8(&self) -> u8 {
        // VGA color attributes expect the background part of the color to be in the upper half of
        // the byte and the foreground part of the color to be in the lower half of the byte
        ((self.bg as u8) << 4) | (self.fg as u8)
    }
}

impl Default for VGAColorPair {
    fn default() -> VGAColorPair {
        // This is a reasonable default, i would think?
        VGAColorPair {
            fg: VGAColor::White,
            bg: VGAColor::Black,
        }
    }
}

pub struct VGADriver<'a> {
    // We use a slice to hide the unsafeness of a raw pointer behind a rust abstraction
    vram: &'a mut [u8],
    // This will really always be 80, good for futureproofing in case some day i will feel evil
    // enough to add VGA mode switching
    width: usize,
    // Same here except it's 25
    height: usize,
    // This just keeps track of the position the cursor is at
    x: usize,
    y: usize,
}

impl<'a> VGADriver<'a> {
    pub const fn new<'b>() -> VGADriver<'b> {
        // All of this is explained in the struct definition
        VGADriver {
            vram: unsafe { core::slice::from_raw_parts_mut(0xb8000 as *mut u8, 80 * 25 * 2) },
            width: 80,
            height: 25,
            x: 0,
            y: 0,
        }
    }

    fn coords_to_idx(&self, x: usize, y: usize) -> usize {
        // VGA VRAM is structured in this way:
        // Offset Value
        // 0      CP437 character (X=0 Y=0)
        // 1      Color attribute (X=0 Y=0)
        // 2      CP437 character (X=1 Y=0)
        // 3      Color attribute (X=1 Y=0)
        // ...
        // 160    CP437 character (X=0 Y=1)
        // 161    Color attribute (X=0 Y=1)
        // So we need to calculate the pixel index which is pretty easily doable by multiplying the
        // row by the width and adding the column. Then we need to multiply it by 2 since each cell
        // takes 2 bytes
        (y * self.width + x) * 2
    }

    fn set_at(&mut self, x: usize, y: usize, c: char, col: VGAColorPair) {
        // Check if the passed position is out of bounds
        if x >= self.width || y >= self.height {
            return;
        }

        // We should support full CP437 but we don't really need that
        let c = if c.is_ascii() { c as u8 } else { b'.' };
        let idx = self.coords_to_idx(x, y);

        // See coords_to_idx for structure of VRAM
        self.vram[idx] = c;
        self.vram[idx + 1] = col.to_u8();
    }

    fn clear_row(&mut self, y: usize, bg: VGAColor) {
        // We need to go through each column and clear it.
        for x in 0..self.width {
            self.set_at(x, y, ' ', VGAColorPair { fg: bg, bg })
        }
    }

    fn next_row(&mut self, bg: VGAColor) {
        self.y += 1;
        if self.y >= self.height {
            self.y -= 1;
            self.scroll(bg);
        }
    }

    fn next_char(&mut self, bg: VGAColor) {
        self.x += 1;
        if self.x >= self.width {
            self.x = 0;
            self.next_row(bg);
        }
    }

    fn scroll(&mut self, bg: VGAColor) {
        // rotate_left rotates all data left by the parameter passed so [1,2,3,4].rotate_left(2) ==
        // [3,4,1,2]
        self.vram.rotate_left(self.width * 2);
        // Then we need to wipe the last row to make space
        self.clear_row(self.height - 1, bg);
    }

    pub fn clear(&mut self, bg: VGAColor) {
        // We need to go through each row and clear it.
        for y in 0..self.height {
            self.clear_row(y, bg);
        }
    }

    pub fn print_char(&mut self, c: char, color: VGAColorPair) {
        match c {
            '\n' => {
                // \n is just a newline so new row
                self.next_row(color.bg);
            }

            '\r' => {
                self.x = 0;
            }

            '\x08' => {
                // for some reason rust doesn't like \b so we have to do \x08, it just skips
                // backwards one column
                if self.x > 0 {
                    self.x -= 1;
                }
            }

            '\t' => {
                // All a tab really is is a couple of spaces, imo
                for _ in 0..4 {
                    self.print_char(' ', color);
                }
            }

            c => {
                self.set_at(self.x, self.y, c, color);
                self.next_char(color.bg);
            }
        }
    }

    pub fn print<'b>(&'b mut self, s: &'b str, color: VGAColorPair) {
        for c in s.chars() {
            self.print_char(c, color);
        }
    }
}

static INSTANCE: Mutex<IsItUninit<VGADriver>> = Mutex::new(IsItUninit::new());

pub fn init() {
    let mut drv = VGADriver::new();
    drv.clear(VGAColorPair::default().bg);
    INSTANCE.lock().write(drv);
}

pub fn print(str: &str) -> usize {
    if let Some(vga) = INSTANCE.lock().try_get_mut() {
        vga.print(str, VGAColorPair::default());
        str.len()
    } else {
        0
    }
}
