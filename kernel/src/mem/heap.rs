// dynamically growing kernel heap

use core::sync::atomic::{AtomicUsize, Ordering};

use linked_list_allocator::LockedHeap;

use crate::mem::allocator::{self, FRAME_SIZE};
use crate::mem::paging;

const HEAP_START: u64 = 0xFFFF_9000_0000_0000;

const HEAP_INITIAL: usize = 1024 * 1024;
const HEAP_MAX: usize = 64 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static COMMITTED: AtomicUsize = AtomicUsize::new(0);

static DYNAMIC: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

const EMERGENCY_HEAP_SIZE: usize = 1024 * 1024;
#[repr(align(16))]
#[allow(dead_code)] // only the array's address is used, never its contents directly
struct EmergencyHeap([u8; EMERGENCY_HEAP_SIZE]);
static mut EMERGENCY_HEAP: EmergencyHeap = EmergencyHeap([0; EMERGENCY_HEAP_SIZE]);

fn map_and_commit(start: u64, len: usize) {
    let mut addr = start;
    let end = start + len as u64;
    while addr < end {
        let phys = allocator::alloc_frames(1).expect("out of physical memory growing kernel heap");
        paging::map(addr, phys, paging::WRITABLE | paging::NO_EXECUTE)
            .expect("failed to map kernel heap page");
        addr += FRAME_SIZE;
    }
    COMMITTED.fetch_add(len, Ordering::Relaxed);
}

pub fn init() {
    if !allocator::has_frame_allocator() || !paging::owns_tables() {
        // degraded boot, no bitmap or no page tables of our own to grow into
        unsafe {
            let start = &raw const EMERGENCY_HEAP as usize;
            ALLOCATOR.lock().init(start as *mut u8, EMERGENCY_HEAP_SIZE);
        }
        COMMITTED.store(EMERGENCY_HEAP_SIZE, Ordering::Relaxed);
        return;
    }

    map_and_commit(HEAP_START, HEAP_INITIAL);

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_MAX);
    }

    DYNAMIC.store(true, Ordering::Release);
}

pub fn grow(fault_addr: u64) -> bool {
    if !DYNAMIC.load(Ordering::Acquire) {
        return false;
    }
    if fault_addr < HEAP_START || fault_addr >= HEAP_START + HEAP_MAX as u64 {
        return false;
    }
    if paging::is_mapped(fault_addr) {
        // already backed, so this fault is a real protection violation
        // (e.g. a stray write through a bad pointer), not growth
        return false;
    }

    let page = fault_addr & !(FRAME_SIZE - 1);
    let Some(phys) = allocator::alloc_frames(1) else {
        return false; // physically out of memory, nothing more we can do
    };
    if paging::map(page, phys, paging::WRITABLE | paging::NO_EXECUTE).is_err() {
        return false;
    }
    COMMITTED.fetch_add(FRAME_SIZE as usize, Ordering::Relaxed);
    true
}

// bytes actually backed by physical memory right now, grows over time
pub fn heap_size() -> usize {
    COMMITTED.load(Ordering::Relaxed)
}

pub fn heap_used() -> usize {
    ALLOCATOR.lock().used()
}

pub fn heap_free() -> usize {
    heap_size().saturating_sub(heap_used())
}
