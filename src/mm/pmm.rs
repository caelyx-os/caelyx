use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{
    boot::multiboot2::{
        _MultibootMmapPart, MultibootMmapEntryType, MultibootTag,
        TagIterator as MultibootTagIterator,
    },
    debug,
    misc::ptr_align::{align_ptr_down, align_ptr_up},
    trace,
};

unsafe extern "C" {
    static KERNEL_START: core::ffi::c_void;
    static KERNEL_END: core::ffi::c_void;
}

const BITMAP_SIZE: usize = usize::MAX / 8;
static BITMAP: [AtomicU8; BITMAP_SIZE] = unsafe { MaybeUninit::zeroed().assume_init() };

struct FreeRegionIterator<'a> {
    mmap_iter: core::slice::Iter<'a, _MultibootMmapPart>,
}

impl<'a> FreeRegionIterator<'a> {
    pub fn new(mmap_tag: &'a [_MultibootMmapPart]) -> Self {
        Self {
            mmap_iter: mmap_tag.iter(),
        }
    }
}

impl Iterator for FreeRegionIterator<'_> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        const MAX: u64 = usize::MAX as u64;

        let entry = self.mmap_iter.next()?.as_mmap_entry();

        if entry.type_ != MultibootMmapEntryType::Available {
            return self.next();
        }

        if entry.start > MAX {
            return self.next();
        }

        let start = entry.start as usize;
        let size = if entry.start + entry.size > MAX {
            MAX as usize - start
        } else {
            entry.size as usize
        };

        Some((start, size))
    }
}

#[allow(dead_code)]
struct PhysicalMemoryAllocatorBlock {
    first_page: usize,
    last_page: usize,
    size: usize,
    page_count: usize,
}

pub struct PhysicalMemoryAllocator {
    block: PhysicalMemoryAllocatorBlock,
}

#[derive(Debug)]
pub enum PhysicalMemoryAllocatorNewError {
    CouldNotFindMmap,
}

impl PhysicalMemoryAllocator {
    pub fn new(
        mut tag_iter: MultibootTagIterator,
    ) -> Result<Self, PhysicalMemoryAllocatorNewError> {
        let mmap = tag_iter
            .find(|x| matches!(x, MultibootTag::Mmap(_)))
            .ok_or(PhysicalMemoryAllocatorNewError::CouldNotFindMmap)?;

        let free_region_iterator = match mmap {
            MultibootTag::Mmap(map) => FreeRegionIterator::new(map),
            _ => unreachable!(),
        };

        let kernel_start = (&raw const KERNEL_START) as usize;
        let kernel_end = (&raw const KERNEL_END) as usize;
        trace!("kernel: 0x{:08X}-0x{:08X}", kernel_start, kernel_end);
        let mut current_biggest = (0, 0);
        for (start, size) in free_region_iterator {
            let end = start + size;

            if !(kernel_start >= start && kernel_end <= end) {
                if let Some(chunk) = process_chunk(start, end - start, current_biggest) {
                    current_biggest = chunk;
                }
            } else {
                if kernel_start > start
                    && let Some(chunk) = process_chunk(start, kernel_start - start, current_biggest)
                {
                    current_biggest = chunk;
                }

                if kernel_end < end
                    && let Some(chunk) =
                        process_chunk(kernel_end, end - kernel_end, current_biggest)
                {
                    current_biggest = chunk;
                }
            }

            fn process_chunk(
                start: usize,
                size: usize,
                biggest: (usize, usize),
            ) -> Option<(usize, usize)> {
                let first_page = align_ptr_up(start as *const u8, 4096) as usize;
                if first_page > start + size {
                    return None;
                }

                let new_size = size - (first_page - start);

                let last_page = align_ptr_down((first_page + new_size) as *const u8, 4096) as usize;

                if first_page == last_page {
                    return None;
                }

                let page_count = (last_page - first_page) / 4096;

                trace!("Found {page_count} pages at 0x{first_page:08X}-0x{last_page:08X}");

                if page_count > biggest.1 {
                    Some((first_page, page_count))
                } else {
                    None
                }
            }
        }

        let page_count = current_biggest.1;
        let first_page = current_biggest.0;
        let size = 4096 * page_count;
        let last_page = first_page + size;
        debug!("Using {page_count} pages at 0x{first_page:08X}-0x{last_page:08X}");

        Ok(Self {
            block: PhysicalMemoryAllocatorBlock {
                page_count,
                first_page,
                last_page,
                size,
            },
        })
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
            if bit + bits >= PhysicalMemoryAllocator::bitmap_size() {
                break None;
            }

            if PhysicalMemoryAllocator::get_bit(bit + bits) {
                bit += bits + 1;
                bits = 0;
            } else {
                PhysicalMemoryAllocator::set_bit(bit + bits, true);
                bits += 1;

                if bits == count {
                    break Some(bit);
                }
            }
        }
    }

    pub fn allocate(&self, count: usize) -> Option<*const u8> {
        if let Some(bit) = self.take_bits(count) {
            let page_offset = bit * 4096;
            let start_page = self.block.first_page;
            let page = start_page + page_offset;
            Some(page as *const u8)
        } else {
            None
        }
    }

    pub fn free(&self, addr: usize, count: usize) {
        if addr < self.block.first_page
            || addr + count * 4096 > self.block.last_page
            || !addr.is_multiple_of(4096)
        {
            panic!("Invalid free {addr:08X}");
        }

        let start_bit = (addr - self.block.first_page) / 4096;
        let mut bit = start_bit;
        while bit < start_bit + count {
            if !PhysicalMemoryAllocator::get_bit(bit) {
                panic!("Double free {addr:08X}");
            }

            PhysicalMemoryAllocator::set_bit(bit, false);

            bit += 1;
        }
    }
}

pub fn init(tag_iter: &mut MultibootTagIterator) {
    let pmm = PhysicalMemoryAllocator::new(*tag_iter).expect("Could not create PMM");
    let mut addrs: [usize; 15] = [0; 15];

    for _ in 0..2 {
        for (i, addr_entry) in addrs.iter_mut().enumerate() {
            if let Some(addr) = pmm.allocate(i + 1).map(|x| x as usize) {
                *addr_entry = addr;
                debug!("allocated 0x{addr:08X} (x{})", i + 1);
            }
        }

        for (i, &addr) in addrs.iter().enumerate() {
            pmm.free(addr, i + 1);
            debug!("freed 0x{addr:08X} (x{})", i + 1);
        }
    }
}
