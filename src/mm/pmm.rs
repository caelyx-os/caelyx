use core::{mem::MaybeUninit, sync::atomic::AtomicU8};

use crate::{
    boot::multiboot2::{
        _MultibootMmapPart, MultibootMmapEntryType, MultibootTag,
        TagIterator as MultibootTagIterator,
    },
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

pub struct PhysicalMemoryAllocator {}

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
        for (start, size) in free_region_iterator {
            let end = start + size;

            let mut first = None;
            let mut second = None;

            if !(kernel_start >= start && kernel_end <= end) {
                first = Some((start, end));
            } else {
                if kernel_start > start {
                    first = Some((start, kernel_start));
                }

                if kernel_end < end {
                    second = Some((kernel_end, end));
                }
            }

            if let Some(first) = first {
                let (start, end) = first;
                process_chunk(start, end - start);
            }

            if let Some(second) = second {
                let (start, end) = second;
                process_chunk(start, end - start);
            }

            fn process_chunk(start: usize, size: usize) -> Option<(usize, usize)> {
                let first_page = align_ptr_up(start as *const u8, 4096) as usize;
                if first_page > start + size {
                    return None;
                }

                let new_size = size - (first_page - start);

                let last_page = align_ptr_down((first_page + new_size) as *const u8, 4096) as usize;

                if first_page == last_page {
                    return None;
                }

                trace!(
                    "0x{:08X}-0x{:08X} ({} pages, {} MB)",
                    first_page,
                    last_page,
                    (last_page - first_page) as f64 / 4096.0,
                    (last_page - first_page) as f64 / 1024.0 / 1024.0
                );

                Some((first_page, last_page))
            }
        }

        Ok(Self {})
    }
}

pub fn init(tag_iter: &mut MultibootTagIterator) {
    let _pmm = PhysicalMemoryAllocator::new(*tag_iter).expect("Could not create PMM");
}
