// physical frame allocator built on limine memory map

use core::slice;
use core::sync::atomic::{AtomicUsize, Ordering};

use limine::request::{HhdmRequest, MemoryMapRequest};
use limine::memory_map::EntryType;
use linked_list_allocator::LockedHeap;
use spin::Mutex;

const FRAME_SIZE: u64 = 4096;

// lower and upper bound for kernel heap size
const HEAP_MIN: usize = 1024 * 1024;
const HEAP_MAX: usize = 64 * 1024 * 1024;

#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// emergency fallback heap if bootloader gives no memory map
const EMERGENCY_HEAP_SIZE: usize = 1024 * 1024;
#[repr(align(16))]
struct EmergencyHeap([u8; EMERGENCY_HEAP_SIZE]);
static mut EMERGENCY_HEAP: EmergencyHeap = EmergencyHeap([0; EMERGENCY_HEAP_SIZE]);

// actual heap byte count computed at runtime
static HEAP_SIZE: AtomicUsize = AtomicUsize::new(0);

// physical page bitmap one bit per 4kb frame 0 free 1 used
struct FrameBitmap {
    bits: &'static mut [u8],
    hhdm_offset: u64,
}

impl FrameBitmap {
    fn frame_count(&self) -> usize {
        self.bits.len() * 8
    }

    fn is_free(&self, frame: usize) -> bool {
        self.bits[frame / 8] & (1 << (frame % 8)) == 0
    }

    fn set_used(&mut self, frame: usize) {
        self.bits[frame / 8] |= 1 << (frame % 8);
    }

    fn set_free(&mut self, frame: usize) {
        self.bits[frame / 8] &= !(1 << (frame % 8));
    }

    fn mark_range_used(&mut self, start_frame: usize, count: usize) {
        for f in start_frame..start_frame + count {
            self.set_used(f);
        }
    }

    fn mark_range_free(&mut self, start_frame: usize, count: usize) {
        for f in start_frame..start_frame + count {
            self.set_free(f);
        }
    }

    // find first run of count free frames
    fn find_free_run(&self, count: usize) -> Option<usize> {
        let total = self.frame_count();
        let mut run_start = 0usize;
        let mut run_len = 0usize;
        for frame in 0..total {
            if self.is_free(frame) {
                if run_len == 0 {
                    run_start = frame;
                }
                run_len += 1;
                if run_len == count {
                    return Some(run_start);
                }
            } else {
                run_len = 0;
            }
        }
        None
    }

    // allocate one physical frame return its address
    #[allow(dead_code)]
    fn alloc_frame(&mut self) -> Option<u64> {
        let frame = self.find_free_run(1)?;
        self.set_used(frame);
        Some(frame as u64 * FRAME_SIZE)
    }

    // free a physical frame by its address
    #[allow(dead_code)]
    fn free_frame(&mut self, addr: u64) {
        let frame = (addr / FRAME_SIZE) as usize;
        if frame < self.frame_count() {
            self.set_free(frame);
        }
    }

    fn phys_to_virt(&self, phys: u64) -> *mut u8 {
        (phys + self.hhdm_offset) as *mut u8
    }
}

static FRAME_ALLOCATOR: Mutex<Option<FrameBitmap>> = Mutex::new(None);

// total frames tracked by the bitmap
#[allow(dead_code)]
pub fn frame_count() -> usize {
    FRAME_ALLOCATOR
        .lock()
        .as_ref()
        .map(|fb| fb.frame_count())
        .unwrap_or(0)
}

// allocate one physical frame directly for future drivers or paging
#[allow(dead_code)]
pub fn alloc_frame() -> Option<u64> {
    FRAME_ALLOCATOR.lock().as_mut()?.alloc_frame()
}

// allocate count contiguous physical frames return physical base address
// used by dma capable devices that need a real hardware address not a kernel pointer
pub fn alloc_frames(count: usize) -> Option<u64> {
    let mut guard = FRAME_ALLOCATOR.lock();
    let fb = guard.as_mut()?;
    let start = fb.find_free_run(count)?;
    fb.mark_range_used(start, count);
    Some(start as u64 * FRAME_SIZE)
}

// turn a physical address into a cpu usable pointer via the hhdm mapping
pub fn phys_to_virt(phys: u64) -> Option<*mut u8> {
    FRAME_ALLOCATOR.lock().as_ref().map(|fb| fb.phys_to_virt(phys))
}

