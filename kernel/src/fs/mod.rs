// файловая система bitmap + inode + папки
// разметка диска суперблок / bitmap / таблица inode / блоки данных
// папка это inode с флагом is_dir
use crate::ata::{self, SECTOR_SIZE};
use spin::Mutex;

const MAGIC: u32 = 0x4D494E33; // "MIN3" - версия с папками

const BITMAP_START: u32 = 1;
const BITMAP_SECTORS: u32 = 2;
const INODE_START: u32 = 3;
const INODE_SECTORS: u32 = 32;
const DATA_START: u32 = 35;

pub const MAX_FILES: usize = 64;
pub const NAME_MAX: usize = 26;

const DIRECT_BLOCKS: usize = 12;
pub const FILE_MAX_BYTES: usize = DIRECT_BLOCKS * SECTOR_SIZE; // 6144

const INODE_SIZE: usize = 256;
const INODES_PER_SECTOR: usize = SECTOR_SIZE / INODE_SIZE;

const TOTAL_DATA_BLOCKS: u32 = 2000;

pub const ROOT_INODE: usize = 0;

// текущая директория (номер inode). глобальное состояние.
static CWD: Mutex<usize> = Mutex::new(ROOT_INODE);

// inode на диске used is_dir parent имя размер и 12 блоков
#[derive(Clone)]
pub struct Inode {
    pub used: bool,
    pub is_dir: bool,
    pub parent: u32,
    pub name: [u8; NAME_MAX],
    pub name_len: usize,
    pub size: u32,
    pub blocks: [u32; DIRECT_BLOCKS],
}

impl Inode {
    fn empty() -> Self {
        Inode {
            used: false,
            is_dir: false,
            parent: 0,
            name: [0; NAME_MAX],
            name_len: 0,
            size: 0,
            blocks: [0; DIRECT_BLOCKS],
        }
    }

    fn name_eq(&self, other: &str) -> bool {
        let ob = other.as_bytes();
        ob.len() == self.name_len && &self.name[..self.name_len] == ob
    }
}


fn read_bitmap(buf: &mut [u8; (BITMAP_SECTORS as usize) * SECTOR_SIZE]) {
    for s in 0..BITMAP_SECTORS {
        let mut sec = [0u8; SECTOR_SIZE];
        ata::read_sector(BITMAP_START + s, &mut sec);
        let off = (s as usize) * SECTOR_SIZE;
        buf[off..off + SECTOR_SIZE].copy_from_slice(&sec);
    }
}

fn write_bitmap(buf: &[u8; (BITMAP_SECTORS as usize) * SECTOR_SIZE]) {
    for s in 0..BITMAP_SECTORS {
        let off = (s as usize) * SECTOR_SIZE;
        let mut sec = [0u8; SECTOR_SIZE];
        sec.copy_from_slice(&buf[off..off + SECTOR_SIZE]);
        ata::write_sector(BITMAP_START + s, &sec);
    }
}

fn alloc_block() -> Option<u32> {
    let mut bm = [0u8; (BITMAP_SECTORS as usize) * SECTOR_SIZE];
    read_bitmap(&mut bm);
    for i in 0..(TOTAL_DATA_BLOCKS as usize) {
        let byte = i / 8;
        let bit = i % 8;
        if bm[byte] & (1 << bit) == 0 {
            bm[byte] |= 1 << bit;
            write_bitmap(&bm);
            return Some(DATA_START + i as u32);
        }
    }
    None
}

fn free_block(lba: u32) {
    if lba < DATA_START {
        return;
    }
    let i = (lba - DATA_START) as usize;
    let mut bm = [0u8; (BITMAP_SECTORS as usize) * SECTOR_SIZE];
    read_bitmap(&mut bm);
    let byte = i / 8;
    let bit = i % 8;
    bm[byte] &= !(1 << bit);
    write_bitmap(&bm);
}

// inode io

