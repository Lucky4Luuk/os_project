use x86_64::{
    structures::paging::{
        mapper,
        PageTable,
        OffsetPageTable,
        Page,
        PhysFrame,
        Mapper,
        Size4KiB,
        FrameAllocator,
    },
    VirtAddr,
    PhysAddr,
};

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

/// Initialize a new OffsetPageTable.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Stack allocation
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackBounds {
    start: VirtAddr,
    end: VirtAddr,
}

impl StackBounds {
    pub fn start(&self) -> VirtAddr {
        self.start
    }

    pub fn end(&self) -> VirtAddr {
        self.end
    }
}

pub fn alloc_stack(
    size_in_pages: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<StackBounds, mapper::MapToError<Size4KiB>> {
    use core::sync::atomic::{AtomicU64, Ordering};
    use x86_64::structures::paging::PageTableFlags as Flags;

    static STACK_ALLOC_NEXT: AtomicU64 = AtomicU64::new(0x_5555_5555_0000);

    let guard_page_start = STACK_ALLOC_NEXT.fetch_add(
        (size_in_pages + 1) * Page::<Size4KiB>::SIZE,
        Ordering::SeqCst,
    );
    let guard_page = Page::from_start_address(VirtAddr::new(guard_page_start))
        .expect("`STACK_ALLOC_NEXT` not page aligned");

    let stack_start = guard_page + 1;
    let stack_end = stack_start + size_in_pages;
    let flags = Flags::PRESENT | Flags::WRITABLE;
    for page in Page::range(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(mapper::MapToError::FrameAllocationFailed)?;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush(); }
    }
    Ok(StackBounds {
        start: stack_start.start_address(),
        end: stack_end.start_address(),
    })
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//Page allocation
///////////////////////////////////////////////////////////////////////////////////////////////////
/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Address translation
///////////////////////////////////////////////////////////////////////////////////////////////////
/// Translates the given virtual address to the mapped physical address, or
/// `None` if the address is not mapped.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`.
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    translate_addr_inner(addr, physical_memory_offset)
}

/// Private function that is called by `translate_addr`.
///
/// This function is safe to limit the scope of `unsafe` because Rust treats
/// the whole body of unsafe functions as an unsafe block. This function must
/// only be reachable through `unsafe fn` from outside of this module.
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;

    // read the active level 4 frame from the CR3 register
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame;

    // traverse the multi-level page table
    for &index in &table_indexes {
        // convert the frame into a page table reference
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe {&*table_ptr};

        // read the page table entry and update `frame`
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    // calculate the physical address by adding the page offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Utility functions
///////////////////////////////////////////////////////////////////////////////////////////////////
/// TODO: Ensure address alignment
pub unsafe fn memory_read_32(addr: u64) -> u32 {
    let phys_addr = PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed) + addr;
    core::ptr::read(phys_addr as *mut u32)
}

/// TODO: Ensure address alignment
pub unsafe fn memory_write_32(addr: u64, value: u32) {
    let phys_addr = PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed) + addr;
    core::ptr::write(phys_addr as *mut u32, value);
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Singleton mapper, frame allocator and physical memory offset
///////////////////////////////////////////////////////////////////////////////////////////////////
lazy_static! {
    pub static ref MAPPER: spin::Mutex<Option<OffsetPageTable<'static>>> = spin::Mutex::new(None);
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: spin::Mutex<Option<BootInfoFrameAllocator>> = spin::Mutex::new(None);
}

use core::sync::atomic::{AtomicU64, Ordering};
pub static PHYSICAL_MEMORY_OFFSET: AtomicU64 = AtomicU64::new(0);
pub fn update_physical_memory_offset(phys_mem_offset: u64) {
    PHYSICAL_MEMORY_OFFSET.store(phys_mem_offset, Ordering::Relaxed);
}
