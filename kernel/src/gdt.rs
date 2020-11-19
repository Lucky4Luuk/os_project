use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

//Found in `src/Cargo.toml`
pub const KERNEL_STACK_START: u64 = 0xFFFFFF8000000000;
pub const KERNEL_STACK_SIZE:  u64 = 512;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // tss.privilege_stack_table[0] = VirtAddr::new(KERNEL_STACK_START + KERNEL_STACK_SIZE * 4096);
        tss.privilege_stack_table[0] = {
            const STACK_SIZE: usize = 4096;
            static mut RING0_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &RING0_STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.add_entry(Descriptor::kernel_data_segment());

        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());

        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors {
            kernel_code_selector: kernel_code_selector,
            kernel_data_selector: kernel_data_selector,
            tss_selector: tss_selector,
            user_code_selector: user_code_selector,
            user_data_selector: user_data_selector,
        })
    };
}

static mut SELECTORS: Selectors = Selectors::new();

#[derive(Copy, Clone)]
struct Selectors {
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
}

impl Selectors {
    pub const fn new() -> Selectors {
        Selectors {
            kernel_code_selector: SegmentSelector(0),
            kernel_data_selector: SegmentSelector(0),
            user_code_selector: SegmentSelector(0),
            user_data_selector: SegmentSelector(0),
            tss_selector: SegmentSelector(0),
        }
    }
}

pub fn init() {
    use x86_64::instructions::segmentation::{set_cs, load_ss};
    use x86_64::instructions::tables::load_tss;

    // trace!("RPL: {:?}", GDT.1.user_code_selector.rpl()); //Prints "3", which is correct

    unsafe {
        SELECTORS = GDT.1;
    }

    GDT.0.load();

    unsafe {
        set_cs(GDT.1.kernel_code_selector);
        load_ss(GDT.1.kernel_data_selector);
        trace!("Kernel segments loaded!");
        load_tss(GDT.1.tss_selector);
        trace!("TSS loaded!");
    }

    trace!("GDT loaded!");
}

pub fn setup_usermode_gdt() {
    unsafe {
        x86_64::registers::model_specific::Star::write(
            SELECTORS.user_code_selector,
            SELECTORS.user_data_selector,
            SELECTORS.kernel_code_selector,
            SELECTORS.kernel_data_selector,
        )
        .unwrap();
    }
}

pub extern "C" fn syscall_entry() {
    info!("Syscall Entry!");
}
