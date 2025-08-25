use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{
    debug,
    misc::{
        isituninit::IsItUninit,
        ptr_align::{align_ptr_down, align_ptr_up},
    },
    sync::mutex::Mutex,
    trace,
};

unsafe extern "C" {
    static KERNEL_START: core::ffi::c_void;
    static KERNEL_END: core::ffi::c_void;
}

const BITMAP_SIZE: usize = usize::MAX / 4 / 8 / 4096;
static BITMAP: [AtomicU8; BITMAP_SIZE] = unsafe { MaybeUninit::zeroed().assume_init() };

pub struct VirtPageAllocator {}

impl VirtPageAllocator {
    pub fn new() -> Self {
        Self {}
    }

    fn set_bit(bit: usize, to: bool) {
        let mut val = BITMAP[bit / 8].load(Ordering::Acquire);
        if to {
            val |= 1 << (bit % 8);
        } else {
            val &= !(1 << (bit % 8));
        }
        BITMAP[bit / 8].store(val, Ordering::Release);
    }

    fn get_bit(bit: usize) -> bool {
        let val = BITMAP[bit / 8].load(Ordering::Acquire);
        (val & (1 << (bit % 8))) != 0
    }

    fn bitmap_size() -> usize {
        BITMAP.len() * 8
    }

    fn take_bits(&self, count: usize) -> Option<usize> {
        let mut bit = 0;
        let mut bits = 0;

        loop {
            if bit + bits >= Self::bitmap_size() {
                break None;
            }

            if Self::get_bit(bit + bits) {
                bit += bits + 1;
                bits = 0;
            } else {
                Self::set_bit(bit + bits, true);
                bits += 1;

                if bits == count {
                    break Some(bit);
                }
            }
        }
    }

    pub fn allocate(&self, count: usize) -> Option<u32> {
        if let Some(bit) = self.take_bits(count) {
            let page_offset = bit * 4096;
            let page = usize::MAX - usize::MAX / 4 + page_offset;
            debug!("Allocated {count} pages at 0x{page:08X}");
            Some(page as u32)
        } else {
            None
        }
    }

    pub fn free(&self, addr: usize, count: usize) {
        if addr < usize::MAX - usize::MAX / 4 || !addr.is_multiple_of(4096) {
            panic!("Invalid free 0x{addr:08X}");
        }

        let start_bit = (addr - usize::MAX - usize::MAX / 4) / 4096;
        let mut bit = start_bit;
        while bit < start_bit + count {
            if !Self::get_bit(bit) {
                panic!("Double free 0x{addr:08X}");
            }

            Self::set_bit(bit, false);

            bit += 1;
        }

        debug!("Freed {count} pages at 0x{addr:08X}");
    }
}

impl Default for VirtPageAllocator {
    fn default() -> Self {
        Self::new()
    }
}

static VIRT_PAGE_ALLOC: Mutex<IsItUninit<VirtPageAllocator>> = Mutex::new(IsItUninit::uninit());

pub fn init() {
    let mut lock = VIRT_PAGE_ALLOC.lock();
    lock.write(VirtPageAllocator::new());
}

pub fn allocate(count: usize) -> Option<u32> {
    let lock = VIRT_PAGE_ALLOC.lock();
    lock.get_ref().allocate(count)
}

pub fn free(addr: *const u8, count: usize) {
    let lock = VIRT_PAGE_ALLOC.lock();
    lock.get_ref().free(addr as usize, count);
}