fn read_inode(index: usize) -> Inode {
    let sector = INODE_START + (index / INODES_PER_SECTOR) as u32;
    let offset = (index % INODES_PER_SECTOR) * INODE_SIZE;
    let mut buf = [0u8; SECTOR_SIZE];
    ata::read_sector(sector, &mut buf);

    let mut n = Inode::empty();
    n.used = buf[offset] == 1;
    if n.used {
        n.is_dir = buf[offset + 1] == 1;
        n.parent = u32::from_le_bytes([
            buf[offset + 2],
            buf[offset + 3],
            buf[offset + 4],
            buf[offset + 5],
        ]);
        let mut len = 0;
        for i in 0..NAME_MAX {
            let b = buf[offset + 6 + i];
            if b == 0 {
                break;
            }
            n.name[i] = b;
            len += 1;
        }
        n.name_len = len;
        n.size = u32::from_le_bytes([
            buf[offset + 32],
            buf[offset + 33],
            buf[offset + 34],
            buf[offset + 35],
        ]);
        for b in 0..DIRECT_BLOCKS {
            let o = offset + 36 + b * 4;
            n.blocks[b] = u32::from_le_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]);
        }
    }
    n
}

fn write_inode(index: usize, node: &Inode) {
    let sector = INODE_START + (index / INODES_PER_SECTOR) as u32;
    let offset = (index % INODES_PER_SECTOR) * INODE_SIZE;
    let mut buf = [0u8; SECTOR_SIZE];
    ata::read_sector(sector, &mut buf);

    for i in 0..INODE_SIZE {
        buf[offset + i] = 0;
    }
    buf[offset] = if node.used { 1 } else { 0 };
    buf[offset + 1] = if node.is_dir { 1 } else { 0 };
    let p = node.parent.to_le_bytes();
    buf[offset + 2] = p[0];
    buf[offset + 3] = p[1];
    buf[offset + 4] = p[2];
    buf[offset + 5] = p[3];
    for i in 0..node.name_len {
        buf[offset + 6 + i] = node.name[i];
    }
    let sz = node.size.to_le_bytes();
    buf[offset + 32] = sz[0];
    buf[offset + 33] = sz[1];
    buf[offset + 34] = sz[2];
    buf[offset + 35] = sz[3];
    for b in 0..DIRECT_BLOCKS {
        let o = offset + 36 + b * 4;
        let bl = node.blocks[b].to_le_bytes();
        buf[o] = bl[0];
        buf[o + 1] = bl[1];
        buf[o + 2] = bl[2];
        buf[o + 3] = bl[3];
    }

    ata::write_sector(sector, &buf);
}

// формат и инициализация диска

