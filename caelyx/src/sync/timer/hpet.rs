use core::{ time::Duration, arch::asm };

use crate::{
    debug,
    misc::{ acpi::{ AcpiGAS, get_hpet_table }, isituninit::IsItUninit },
    mm::{ virt_page_alloc, vmm },
    sync::mutex::Mutex,
    trace,
};
use uacpi::uacpi_table;

#[derive(Debug)]
pub enum HpetTimerError {
    HpetAcpiSdtNotFound,
}

#[repr(C, packed)]
pub struct HpetSdt {
    pub event_timer_block_id: u32,
    pub base_address: AcpiGAS,
    pub id: u8,
    pub min_clk_tick: u16,
    pub page_protect: u8,
}

pub struct HpetTimer {
    address: usize,
    counter_period: u64,
}

impl HpetTimer {
    pub fn new() -> Result<Self, HpetTimerError> {
        let mut hpet_table: uacpi_table = uacpi_table::default();
        if !get_hpet_table(&mut hpet_table) {
            return Err(HpetTimerError::HpetAcpiSdtNotFound);
        }

        let hpet_table_data = (unsafe {
            (hpet_table.__bindgen_anon_1.ptr as *const u8).add(0x24)
        }) as *mut HpetSdt;

        assert_eq!(
            unsafe {
                (*hpet_table_data).base_address.address_space
            },
            0,
            "HPET is not based in the memory address space!"
        );

        let hpet_address: u64;

        unsafe {
            hpet_address = (*hpet_table_data).base_address.address;
        }

        assert!(hpet_address <= (u32::MAX as u64), "HPET address > 4GB");

        let counter_period: u64;

        let virt_pages = virt_page_alloc
            ::allocate(1)
            .expect("Could not allocate virtual page to map HPET MMIO region");

        vmm::map(hpet_address as u32, virt_pages, false, false, true, true);
        trace!("Mapped HPET MMIO region ({hpet_address:#08X} -> {virt_pages:#08X})");

        let hpet_address = virt_pages;

        unsafe {
            counter_period = core::ptr::read_volatile(hpet_address as *const u64) >> 32;
        }

        trace!("HPET speed: {counter_period} femtoseconds/tick");

        /*
        General Configuration Register
        0>ENABLE_CNF	Overall enable.
        0 - main counter is halted, timer interrupts are disabled

        1 - main counter is running, timer interrupts are allowed if enabled
        */

        unsafe {
            core::ptr::write_volatile((hpet_address as *mut u8).add(0x10) as *mut u64, 0); // disable counter
            core::ptr::write_volatile((hpet_address as *mut u8).add(0xf0) as *mut u64, 0); // clear counter
            core::ptr::write_volatile((hpet_address as *mut u8).add(0x10) as *mut u64, 1); // enable counter
        }

        trace!("Cleared HPET counter");

        debug!("Initialized HPET at {hpet_address:#08X}");
        Ok(HpetTimer { counter_period, address: hpet_address as usize })
    }

    pub fn sleep(&self, dur: Duration) {
        let mic = dur.as_micros();
        let pass = (mic * 1_000_000_000) / (self.counter_period as u128); // period is in femtoseconds/tick
        let addr = unsafe { (self.address as *const u8).add(0xf0) as *const u64 };
        let start = unsafe { core::ptr::read_volatile(addr) };

        while ((unsafe { core::ptr::read_volatile(addr) }) as u128) < pass + (start as u128) {
            unsafe {
                asm!("pause");
            }
        }
    }

    pub fn get_us_passed(&self) -> u64 {
        let ticks = unsafe { core::ptr::read_volatile(self.address as *const u64) };
        let usx1 = 1_000_000_000 * self.counter_period;
        let us_passed = ticks / usx1;
        us_passed
    }
}

static HPET: Mutex<IsItUninit<HpetTimer>> = Mutex::new(IsItUninit::uninit());

pub fn init() {
    HPET.lock().write(HpetTimer::new().expect("Could not initialize HPET!"));
}

pub fn hpet_sleep(dur: Duration) {
    HPET.lock().get_ref().sleep(dur);
}

pub fn hpet_get_us_passed() -> u64 {
    HPET.lock().get_ref().get_us_passed()
}
