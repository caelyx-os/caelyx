use core::{
    alloc::GlobalAlloc,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    debug,
    misc::{isituninit::IsItUninit, ptr_align::align_ptr_up},
    mm::{pmm, virt_page_alloc, vmm},
    sync::mutex::Mutex,
};
pub struct Heap {
    mem_block: (usize, usize),
    curr_offset: AtomicUsize, // okay why not make people lifes miserable and make
                              // GlobalAlloc::alloc take in a &self
}

impl Heap {
    pub fn new() -> Self {
        let phys_pages = pmm::allocate(16).expect("Could not allocate initial heap physical pages"); // initial heap size = 16 pages
        let virt_pages =
            virt_page_alloc::allocate(16).expect("Could not allocate initial heap virtual pages");
        vmm::map(phys_pages as u32, virt_pages, false, true, false, false);

        Self {
            mem_block: (virt_pages as usize, 16 * 4096),
            curr_offset: AtomicUsize::new(0),
        }
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        let curr_offset = self.curr_offset.load(Ordering::Acquire);
        let allocation_ptr =
            align_ptr_up((self.mem_block.0 + curr_offset) as *const u8, align) as usize;

        self.curr_offset
            .store(allocation_ptr - self.mem_block.0 + size, Ordering::Release);

        debug!("Allocated {size} bytes with alignment {align} at 0x{allocation_ptr:08X}");

        allocation_ptr as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

struct HeapWrapper(Mutex<IsItUninit<Heap>>);

unsafe impl GlobalAlloc for HeapWrapper {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let lock = self.0.lock();
        assert!(lock.initialized());
        unsafe { lock.get_ref().alloc(layout) }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[global_allocator]
static HEAP: HeapWrapper = HeapWrapper(Mutex::new(IsItUninit::uninit()));

pub fn init() {
    let mut lock = HEAP.0.lock();
    let heap = Heap::new();
    lock.write(heap);
    debug!("Initialized heap");
}