fn format() {
    let mut sb = [0u8; SECTOR_SIZE];
    sb[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    sb[4..8].copy_from_slice(&(SECTOR_SIZE as u32).to_le_bytes());
    ata::write_sector(0, &sb);

    let zero = [0u8; SECTOR_SIZE];
    for s in 0..BITMAP_SECTORS {
        ata::write_sector(BITMAP_START + s, &zero);
    }
    for s in 0..INODE_SECTORS {
        ata::write_sector(INODE_START + s, &zero);
    }

    // создать корневую директорию (inode 0)
    let mut root = Inode::empty();
    root.used = true;
    root.is_dir = true;
    root.parent = ROOT_INODE as u32; // корень сам себе родитель
    root.name[0] = b'/';
    root.name_len = 1;
    write_inode(ROOT_INODE, &root);
}

pub fn init() {
    let mut sb = [0u8; SECTOR_SIZE];
    ata::read_sector(0, &mut sb);
    let magic = u32::from_le_bytes([sb[0], sb[1], sb[2], sb[3]]);
    if magic != MAGIC {
        format();
    }
    *CWD.lock() = ROOT_INODE;
}

// навигация по папкам

pub fn cwd() -> usize {
    *CWD.lock()
}

// найти в директории dir запись с именем name -> вернуть индекс inode
fn find_in_dir(dir: usize, name: &str) -> Option<usize> {
    for i in 0..MAX_FILES {
        let n = read_inode(i);
        if n.used && n.parent as usize == dir && n.name_eq(name) {
            return Some(i);
        }
    }
    None
}

// найти файл в ТЕКУЩЕЙ директории (публичный, для shell/editor)
pub fn find(name: &str) -> Option<usize> {
    find_in_dir(cwd(), name)
}

// список содержимого текущей директории: callback(имя, размер, это_папка)
pub fn list<F: FnMut(&str, u32, bool)>(mut f: F) {
    let dir = cwd();
    for i in 0..MAX_FILES {
        let n = read_inode(i);
        if n.used && n.parent as usize == dir && i != ROOT_INODE {
            if let Ok(s) = core::str::from_utf8(&n.name[..n.name_len]) {
                f(s, n.size, n.is_dir);
            }
        }
    }
}

// список содержимого произвольной директории dir: callback(inode, имя, размер, папка)
pub fn list_dir<F: FnMut(usize, &str, u32, bool)>(dir: usize, mut f: F) {
    for i in 0..MAX_FILES {
        let n = read_inode(i);
        if n.used && n.parent as usize == dir && i != ROOT_INODE {
            if let Ok(s) = core::str::from_utf8(&n.name[..n.name_len]) {
                f(i, s, n.size, n.is_dir);
            }
        }
    }
}

// рекурсивный поиск по имени начиная с dir. callback(полный_путь_фрагмент, папка).
// вызывает f для каждого совпадения имени. глубина ограничена MAX_FILES.
pub fn find_recursive<F: FnMut(usize, bool)>(dir: usize, name: &str, f: &mut F) {
    for i in 0..MAX_FILES {
        let n = read_inode(i);
        if n.used && n.parent as usize == dir && i != ROOT_INODE {
            if n.name_eq(name) {
                f(i, n.is_dir); // нашли
            }
            if n.is_dir {
                find_recursive(i, name, f);
            }
        }
    }
}

// имя inode по индексу в буфер, вернуть длину
pub fn name_of(idx: usize, out: &mut [u8; NAME_MAX]) -> usize {
    let n = read_inode(idx);
    let len = n.name_len.min(NAME_MAX);
    out[..len].copy_from_slice(&n.name[..len]);
    len
}

// найти свободный inode
fn alloc_inode() -> Option<usize> {
    for i in 0..MAX_FILES {
        if !read_inode(i).used {
            return Some(i);
        }
    }
    None
}

// создать файл в текущей директории
pub fn create(name: &str) -> Result<(), &'static str> {
    if name.is_empty() || name.len() > NAME_MAX {
        return Err("name too long (max 26)");
    }
    if find(name).is_some() {
        return Err("already exists");
    }
    let idx = alloc_inode().ok_or("inode table full")?;

    let mut n = Inode::empty();
    n.used = true;
    n.is_dir = false;
    n.parent = cwd() as u32;
    n.name[..name.len()].copy_from_slice(name.as_bytes());
    n.name_len = name.len();
    write_inode(idx, &n);
    Ok(())
}

// создать папку в текущей директории
pub fn mkdir(name: &str) -> Result<(), &'static str> {
    if name.is_empty() || name.len() > NAME_MAX {
        return Err("name too long (max 26)");
    }
    if find(name).is_some() {
        return Err("already exists");
    }
    let idx = alloc_inode().ok_or("inode table full")?;

    let mut n = Inode::empty();
    n.used = true;
    n.is_dir = true;
    n.parent = cwd() as u32;
    n.name[..name.len()].copy_from_slice(name.as_bytes());
    n.name_len = name.len();
    write_inode(idx, &n);
    Ok(())
}

// сменить директорию: name может быть ".." или именем подпапки
pub fn chdir(name: &str) -> Result<(), &'static str> {
    if name == "/" {
        *CWD.lock() = ROOT_INODE;
        return Ok(());
    }
    if name == ".." {
        let cur = cwd();
        let node = read_inode(cur);
        *CWD.lock() = node.parent as usize;
        return Ok(());
    }
    if name == "." {
        return Ok(());
    }
    let idx = find(name).ok_or("no such directory")?;
    let node = read_inode(idx);
    if !node.is_dir {
        return Err("not a directory");
    }
    *CWD.lock() = idx;
    Ok(())
}

