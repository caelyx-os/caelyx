use core::{ alloc::Layout, hint::spin_loop, sync::atomic::AtomicBool };

use uacpi::{
    uacpi_bool,
    uacpi_char,
    uacpi_cpu_flags,
    uacpi_firmware_request,
    uacpi_handle,
    uacpi_initialize,
    uacpi_interrupt_handler,
    uacpi_io_addr,
    uacpi_log_level,
    uacpi_log_level_UACPI_LOG_DEBUG,
    uacpi_log_level_UACPI_LOG_ERROR,
    uacpi_log_level_UACPI_LOG_INFO,
    uacpi_log_level_UACPI_LOG_TRACE,
    uacpi_log_level_UACPI_LOG_WARN,
    uacpi_namespace_initialize,
    uacpi_namespace_load,
    uacpi_pci_address,
    uacpi_phys_addr,
    uacpi_size,
    uacpi_status,
    uacpi_status_UACPI_STATUS_OK,
    uacpi_status_UACPI_STATUS_UNIMPLEMENTED,
    uacpi_table,
    uacpi_table_find_by_signature,
    uacpi_thread_id,
    uacpi_u8,
    uacpi_u16,
    uacpi_u32,
    uacpi_u64,
    uacpi_work_handler,
    uacpi_work_type,
};

use crate::{
    boot::multiboot2,
    debug,
    error,
    info,
    misc::{ isituninit::IsItUninit, ptr_align::{ align_ptr_down, align_ptr_up } },
    mm::{ virt_page_alloc, vmm },
    sync::mutex::Mutex,
    trace,
    warning,
    x86::ioport::{ inb, inl, inw, outb, outl, outw },
};

static RSDP: Mutex<IsItUninit<usize>> = Mutex::new(IsItUninit::uninit());
static HANDLE: Mutex<u32> = Mutex::new(69);

fn find_rsdp(tag_iter: &mut multiboot2::TagIterator) -> *const () {
    let rsdp_mb2 = tag_iter.find(|x| {
        matches!(x, multiboot2::MultibootTag::AcpiOld(_)) ||
            matches!(x, multiboot2::MultibootTag::AcpiNew(_))
    });

    if let Some(tag) = rsdp_mb2 {
        if let multiboot2::MultibootTag::AcpiNew(rsdp_new) = tag {
            debug!("Found RSDP at 0x{:08X}", rsdp_new as usize);
            return rsdp_new;
        } else if let multiboot2::MultibootTag::AcpiOld(rsdp_old) = tag {
            debug!("Found RSDP at 0x{:08X}", rsdp_old as usize);
            return rsdp_old;
        }
    }

    let mut addr = 0x000e0000u32;
    while addr < 0x000fffff {
        if (unsafe { core::slice::from_raw_parts::<u8>(addr as *const u8, 8) }) == b"RSD PTR " {
            debug!("Found RSDP at 0x{addr:08X}");
            return addr as *const ();
        }
        addr += 16;
    }

    panic!("No ACPI RSDP found (required for Caelyx to function properly)");
}

