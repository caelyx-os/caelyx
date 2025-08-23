use crate::{debug, sync::mutex::Mutex, trace, x86::halt};

// This is just a minimal GDT i sticked together 50 decades ago
static GDT: [u64; 3] = [0x0000000000000000, 0x00CF9A000000FFFF, 0x00CF92000000FFFF];
#[allow(clippy::erasing_op)]
// The byte index of the null descriptor
pub const GDT_NULL: u16 = 0 * core::mem::size_of::<u64>() as u16;
#[allow(clippy::identity_op)]
// The byte index of the code descriptor
pub const GDT_CODE: u16 = 1 * core::mem::size_of::<u64>() as u16;
// The byte index of the data descriptor
pub const GDT_DATA: u16 = 2 * core::mem::size_of::<u64>() as u16;

#[repr(C, packed)]
#[derive(Clone)]
// This structure is the same for the IDTR and GDTR so who cares
pub struct SharedGdtrAndIdtr {
    pub limit: u16,
    pub base: u32,
}

static GDTR: Mutex<SharedGdtrAndIdtr> = Mutex::new(SharedGdtrAndIdtr { limit: 0, base: 0 });

pub fn init() {
    let gdt_ptr;
    {
        let mut lock = GDTR.lock();
        lock.base = &raw const GDT as u32;
        lock.limit = (core::mem::size_of_val(&GDT) - 1) as u16;
        gdt_ptr = &raw const *lock;
    }

    trace!("Initialized GDTR");

    unsafe {
        // We first load the gdt using the lgdt instruction and after that we need to execute
        // something called a far jump since we cannot directly change cs. Then we need to change
        // all data segments which we cant change directly too so we use ax for that
        core::arch::asm!("lgdt [{gdt_reg:e}]",
                         "push {cs}",
                         "lea eax, [2f]",
                         "push eax",
                         "retf",
                         "2:",
                         "mov ax, {ds_reg:x}",
                         "mov ds, ax",
                         "mov es, ax",
                         "mov fs, ax",
                         "mov gs, ax",
                         "mov ss, ax",
                         gdt_reg = in(reg) gdt_ptr,
                         cs = const GDT_CODE,
                         ds_reg = in(reg) GDT_DATA,
                         out("eax") _);
    }

    trace!("Loaded GDTR & Reloaded cs,ds,es,fs,gs,ss");
    debug!("Initialized GDT");
}