// построить путь текущей директории в переданный буфер, вернуть срез
pub fn pwd_into(buf: &mut [u8; 256]) -> usize {
    let cur = cwd();
    if cur == ROOT_INODE {
        buf[0] = b'/';
        return 1;
    }
    // собираем компоненты от текущей до корня, потом переворачиваем.
    // храним индексы inode по пути.
    let mut chain = [0usize; 16];
    let mut depth = 0;
    let mut node_idx = cur;
    while node_idx != ROOT_INODE && depth < 16 {
        chain[depth] = node_idx;
        depth += 1;
        node_idx = read_inode(node_idx).parent as usize;
    }
    // пишем от корня вниз
    let mut pos = 0;
    for d in (0..depth).rev() {
        buf[pos] = b'/';
        pos += 1;
        let n = read_inode(chain[d]);
        for i in 0..n.name_len {
            if pos < 256 {
                buf[pos] = n.name[i];
                pos += 1;
            }
        }
    }
    pos
}

// чтение и запись файлов

pub fn read(name: &str, out: &mut [u8; FILE_MAX_BYTES]) -> Result<usize, &'static str> {
    let idx = find(name).ok_or("file not found")?;
    let node = read_inode(idx);
    if node.is_dir {
        return Err("is a directory");
    }
    let size = node.size as usize;

    let mut read_total = 0;
    let mut bi = 0;
    while read_total < size && bi < DIRECT_BLOCKS {
        let blk = node.blocks[bi];
        if blk == 0 {
            break;
        }
        let mut buf = [0u8; SECTOR_SIZE];
        ata::read_sector(blk, &mut buf);
        let n = core::cmp::min(SECTOR_SIZE, size - read_total);
        out[read_total..read_total + n].copy_from_slice(&buf[..n]);
        read_total += n;
        bi += 1;
    }
    Ok(size)
}

pub fn write(name: &str, data: &[u8]) -> Result<(), &'static str> {
    if data.len() > FILE_MAX_BYTES {
        return Err("file too big (max 6144)");
    }
    let idx = find(name).ok_or("file not found")?;
    let mut node = read_inode(idx);
    if node.is_dir {
        return Err("is a directory");
    }

    for b in 0..DIRECT_BLOCKS {
        if node.blocks[b] != 0 {
            free_block(node.blocks[b]);
            node.blocks[b] = 0;
        }
    }

    let need = (data.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;
    if need > DIRECT_BLOCKS {
        return Err("file too big");
    }

    let mut written = 0;
    for b in 0..need {
        let blk = alloc_block().ok_or("disk full")?;
        node.blocks[b] = blk;

        let mut buf = [0u8; SECTOR_SIZE];
        let n = core::cmp::min(SECTOR_SIZE, data.len() - written);
        buf[..n].copy_from_slice(&data[written..written + n]);
        ata::write_sector(blk, &buf);
        written += n;
    }

    node.size = data.len() as u32;
    write_inode(idx, &node);
    Ok(())
}

// удалить файл или пустую папку из текущей директории
pub fn remove(name: &str) -> Result<(), &'static str> {
    let idx = find(name).ok_or("not found")?;
    let mut node = read_inode(idx);

    // нельзя удалить непустую папку
    if node.is_dir {
        for i in 0..MAX_FILES {
            let child = read_inode(i);
            if child.used && child.parent as usize == idx {
                return Err("directory not empty");
            }
        }
    }

    for b in 0..DIRECT_BLOCKS {
        if node.blocks[b] != 0 {
            free_block(node.blocks[b]);
        }
    }
    node = Inode::empty();
    write_inode(idx, &node);
    Ok(())
}

pub fn used_blocks() -> u32 {
    let mut bm = [0u8; (BITMAP_SECTORS as usize) * SECTOR_SIZE];
    read_bitmap(&mut bm);
    let mut count = 0;
    for i in 0..(TOTAL_DATA_BLOCKS as usize) {
        let byte = i / 8;
        let bit = i % 8;
        if bm[byte] & (1 << bit) != 0 {
            count += 1;
        }
    }
    count
}

pub fn total_blocks() -> u32 {
    TOTAL_DATA_BLOCKS
}
