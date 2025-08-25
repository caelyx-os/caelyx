use crate::{debug, misc::isituninit::IsItUninit, sync::mutex::Mutex};

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Features {
    Fpu = 1 << 0,
    Vme = 1 << 1,
    De = 1 << 2,
    Pse = 1 << 3,
    Tsc = 1 << 4,
    Msr = 1 << 5,
    Pae = 1 << 6,
    Mce = 1 << 7,
    Cx8 = 1 << 8,
    Apic = 1 << 9,
    Sep = 1 << 11,
    Mtrr = 1 << 12,
    Pge = 1 << 13,
    Mca = 1 << 14,
    Cmov = 1 << 15,
    Pat = 1 << 16,
    Pse36 = 1 << 17,
    Psn = 1 << 18,
    Clflush = 1 << 19,
    Ds = 1 << 21,
    Acpi = 1 << 22,
    Mmx = 1 << 23,
    Fxsr = 1 << 24,
    Sse = 1 << 25,
    Sse2 = 1 << 26,
    Ss = 1 << 27,
    Htt = 1 << 28,
    Tm = 1 << 29,
    Ia64 = 1 << 30,
    Pbe = 1 << 31,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vendor {
    Amd,
    Intel,
    Via,
    Transmeta,
    Cyrix,
    Centaur,
    Nexgen,
    Umc,
    Sis,
    Nsc,
    Rise,
    Vortex,
    AO486,
    Zhaoxin,
    Hygon,
    Elbrus,
    Qemu,
    Kvm,
    Vmware,
    VirtualBox,
    Xen,
    HyperV,
    Parallels,
    Bhyve,
    Qnx,
    Unknown,
}

impl Vendor {
    pub fn from_vendor_bytes(vendor: &[u8; 12]) -> Self {
        match vendor {
            b"AuthenticAMD" => Self::Amd,
            b"AMDisbetter!" => Self::Amd,
            b"GenuineIntel" => Self::Intel,
            b"VIA VIA VIA " => Self::Via,
            b"GenuineTMx86" => Self::Transmeta,
            b"TransmetaCPU" => Self::Transmeta,
            b"CyrixInstead" => Self::Cyrix,
            b"CentaurHauls" => Self::Centaur,
            b"NexGenDriven" => Self::Nexgen,
            b"UMC UMC UMC " => Self::Umc,
            b"SiS SiS SiS " => Self::Sis,
            b"Geode by NSC" => Self::Nsc,
            b"RiseRiseRise" => Self::Rise,
            b"Vortex86 SoC" => Self::Vortex,
            b"MiSTer AO486" => Self::AO486,
            b"GenuineAO486" => Self::AO486,
            b"  Shanghai  " => Self::Zhaoxin,
            b"HygonGenuine" => Self::Hygon,
            b"E2K MACHINE " => Self::Elbrus,
            b"TCGTCGTCGTCG" => Self::Qemu,
            b" KVMKVMKVM  " => Self::Kvm,
            b"VMwareVMware" => Self::Vmware,
            b"VBoxVBoxVBox" => Self::VirtualBox,
            b"XenVMMXenVMM" => Self::Xen,
            b"Microsoft Hv" => Self::HyperV,
            b" prl hyperv " => Self::Parallels,
            b" lrpepyh vr " => Self::Parallels,
            b"bhyve bhyve " => Self::Bhyve,
            b" QNXQVMBSQG " => Self::Qnx,
            _ => Self::Unknown,
        }
    }
}

static CPUID_SUPPORTED: Mutex<IsItUninit<bool>> = Mutex::new(IsItUninit::uninit());

fn check_for_cpuid() -> bool {
    let lock = CPUID_SUPPORTED.lock();
    if let Some(supported) = lock.try_get_ref() {
        *supported
    } else {
        unsafe {
            // Test for CPUID support. We can do this by changing the ID bit in EFLAGS and if it
            // actually changes then it's supported
            let supported: u32;

            core::arch::asm!(
                "pushfd",
                "pushfd",
                "xor dword ptr [esp], {id_bit}",
                "popfd",
                "pushfd",
                "pop eax",
                "xor eax, [esp]",
                "popfd",
                "and eax, {id_bit}",
                id_bit = const 1 << 21,
                out("eax") supported
            );

            supported != 0
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct CpuidGp {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

fn _cpuid(
    gp: CpuidGp,
    gp_out: &mut CpuidGp,
    bypass_cpuid_check: bool,
    bypass_cpuid_leaf_check: bool,
) {
    if !bypass_cpuid_check && !check_for_cpuid() {
        panic!("cpuid() called on a machine with no CPUID support");
    }

    if !bypass_cpuid_leaf_check {
        ensure_cpuid_leaf_supported(gp.eax);
    }

    unsafe {
        core::arch::asm!("cpuid", in("eax") gp.eax, in("ebx") gp.ebx, in("ecx") gp.ecx, in("edx") gp.edx, lateout("eax") gp_out.eax, lateout("ebx") gp_out.ebx, lateout("ecx") gp_out.ecx, lateout("edx") gp_out.edx);
    }
}

fn cpuid(gp: CpuidGp, gp_out: &mut CpuidGp) {
    _cpuid(gp, gp_out, false, false);
}

fn ensure_cpuid_leaf_supported(leaf: u32) {
    let mut out = CpuidGp::default();

    _cpuid(
        CpuidGp {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        },
        &mut out,
        false,
        true,
    );

    if out.eax < leaf {
        panic!("CPUID Leaf {leaf} requested but it's not supported on this machine!");
    }
}

pub fn get_vendor() -> Vendor {
    let mut out = CpuidGp::default();

    cpuid(
        CpuidGp {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        },
        &mut out,
    );

    let mut vendor_str: [u8; 12] = [0; 12];
    vendor_str[0] = (out.ebx & 0xFF) as u8;
    vendor_str[1] = ((out.ebx >> 8) & 0xFF) as u8;
    vendor_str[2] = ((out.ebx >> 16) & 0xFF) as u8;
    vendor_str[3] = ((out.ebx >> 24) & 0xFF) as u8;
    vendor_str[4] = (out.edx & 0xFF) as u8;
    vendor_str[5] = ((out.edx >> 8) & 0xFF) as u8;
    vendor_str[6] = ((out.edx >> 16) & 0xFF) as u8;
    vendor_str[7] = ((out.edx >> 24) & 0xFF) as u8;
    vendor_str[8] = (out.ecx & 0xFF) as u8;
    vendor_str[9] = ((out.ecx >> 8) & 0xFF) as u8;
    vendor_str[10] = ((out.ecx >> 16) & 0xFF) as u8;
    vendor_str[11] = ((out.ecx >> 24) & 0xFF) as u8;
    Vendor::from_vendor_bytes(&vendor_str)
}

pub fn feature_present(feature: &Features) -> bool {
    let mut out = CpuidGp::default();

    cpuid(
        CpuidGp {
            eax: 1,
            ebx: 0,
            ecx: 0,
            edx: 0,
        },
        &mut out,
    );

    (out.edx & (*feature as u32)) != 0
}

fn print_features() {
    let features = &[
        Features::Fpu,
        Features::Vme,
        Features::De,
        Features::Pse,
        Features::Tsc,
        Features::Msr,
        Features::Pae,
        Features::Mce,
        Features::Cx8,
        Features::Apic,
        Features::Sep,
        Features::Mtrr,
        Features::Pge,
        Features::Mca,
        Features::Cmov,
        Features::Pat,
        Features::Pse36,
        Features::Psn,
        Features::Clflush,
        Features::Ds,
        Features::Acpi,
        Features::Mmx,
        Features::Fxsr,
        Features::Sse,
        Features::Sse2,
        Features::Ss,
        Features::Htt,
        Features::Tm,
        Features::Ia64,
        Features::Pbe,
    ];

    for feature in features {
        if feature_present(feature) {
            debug!("CPU feature: {feature:?}");
        }
    }
}

fn print_vendor() {
    let vendor = get_vendor();
    debug!("CPU vendor: {vendor:?}");
}

pub fn print_cpuid() {
    debug!("Printing CPU information");

    debug!("--------------------------------------");

    print_vendor();
    print_features();

    debug!("--------------------------------------");
}
