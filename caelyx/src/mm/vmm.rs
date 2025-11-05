use crate::{
    debug,
    misc::output::flanterm::paging_fix,
    mm::pmm,
    trace,
    x86::cpuid::feature_present,
};
use core::{ mem::MaybeUninit, sync::atomic::{ AtomicU32, Ordering } };

fn switch_cr3(cr3: u32) {
    // cr3 stores the pointer to the highest level paging structure
    unsafe {
        core::arch::asm!("mov cr3, eax", in("eax") cr3)
    }
}

fn enable_pse() {
    // set PSE bit (page size extension enable) in cr4
    let mut cr4: u32;

    unsafe {
        core::arch::asm!("mov eax, cr4", out("eax") cr4);
    }

    cr4 |= 1 << 4;

    unsafe {
        core::arch::asm!("mov cr4, eax", in("eax") cr4);
    }
}

fn enable_pg() {
    // set PG bit (paging enable) in cr0
    let mut cr0: u32;

    unsafe {
        core::arch::asm!("mov eax, cr0", out("eax") cr0);
    }

    cr0 |= 1 << 31;

    unsafe {
        core::arch::asm!("mov cr0, eax", in("eax") cr0);
    }
}

fn flush_tlb(virt_addr: u32) {
    // flush tlb cache for a virtual address
    unsafe {
        core::arch::asm!("invlpg [{virt_addr}]", virt_addr = in(reg) virt_addr)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PageDirectoryEntry {
    pub addr: u32,
    pub present: bool,
    pub writable: bool,
    pub user: bool,
    pub write_through: bool,
    pub cache_disable: bool,
    pub accessed: bool,
    pub page_size: bool,
    pub global: bool,
    pub page_attribute_table: bool,
    pub dirty: bool,
}

impl PageDirectoryEntry {
    pub fn to_u32(&self) -> u32 {
        assert!(
            self.addr.is_multiple_of(if !self.page_size { (2u32).pow(12) } else { (2u32).pow(22) })
        );

        let mut end_u32: u32 = 0;

        end_u32 |= self.present as u32;
        end_u32 |= (self.writable as u32) << 1;
        end_u32 |= (self.user as u32) << 2;
        end_u32 |= (self.write_through as u32) << 3;
        end_u32 |= (self.cache_disable as u32) << 4;
        end_u32 |= (self.accessed as u32) << 5;
        if self.page_size {
            end_u32 |= (self.dirty as u32) << 6;
        }

        end_u32 |= (self.page_size as u32) << 7;
        if self.page_size {
            end_u32 |= (self.global as u32) << 8;
            end_u32 |= (self.page_attribute_table as u32) << 12;
            end_u32 |= (self.addr >> 22) << 22;
        } else {
            end_u32 |= (self.addr >> 12) << 12;
        }

        end_u32
    }

    pub fn from_u32(from: u32) -> Self {
        let dirty: bool;
        let global: bool;
        let page_attribute_table: bool;
        let addr: u32;

        let present: bool = (from & (1 << 0)) != 0;
        let writable: bool = (from & (1 << 1)) != 0;
        let user: bool = (from & (1 << 2)) != 0;
        let write_through: bool = (from & (1 << 3)) != 0;
        let cache_disable: bool = (from & (1 << 4)) != 0;
        let accessed: bool = (from & (1 << 5)) != 0;
        let page_size: bool = (from & (1 << 7)) != 0;

        if page_size {
            dirty = (from & (1 << 6)) != 0;
            global = (from & (1 << 8)) != 0;
            page_attribute_table = (from & (1 << 12)) != 0;
            addr = (from >> 22) << 22;
        } else {
            global = false;
            page_attribute_table = false;
            dirty = false;
            addr = (from >> 12) << 12;
        }

        PageDirectoryEntry {
            addr,
            present,
            writable,
            user,
            write_through,
            cache_disable,
            accessed,
            page_size,
            global,
            page_attribute_table,
            dirty,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PageTableEntry {
    pub addr: u32,
    pub global: bool,
    pub page_attribute_table: bool,
    pub dirty: bool,
    pub present: bool,
    pub writable: bool,
    pub user: bool,
    pub write_through: bool,
    pub cache_disable: bool,
    pub accessed: bool,
}

impl PageTableEntry {
    pub fn to_u32(&self) -> u32 {
        assert!(self.addr.is_multiple_of((2u32).pow(12)));

        let mut end_u32: u32 = 0;

        end_u32 |= self.present as u32;
        end_u32 |= (self.writable as u32) << 1;
        end_u32 |= (self.user as u32) << 2;
        end_u32 |= (self.write_through as u32) << 3;
        end_u32 |= (self.cache_disable as u32) << 4;
        end_u32 |= (self.accessed as u32) << 5;
        end_u32 |= (self.dirty as u32) << 6;
        end_u32 |= (self.global as u32) << 8;
        end_u32 |= (self.page_attribute_table as u32) << 12;
        end_u32 |= (self.addr >> 12) << 12;

        end_u32
    }

    pub fn from_u32(from: u32) -> Self {
        let present: bool = (from & (1 << 0)) != 0;
        let writable: bool = (from & (1 << 1)) != 0;
        let user: bool = (from & (1 << 2)) != 0;
        let write_through: bool = (from & (1 << 3)) != 0;
        let cache_disable: bool = (from & (1 << 4)) != 0;
        let accessed: bool = (from & (1 << 5)) != 0;
        let dirty: bool = (from & (1 << 6)) != 0;
        let page_attribute_table: bool = (from & (1 << 7)) != 0;
        let global: bool = (from & (1 << 8)) != 0;
        let addr: u32 = (from >> 12) << 12;

        PageTableEntry {
            addr,
            present,
            writable,
            user,
            write_through,
            cache_disable,
            accessed,
            global,
            page_attribute_table,
            dirty,
        }
    }
}

#[repr(C, align(4096))]
struct PageDirectory([AtomicU32; 1024]);

impl PageDirectory {
    pub const fn create() -> Self {
        Self(unsafe { MaybeUninit::zeroed().assume_init() })
    }

    pub const fn as_ptr<T>(&self) -> *const T {
        self.0.as_ptr() as *const T
    }

    #[allow(unused)]
    pub const fn as_mut_ptr<T>(&mut self) -> *mut T {
        self.0.as_mut_ptr() as *mut T
    }

    pub fn set(&self, idx: usize, value: PageDirectoryEntry) {
        self.0[idx].store(value.to_u32(), Ordering::Release);
    }

    pub fn get(&self, idx: usize) -> PageDirectoryEntry {
        PageDirectoryEntry::from_u32(self.0[idx].load(Ordering::Acquire))
    }
}

#[repr(C, align(4096))]
struct PageTable([AtomicU32; 1024]);

impl PageTable {
    #[allow(unused)]
    pub const fn create() -> Self {
        Self(unsafe { MaybeUninit::zeroed().assume_init() })
    }

    #[allow(unused)]
    pub const fn as_ptr<T>(&self) -> *const T {
        self.0.as_ptr() as *const T
    }

    #[allow(unused)]
    pub const fn as_mut_ptr<T>(&mut self) -> *mut T {
        self.0.as_mut_ptr() as *mut T
    }

    pub fn set(&self, idx: usize, value: PageTableEntry) {
        self.0[idx].store(value.to_u32(), Ordering::Release);
    }

    pub fn get(&self, idx: usize) -> PageTableEntry {
        PageTableEntry::from_u32(self.0[idx].load(Ordering::Acquire))
    }
}

static PAGE_DIRECTORY: PageDirectory = PageDirectory::create();

pub fn map(
    phys_addr: u32,
    virt_addr: u32,
    user: bool,
    writable: bool,
    cache_disable: bool,
    write_through: bool
) {
    let pde: usize = ((virt_addr >> 22) & 0x3ff) as usize;
    let pte: usize = ((virt_addr >> 12) & 0x3ff) as usize;

    if !PAGE_DIRECTORY.get(pde).present {
        let pt = pmm::allocate(1).expect("Could not allocate PT");

        unsafe {
            core::ptr::write_bytes(pt, 0u8, 4096);
        }

        let pt = pt as *const PageTable;
        PAGE_DIRECTORY.set(pde, PageDirectoryEntry {
            addr: pt as u32,
            cache_disable,
            write_through,
            page_size: false,
            writable,
            user,
            present: true,
            accessed: false,
            dirty: false,
            global: false,
            page_attribute_table: false,
        });
    }

    let pt = PAGE_DIRECTORY.get(pde).addr as *const PageTable;
    if (unsafe { (*pt).get(pte) }).present {
        panic!("Double map (PTE level) 0x{virt_addr:08X}");
    }

    unsafe {
        (*pt).set(pte, PageTableEntry {
            addr: phys_addr,
            cache_disable,
            write_through,
            writable,
            user,
            present: true,
            accessed: false,
            dirty: false,
            global: false,
            page_attribute_table: false,
        });
    }

    flush_tlb(virt_addr);
}

pub fn map4mb(
    phys_addr: u32,
    virt_addr: u32,
    user: bool,
    writable: bool,
    cache_disable: bool,
    write_through: bool
) {
    let pde: usize = ((virt_addr >> 22) & 0x3ff) as usize;

    if PAGE_DIRECTORY.get(pde).present {
        panic!("Double map (PDE level) 0x{virt_addr:08X}");
    }

    PAGE_DIRECTORY.set(pde, PageDirectoryEntry {
        addr: phys_addr,
        cache_disable,
        write_through,
        page_size: true,
        writable,
        user,
        present: true,
        accessed: false,
        dirty: false,
        global: false,
        page_attribute_table: false,
    });

    flush_tlb(virt_addr);
}

pub fn unmap(virt_addr: u32) {
    let pde: usize = ((virt_addr >> 22) & 0x3ff) as usize;
    let pte: usize = ((virt_addr >> 12) & 0x3ff) as usize;

    let pde_entry = PAGE_DIRECTORY.get(pde);
    if !pde_entry.present {
        panic!("Double free (PDE level) 0x{virt_addr:08X}");
    }

    if pde_entry.page_size {
        PAGE_DIRECTORY.set(pde, PageDirectoryEntry::default());
        debug!("Unmapped 4MB page at 0x{virt_addr:08X}");
        return;
    }

    let pt = pde_entry.addr as *const PageTable;
    unsafe {
        (*pt).set(pte, PageTableEntry::default());
    }

    let mut present = false;
    for i in 0..1024 {
        if (unsafe { (*pt).get(i) }).present {
            present = true;
            break;
        }
    }

    if !present {
        pmm::free(pde_entry.addr as *const u8, 1);
        PAGE_DIRECTORY.set(pde, PageDirectoryEntry::default());
    }

    flush_tlb(virt_addr);
}

// My virtual address space layout is beyond horrendously fucked:
// 0x00000000 - 0x003FFFFF : Kernel
// 0x00400000 - 0xBFFFFFFF : User (future)
// 0xC0000000 - 0xFFFFFFFF : Kernel
pub fn init() {
    assert!(feature_present(&crate::x86::cpuid::Features::Pse));

    enable_pse();
    trace!("Enabled PSE");

    map4mb(0, 0, false, true, false, false);
    trace!("Identity-mapped first 4MB");

    switch_cr3(PAGE_DIRECTORY.as_ptr::<()>() as u32);
    trace!("Loaded CR3");

    paging_fix();
    enable_pg();
    trace!("Turned on PG");

    debug!("Initialized VMM");
}
