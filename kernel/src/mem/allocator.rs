use core::slice;

use limine::memory_map::EntryType;
use limine::request::{HhdmRequest, MemoryMapRequest};
use spin::Mutex;

pub(crate) const FRAME_SIZE: u64 = 4096;

#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

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

// true once the bitmap is up, i.e. paging and the growable heap have
// something to allocate page frames from
pub(crate) fn has_frame_allocator() -> bool {
    FRAME_ALLOCATOR.lock().is_some()
}

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

pub fn alloc_frames(count: usize) -> Option<u64> {
    let mut guard = FRAME_ALLOCATOR.lock();
    let fb = guard.as_mut()?;
    let start = fb.find_free_run(count)?;
    fb.mark_range_used(start, count);
    Some(start as u64 * FRAME_SIZE)
}

// turn a physical address into a cpu usable pointer via the hhdm mapping
pub fn phys_to_virt(phys: u64) -> Option<*mut u8> {
    FRAME_ALLOCATOR
        .lock()
        .as_ref()
        .map(|fb| fb.phys_to_virt(phys))
}

#[allow(dead_code)]
pub fn free_frame(addr: u64) {
    if let Some(fb) = FRAME_ALLOCATOR.lock().as_mut() {
        fb.free_frame(addr);
    }
}

pub fn init() {
    try_init_from_memory_map();
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
    let highest_addr = entries.iter().map(|e| e.base + e.length).max().unwrap_or(0);
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
    let bits: &'static mut [u8] = unsafe { slice::from_raw_parts_mut(bitmap_ptr, bitmap_bytes) };

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

    *FRAME_ALLOCATOR.lock() = Some(fb);
    true
}

// total heap bytes currently backed by physical memory
pub fn heap_size() -> usize {
    crate::mem::heap::heap_size()
}

// currently used bytes
pub fn heap_used() -> usize {
    crate::mem::heap::heap_used()
}

pub fn heap_free() -> usize {
    crate::mem::heap::heap_free()
}