pub fn init(tag_iter: &mut multiboot2::TagIterator) {
    RSDP.lock().write(find_rsdp(tag_iter) as usize);

    unsafe {
        if
            uacpi_initialize(0) != uacpi_status_UACPI_STATUS_OK ||
            uacpi_namespace_load() != uacpi_status_UACPI_STATUS_OK ||
            uacpi_namespace_initialize() != uacpi_status_UACPI_STATUS_OK
        {
            panic!("uACPI initialization failed");
        }
    }

    let mut madt_table: uacpi_table = uacpi_table::default();
    unsafe {
        uacpi_table_find_by_signature(b"APIC".as_ptr() as *const i8, &mut madt_table);
    }

    let madt_virtual_address: usize;
    let madt_physical_address: usize;

    unsafe {
        madt_virtual_address = madt_table.__bindgen_anon_1.virt_addr;
        madt_physical_address = madt_table.__bindgen_anon_1.ptr as usize;
    }

    debug!(
        "MADT virtual address: {madt_virtual_address:#08X}, MADT physical address: {madt_physical_address:#08X}"
    );

    info!("Initialized ACPI");
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_get_rsdp(_out_rsdp_address: *mut uacpi_phys_addr) -> uacpi_status {
    unsafe {
        *_out_rsdp_address = *RSDP.lock().get_ref() as u64;
    }

    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_map(
    _addr: uacpi_phys_addr,
    _len: uacpi_size
) -> *mut ::core::ffi::c_void {
    let first_phys_page = align_ptr_down(_addr as *const u8, 4096) as u32;
    let last_phys_page = align_ptr_up(
        (first_phys_page + (align_ptr_up(_len as *const u8, 4096) as u32)) as *const u8,
        4096
    ) as u32;

    let page_count = (last_phys_page - first_phys_page) / 4096;
    let first_virt_page = virt_page_alloc
        ::allocate(page_count as usize)
        .expect("Could not allocate virtual pages for uACPI");

    for i in 0..page_count {
        vmm::map(first_phys_page + i * 4096, first_virt_page + i * 4096, false, true, false, false);
    }

    (first_virt_page + ((_addr as u32) - first_phys_page)) as *mut core::ffi::c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_unmap(_addr: *mut ::core::ffi::c_void, _len: uacpi_size) {
    let last_virt_page = align_ptr_up(
        ((_addr as u32) + (align_ptr_up(_len as *const u8, 4096) as u32)) as *const u8,
        4096
    ) as u32;

    let page_count = (last_virt_page - (_addr as u32)) / 4096;

    for i in 0..page_count {
        vmm::unmap((align_ptr_down(_addr as *const u8, 4096) as u32) + i * 4096);
    }

    virt_page_alloc::free(align_ptr_down(_addr as *const u8, 4096), page_count as usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_log(_arg1: uacpi_log_level, _arg2: *const uacpi_char) {
    let mut len = 0;
    let ptr = _arg2 as *const u8;
    while (unsafe { *ptr.add(len) }) != 0 {
        len += 1;
    }

    let _str = (unsafe { core::str::from_raw_parts(ptr, len) }).strip_suffix("\n").unwrap_or("");
    match _arg1 {
        #[allow(non_upper_case_globals)]
        uacpi_log_level_UACPI_LOG_DEBUG => debug!("uACPI: {_str}"),
        #[allow(non_upper_case_globals)]
        uacpi_log_level_UACPI_LOG_ERROR => error!("uACPI: {_str}"),
        #[allow(non_upper_case_globals)]
        uacpi_log_level_UACPI_LOG_TRACE => trace!("uACPI: {_str}"),
        #[allow(non_upper_case_globals)]
        uacpi_log_level_UACPI_LOG_WARN => warning!("uACPI: {_str}"),
        #[allow(non_upper_case_globals)]
        uacpi_log_level_UACPI_LOG_INFO => debug!("uACPI: {_str}"), // INFO: heres all the addresses that every person that isnt debugging needs to see
        _ => unreachable!(),
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_pci_device_open(
    _address: uacpi_pci_address,
    _out_handle: *mut uacpi_handle
) -> uacpi_status {
    let mut handle = 0;
    handle |= _address.bus as u64;
    handle |= (_address.device as u64) >> 8;
    handle |= (_address.segment as u64) >> 16;
    handle |= (_address.function as u64) >> 32;

    unsafe {
        *_out_handle = &raw const handle as *mut core::ffi::c_void;
    }

    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_device_close(_arg1: uacpi_handle) {}

const CONFIG_ADDRESS: u16 = 0xcf8;
const CONFIG_DATA: u16 = 0xcfc;

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_pci_read8(
    _device: uacpi_handle,
    _offset: uacpi_size,
    _value: *mut uacpi_u8
) -> uacpi_status {
    let address: u64 = unsafe { *(_device as *const u64) };
    let bus = (address & 0xff) as u8;
    let device = ((address >> 8) & 0xff) as u8;
    let function = ((address >> 32) & 0xff) as u8;

    let address: u32 =
        ((bus as u32) << 16) |
        ((device as u32) << 11) |
        ((function as u32) << 8) |
        ((_offset as u32) & 0xfc) |
        0x80000000;

    outl(CONFIG_ADDRESS, address);
    unsafe {
        *_value = ((inl(CONFIG_DATA) >> ((_offset & 3) * 8)) & 0xff) as u8;
    }
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_pci_read16(
    _device: uacpi_handle,
    _offset: uacpi_size,
    _value: *mut uacpi_u16
) -> uacpi_status {
    let address: u64 = unsafe { *(_device as *const u64) };
    let bus = (address & 0xff) as u8;
    let device = ((address >> 8) & 0xff) as u8;
    let function = ((address >> 32) & 0xff) as u8;

    let address: u32 =
        ((bus as u32) << 16) |
        ((device as u32) << 11) |
        ((function as u32) << 8) |
        ((_offset as u32) & 0xfc) |
        0x80000000;

    outl(CONFIG_ADDRESS, address);
    unsafe {
        *_value = ((inl(CONFIG_DATA) >> ((_offset & 2) * 8)) & 0xffff) as u16;
    }
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_pci_read32(
    _device: uacpi_handle,
    _offset: uacpi_size,
    _value: *mut uacpi_u32
) -> uacpi_status {
    let address: u64 = unsafe { *(_device as *const u64) };
    let bus = (address & 0xff) as u8;
    let device = ((address >> 8) & 0xff) as u8;
    let function = ((address >> 32) & 0xff) as u8;

    let address: u32 =
        ((bus as u32) << 16) |
        ((device as u32) << 11) |
        ((function as u32) << 8) |
        ((_offset as u32) & 0xfc) |
        0x80000000;

    outl(CONFIG_ADDRESS, address);
    unsafe {
        *_value = inl(CONFIG_DATA);
    }
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_write8(
    _device: uacpi_handle,
    _offset: uacpi_size,
    _value: uacpi_u8
) -> uacpi_status {
    uacpi_status_UACPI_STATUS_UNIMPLEMENTED
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_write16(
    _device: uacpi_handle,
    _offset: uacpi_size,
    _value: uacpi_u16
) -> uacpi_status {
    uacpi_status_UACPI_STATUS_UNIMPLEMENTED
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_write32(
    _device: uacpi_handle,
    _offset: uacpi_size,
    _value: uacpi_u32
) -> uacpi_status {
    uacpi_status_UACPI_STATUS_UNIMPLEMENTED
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_io_map(
    _base: uacpi_io_addr,
    _len: uacpi_size,
    _out_handle: *mut uacpi_handle
) -> uacpi_status {
    unsafe {
        *_out_handle = _base as u32 as *mut core::ffi::c_void;
    }

    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_unmap(_handle: uacpi_handle) {}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_io_read8(
    _arg1: uacpi_handle,
    _offset: uacpi_size,
    _out_value: *mut uacpi_u8
) -> uacpi_status {
    unsafe {
        *_out_value = inb(_arg1 as u16);
    }
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_io_read16(
    _arg1: uacpi_handle,
    _offset: uacpi_size,
    _out_value: *mut uacpi_u16
) -> uacpi_status {
    unsafe {
        *_out_value = inw(_arg1 as u16);
    }
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_io_read32(
    _arg1: uacpi_handle,
    _offset: uacpi_size,
    _out_value: *mut uacpi_u32
) -> uacpi_status {
    unsafe {
        *_out_value = inl(_arg1 as u16);
    }
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_write8(
    _arg1: uacpi_handle,
    _offset: uacpi_size,
    _in_value: uacpi_u8
) -> uacpi_status {
    outb(_arg1 as u16, _in_value);
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_write16(
    _arg1: uacpi_handle,
    _offset: uacpi_size,
    _in_value: uacpi_u16
) -> uacpi_status {
    outw(_arg1 as u16, _in_value);
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_write32(
    _arg1: uacpi_handle,
    _offset: uacpi_size,
    _in_value: uacpi_u32
) -> uacpi_status {
    outl(_arg1 as u16, _in_value);
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_alloc(_size: uacpi_size) -> *mut ::core::ffi::c_void {
    unsafe {
        alloc::alloc::alloc(Layout::from_size_align(_size, 16).unwrap()) as *mut ::core::ffi::c_void
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free(_mem: *mut ::core::ffi::c_void) {
    // TODO: save layouts used somewhere and call alloc::alloc::dealloc. Not required for now as we
    // have a bump alloator but yeah
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_get_nanoseconds_since_boot() -> uacpi_u64 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_stall(_usec: uacpi_u8) {
    panic!("unimplemented uacpi_kernel_stall");
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_sleep(_msec: uacpi_u64) {
    panic!("unimplemented uacpi_kernel_sleep");
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_create_mutex() -> uacpi_handle {
    let mut lock = HANDLE.lock();
    *lock += 1;
    (*lock - 1) as *mut ::core::ffi::c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free_mutex(_arg1: uacpi_handle) {}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_create_event() -> uacpi_handle {
    let mut lock = HANDLE.lock();
    *lock += 1;
    (*lock - 1) as *mut ::core::ffi::c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free_event(_arg1: uacpi_handle) {}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_get_thread_id() -> uacpi_thread_id {
    69 as *mut ::core::ffi::c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_acquire_mutex(
    _arg1: uacpi_handle,
    _arg2: uacpi_u16
) -> uacpi_status {
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_release_mutex(_arg1: uacpi_handle) {}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_wait_for_event(_arg1: uacpi_handle, _arg2: uacpi_u16) -> uacpi_bool {
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_signal_event(_arg1: uacpi_handle) {}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_reset_event(_arg1: uacpi_handle) {}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_handle_firmware_request(
    _arg1: *mut uacpi_firmware_request
) -> uacpi_status {
    uacpi_status_UACPI_STATUS_UNIMPLEMENTED
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn uacpi_kernel_install_interrupt_handler(
    _irq: uacpi_u32,
    _arg1: uacpi_interrupt_handler,
    _ctx: uacpi_handle,
    _out_irq_handle: *mut uacpi_handle
) -> uacpi_status {
    unsafe {
        *_out_irq_handle = 69 as *mut core::ffi::c_void;
    }

    // FIXME: Actually handle this
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_uninstall_interrupt_handler(
    _arg1: uacpi_interrupt_handler,
    _irq_handle: uacpi_handle
) -> uacpi_status {
    uacpi_status_UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_create_spinlock() -> uacpi_handle {
    unsafe {
        alloc::alloc::alloc_zeroed(
            Layout::from_size_align(
                core::mem::size_of::<AtomicBool>(),
                core::mem::align_of::<AtomicBool>()
            ).unwrap()
        ) as *mut ::core::ffi::c_void
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free_spinlock(_arg1: uacpi_handle) {}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_lock_spinlock(_arg1: uacpi_handle) -> uacpi_cpu_flags {
    let flag = unsafe { &*(_arg1 as *mut AtomicBool) };

    while
        flag
            .compare_exchange_weak(
                false,
                true,
                core::sync::atomic::Ordering::AcqRel,
                core::sync::atomic::Ordering::Acquire
            )
            .is_err()
    {
        spin_loop();
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_unlock_spinlock(_arg1: uacpi_handle, _arg2: uacpi_cpu_flags) {
    let flag = unsafe { &*(_arg1 as *mut AtomicBool) };
    flag.store(false, core::sync::atomic::Ordering::Release);
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_schedule_work(
    _arg1: uacpi_work_type,
    _arg2: uacpi_work_handler,
    _ctx: uacpi_handle
) -> uacpi_status {
    panic!("unimplemented uacpi_kernel_schedule_work");
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_wait_for_work_completion() -> uacpi_status {
    panic!("unimplemented uacpi_kernel_wait_for_work_completion");
}