#[allow(dead_code)]
pub fn free_frame(addr: u64) {
    if let Some(fb) = FRAME_ALLOCATOR.lock().as_mut() {
        fb.free_frame(addr);
    }
}

pub fn init() {
    if try_init_from_memory_map() {
        return;
    }

    // fallback memory map unavailable use static emergency heap
    unsafe {
        let start = &raw const EMERGENCY_HEAP as usize;
        ALLOCATOR.lock().init(start as *mut u8, EMERGENCY_HEAP_SIZE);
    }
    HEAP_SIZE.store(EMERGENCY_HEAP_SIZE, Ordering::Relaxed);
}

fn try_init_from_memory_map() -> bool {
    let Some(mmap) = MEMORY_MAP_REQUEST.get_response() else {
        return false;
    };
    let Some(hhdm) = HHDM_REQUEST.get_response() else {
        return false;
    };

    let entries = mmap.entries();
    let hhdm_offset = hhdm.offset();

    // highest physical address used to size the bitmap
    let highest_addr = entries
        .iter()
        .map(|e| e.base + e.length)
        .max()
        .unwrap_or(0);
    if highest_addr == 0 {
        return false;
    }

    let total_frames = highest_addr.div_ceil(FRAME_SIZE) as usize;
    let bitmap_bytes = total_frames.div_ceil(8);
    let bitmap_frames_needed = (bitmap_bytes as u64).div_ceil(FRAME_SIZE);

    // find a usable region big enough to hold the bitmap
    let bitmap_region = entries.iter().find(|e| {
        e.entry_type == EntryType::USABLE && e.length >= bitmap_frames_needed * FRAME_SIZE
    });
    let Some(bitmap_region) = bitmap_region else {
        return false;
    };

    let bitmap_phys_base = bitmap_region.base;
    let bitmap_ptr = (bitmap_phys_base + hhdm_offset) as *mut u8;
    let bits: &'static mut [u8] =
        unsafe { slice::from_raw_parts_mut(bitmap_ptr, bitmap_bytes) };

    // default everything to used then open up usable regions
    bits.fill(0xFF);

    let mut fb = FrameBitmap { bits, hhdm_offset };

    for entry in entries.iter() {
        if entry.entry_type != EntryType::USABLE {
            continue;
        }
        let start_frame = (entry.base / FRAME_SIZE) as usize;
        let frame_len = (entry.length / FRAME_SIZE) as usize;
        fb.mark_range_free(start_frame, frame_len);
    }

    // reserve the space we used for the bitmap itself
    let bitmap_start_frame = (bitmap_phys_base / FRAME_SIZE) as usize;
    fb.mark_range_used(bitmap_start_frame, bitmap_frames_needed as usize);

    // count total free memory to pick a sensible heap size
    let usable_bytes: u64 = entries
        .iter()
        .filter(|e| e.entry_type == EntryType::USABLE)
        .map(|e| e.length)
        .sum();

    let desired = ((usable_bytes / 4) as usize).clamp(HEAP_MIN, HEAP_MAX);

    // try to find a contiguous free chunk shrink if not found
    let mut heap_bytes = desired;
    let heap_start_frame = loop {
        let frames_needed = (heap_bytes as u64).div_ceil(FRAME_SIZE) as usize;
        if frames_needed == 0 {
            return false;
        }
        if let Some(start) = fb.find_free_run(frames_needed) {
            break start;
        }
        if heap_bytes <= HEAP_MIN {
            return false;
        }
        heap_bytes /= 2;
    };

    let heap_frames = (heap_bytes as u64).div_ceil(FRAME_SIZE) as usize;
    fb.mark_range_used(heap_start_frame, heap_frames);
    let heap_phys = heap_start_frame as u64 * FRAME_SIZE;
    let heap_virt = fb.phys_to_virt(heap_phys);
    let heap_len = heap_frames * FRAME_SIZE as usize;

    unsafe {
        ALLOCATOR.lock().init(heap_virt, heap_len);
    }
    HEAP_SIZE.store(heap_len, Ordering::Relaxed);

    *FRAME_ALLOCATOR.lock() = Some(fb);
    true
}

// total heap bytes
pub fn heap_size() -> usize {
    HEAP_SIZE.load(Ordering::Relaxed)
}

// currently used bytes
pub fn heap_used() -> usize {
    ALLOCATOR.lock().used()
}

pub fn heap_free() -> usize {
    ALLOCATOR.lock().free()
}
