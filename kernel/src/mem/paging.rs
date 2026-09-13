// our own x86_64 4 level page tables
use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::mem::allocator::{self, FRAME_SIZE};

pub const PRESENT: u64 = 1 << 0;
pub const WRITABLE: u64 = 1 << 1;
#[allow(dead_code)]
pub const USER: u64 = 1 << 2;
pub const NO_EXECUTE: u64 = 1 << 63;

const ADDR_MASK: u64 = 0x000f_ffff_ffff_f000;

#[repr(align(4096))]
struct PageTable([u64; 512]);

static ROOT_PML4: AtomicU64 = AtomicU64::new(0);

fn read_cr3() -> u64 {
    let value: u64;
    unsafe { asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value & ADDR_MASK
}

fn write_cr3(phys: u64) {
    unsafe { asm!("mov cr3, {}", in(reg) phys, options(nostack, preserves_flags)) };
}

fn invlpg(virt: u64) {
    unsafe { asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags)) };
}

fn table_at(phys: u64) -> &'static mut PageTable {
    let ptr =
        allocator::phys_to_virt(phys).expect("page table frame outside hhdm") as *mut PageTable;
    unsafe { &mut *ptr }
}

fn indices(virt: u64) -> (usize, usize, usize, usize) {
    let i4 = ((virt >> 39) & 0x1ff) as usize;
    let i3 = ((virt >> 30) & 0x1ff) as usize;
    let i2 = ((virt >> 21) & 0x1ff) as usize;
    let i1 = ((virt >> 12) & 0x1ff) as usize;
    (i4, i3, i2, i1)
}

fn align_down(addr: u64) -> u64 {
    addr & !(FRAME_SIZE - 1)
}

fn next_table(table: &mut PageTable, index: usize) -> u64 {
    let entry = table.0[index];
    if entry & PRESENT != 0 {
        return entry & ADDR_MASK;
    }
    let phys = allocator::alloc_frames(1).expect("out of physical memory building page tables");
    let child = table_at(phys);
    child.0 = [0u64; 512];
    table.0[index] = phys | PRESENT | WRITABLE;
    phys
}

fn next_table_existing(table: &PageTable, index: usize) -> Option<u64> {
    let entry = table.0[index];
    (entry & PRESENT != 0).then_some(entry & ADDR_MASK)
}

pub fn init() {
    if !allocator::has_frame_allocator() {
        return;
    }

    let limine_pml4 = table_at(read_cr3());
    let our_phys = allocator::alloc_frames(1).expect("out of physical memory for the initial pml4");
    let ours = table_at(our_phys);
    ours.0 = limine_pml4.0; // same mappings, new table we control

    ROOT_PML4.store(our_phys, Ordering::Release);
    write_cr3(our_phys);
}

pub fn owns_tables() -> bool {
    ROOT_PML4.load(Ordering::Acquire) != 0
}

pub fn map(virt: u64, phys: u64, flags: u64) -> Result<(), &'static str> {
    let root = ROOT_PML4.load(Ordering::Acquire);
    if root == 0 {
        return Err("paging not initialized");
    }
    let (i4, i3, i2, i1) = indices(virt);

    let pml4 = table_at(root);
    let pdpt = table_at(next_table(pml4, i4));
    let pd = table_at(next_table(pdpt, i3));
    let pt = table_at(next_table(pd, i2));

    pt.0[i1] = align_down(phys) | flags | PRESENT;
    invlpg(align_down(virt));
    Ok(())
}

#[allow(dead_code)]
pub fn unmap(virt: u64) {
    let root = ROOT_PML4.load(Ordering::Acquire);
    if root == 0 {
        return;
    }
    let (i4, i3, i2, i1) = indices(virt);

    let pml4 = table_at(root);
    let Some(pdpt_phys) = next_table_existing(pml4, i4) else {
        return;
    };
    let pdpt = table_at(pdpt_phys);
    let Some(pd_phys) = next_table_existing(pdpt, i3) else {
        return;
    };
    let pd = table_at(pd_phys);
    let Some(pt_phys) = next_table_existing(pd, i2) else {
        return;
    };
    let pt = table_at(pt_phys);

    pt.0[i1] = 0;
    invlpg(align_down(virt));
}

pub fn is_mapped(virt: u64) -> bool {
    let root = ROOT_PML4.load(Ordering::Acquire);
    if root == 0 {
        return false;
    }
    let (i4, i3, i2, i1) = indices(virt);

    let pml4 = table_at(root);
    let Some(pdpt_phys) = next_table_existing(pml4, i4) else {
        return false;
    };
    let pdpt = table_at(pdpt_phys);
    let Some(pd_phys) = next_table_existing(pdpt, i3) else {
        return false;
    };
    let pd = table_at(pd_phys);
    let Some(pt_phys) = next_table_existing(pd, i2) else {
        return false;
    };
    let pt = table_at(pt_phys);

    pt.0[i1] & PRESENT != 0
}
