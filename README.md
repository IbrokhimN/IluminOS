# IluminOS

A 64-bit educational operating system written from scratch in Rust and running on bare metal (in QEMU). From boot and the login screen to a graphical interface with a browser and a network stack with `ping` — everything is implemented from scratch, without the standard library.

Around 6,000 lines of custom code across 28 modules.

https://github.com/user-attachments/assets/54edbe24-f3ee-44f6-a634-697f1c2de1c1

![Терминал IluminOS](https://raw.githubusercontent.com/IbrokhimN/IluminOS/refs/heads/main/docs/tty.png)

## What IluminOS Can Do

- boots through the Limine bootloader in 64-bit mode
- login screen with username and password verification
- custom graphics output (font, colors, cursor, scrolling, banner, themes)
- hand-written keyboard, mouse, and disk drivers
- filesystem with dynamic block allocation, inodes, and directories
- command shell with history, tab completion, and a large set of commands
- vim-style text editor with syntax highlighting


- dynamic memory (custom heap and allocator)
- hardware-based random number generator
- sound through the PC Speaker and a mini piano
- htop-style system monitor
- WebAssembly module execution (through the built-in `wasmi`)
- custom interpreted programming language
- Windows 3.1-style graphical shell with mouse support
- Not-Google browser with HTML parsing and rendering
- a set of applications: terminal, clock, calculator, and Paint
- networking: RTL8139 NIC driver, PCI scanner, smoltcp stack, and working `ping`

## Quick Start

Requirements: Rust nightly with the `x86_64-unknown-none` target, QEMU, and Limine.

Build and run (with disk and network card):

```bash
make run QEMUFLAGS="-m 2G \
  -device piix3-ide,id=ide -drive id=disk,file=fs.img,format=raw,if=none -device ide-hd,drive=disk,bus=ide.0 \
  -netdev user,id=n0 -device rtl8139,netdev=n0"
```

Or use the script:

```bash
./run.fish
```

The `-device rtl8139` flag is required for `lspci`, `nic`, and `ping`. Without it, the system works, but networking is unavailable.

A login screen appears on startup. Demo account: `root` / `iluminos`.

## Shell Commands

### Filesystem

| Command | Description |
|---|---|
| `ls` | list files |
| `pwd` | current path |
| `cd <dir>` | change directory (`..` for parent, `/` for root) |
| `mkdir <name>` | create directory |
| `touch <name>` | create empty file |
| `cat <name>` | show file contents |
| `edit <name>` | editor (`:w`, `:q`, `:wq`) |
| `rm <name>` | remove a file or empty directory |
| `cp <src> <dst>` | copy a file |
| `tree` | directory tree |
| `find <name>` | recursive file search |
| `wc <file>` | line, word, and character counter |
| `df` | disk usage |

### Execution and Development

| Command | Description |
|---|---|
| `run <file>` | execute a script written in the custom language |
| `wasm` | run the built-in WebAssembly module |
| `calc <expr>` | quick arithmetic |
| `mem` / `memtest` | heap status and dynamic memory test |

### System and Utilities

| Command | Description |
|---|---|
| `help` | list commands |
| `about` | about the author and system |
| `clear` | clear the screen |
| `echo <text>` | print text |
| `rand [max]` | random number (hardware-generated) |
| `cowsay <text>` | ASCII cow |
| `uptime` / `date` | system uptime |
| `whoami` / `hostname` | system identity |
| `theme dark\|light` | switch theme |
| `history` | command history |
| `htop` | system monitor |
| `piano` | mini piano |
| `gui` | launch graphical mode |

### Networking

| Command | Description |
|---|---|
| `lspci` | find the RTL8139 network card on the PCI bus |
| `nic` | initialize the card and read its MAC address |
| `ping <ip>` | ICMP echo (e.g. `ping 10.0.2.2` — QEMU gateway) |

## Architecture

The project consists of 28 modules, each responsible for its own subsystem.

### Boot and Output

**main.rs** — `kmain` entry point. Declares Limine requests (framebuffer), initializes subsystems in order: allocator, random number generator, timekeeping, filesystem, displays the login screen, and starts the shell.

**framebuffer.rs** — all screen output. Limine provides a graphics framebuffer (an array of pixels), but there are no ready-made glyphs — every character is drawn pixel by pixel using the `font8x8` font table. Supports colors, cursor, scrolling, light and dark themes, as well as GUI drawing primitives (rectangles, borders, text at arbitrary positions, scalable text).

**banner.rs** — startup IluminOS ASCII banner with a color gradient.

**login.rs** — login screen. Drawn pixel by pixel: a card with username and password fields, account verification, a shake animation on failure, and a sound chord on success.

### Drivers

**port.rs** — reads and writes I/O ports through inline assembly (`inb`/`outb`, as well as `inw`/`outw` and 32-bit `inl`/`outl` for PCI). Written manually instead of using an external crate to remove a dependency incompatible with the modern compiler.

**keyboard.rs** — PS/2 keyboard driver using polling. Reads scan codes, converts them to characters, handles Shift, Caps Lock, and arrow keys. Distinguishes keyboard and mouse bytes using a status bit.

**mouse.rs** — PS/2 mouse driver. Initializes the controller's second channel, reads 3-byte packets (buttons and offsets), and moves the cursor.

**ata.rs** — disk driver (ATA PIO). Reads and writes sectors through ports with timeouts. Works with the piix3-ide controller.

### Data Storage

**fs.rs** — filesystem. Dynamic block allocation through a bitmap, inodes with variable file sizes, and directory hierarchy through a parent pointer. Supports a current working directory and path construction.

**allocator.rs** — heap and global allocator. Provides dynamic memory (`Vec`, `String`, `Box`) through `linked_list_allocator` over a static heap region.

**random.rs** — random number generator based on the CPU cycle counter (`rdtsc`) as an entropy source and `xorshift64` as the PRNG.

**time.rs** — system uptime tracking through the cycle counter: uptime in seconds, split into hours, minutes, and seconds.

### Shell and Editor

**shell.rs** — command shell (REPL). Reads a command, executes it, and prints the result. The prompt shows the command counter and current path. Includes history (up/down arrows), Tab completion, and a blinking cursor.

**editor.rs** — vim-style text editor. Three modes (normal, insert, command), hjkl navigation, `dd`, `dw`, `x`, `o` commands, Rust syntax highlighting, and saving through `:w` / `:wq`.

### Sound

**sound.rs** — sound through the PC Speaker: configure the PIT to the desired frequency, note table, and short tones.

**piano.rs** — mini piano in the console: keys are converted into notes and played through the PC Speaker.

### Monitoring

**monitor.rs** — htop-style system monitor: uptime, heap and disk usage shown as graphical bars, file count, and CPU cycles with periodic updates.

### Program Execution

**wasm.rs** — WebAssembly execution through the built-in `wasmi` interpreter. A compiled module with `add`, `factorial`, and `fib` functions runs inside the OS on bare metal.

**script.rs** — interpreter for the custom mini-language. A tokenizer and recursive-descent parser with correct operator precedence. Supports variables (`let`), output (`print`), and arithmetic with parentheses. The same parser powers the `calc` command.

### Graphical Interface

**gui.rs** — Windows 3.1-style graphical shell. Desktop with icons, windows with title bars and 3D borders, close buttons, and mouse event routing. Contains the terminal, Not-Google browser, clock, calculator, and Paint.

**html.rs** — parser for an HTML subset. Supports h1-h6 headings, paragraphs, formatting (b, i, u, code), color (`font color`), lists (ul, ol, li), links, quotes, and separators. Produces a list of blocks for rendering.

**apps.rs** — GUI applications: clock (uptime through the cycle counter), calculator (state machine, mouse control), and Paint (raster editor with a palette and mouse drawing).

![Графическая оболочка IluminOS](https://raw.githubusercontent.com/IbrokhimN/IluminOS/refs/heads/main/docs/gui.png)

### Networking

**tcp/** — networking subsystem:

- **pci.rs** — PCI bus scanner: finds a device by vendor/device, reads BAR0 and IRQ, and enables bus mastering.
- **rtl8139.rs** — RTL8139 network card driver using polling (without interrupts): reset, receive ring buffer, frame transmission and reception, and MAC address reading.
- **device.rs** — layer between the driver and the smoltcp stack through the `Device` trait.
- **net.rs** — smoltcp stack on top of the card: interface configuration, ICMP socket, and `ping` command.
- **interrupts.rs** — IDT and IRQ skeleton for switching from polling to interrupts (not connected yet).

## Key Technical Decisions

### Why Limine Instead of a Custom Bootloader

Writing a bootloader from scratch is a large project of its own (switching CPU modes, setting up memory pages, parsing ELF). The project originally used `bootloader 0.9`, but it proved incompatible with the modern compiler because of its dependency chain. After several failed builds, the project switched to Limine, which depends only on `core` and the lightweight `limine` crate. A lesson in the fragility of the bare-metal ecosystem on an unstable compiler.

### Why the Output Is in English

The `font8x8` font in the BASIC_LEGACY set contains only ASCII. Cyrillic characters take two bytes per character in UTF-8 and would not render correctly.

### Why Polling Instead of Interrupts

The keyboard, mouse, and even the network card are read using polling rather than interrupts. Polling is simpler (no interrupt table, handlers, or queues are needed), although the CPU wastes cycles doing so. This is acceptable for a single-tasking system. The interrupt framework (`tcp/interrupts.rs`) is already outlined as a direction for future development.

### How Networking Works

Networking is built in three layers: the hand-written RTL8139 driver moves raw Ethernet frames through the card's ring buffer, a thin layer passes those frames to the smoltcp stack, and smoltcp builds IP and ICMP packets from them. Reception uses polling, so `ping` runs a `poll` loop, sends an echo request, and waits for a reply. The addresses are hard-coded for QEMU user mode (ours is `10.0.2.15`, gateway `10.0.2.2`), so `ping 10.0.2.2` responds immediately.

### Why WebAssembly Is Embedded Instead of Loaded from Disk

Files in the filesystem are size-limited, and there is no way to load a binary module into the OS from outside. Therefore, the demo wasm module is embedded into the kernel using `include_bytes`.

### Why Double Buffering Is Not Used

Full double buffering requires copying the entire screen every frame. On bare metal without graphics acceleration, this copying is performed by the CPU and is too slow (causing a noticeable drop in frame rate). Therefore, partial redraw is used — only the changed area is updated (for example, the area under the mouse cursor).

### Why Sound May Not Be Audible

Sound is produced through the PC Speaker (PIT timer). To hear it in QEMU, a suitable audio backend is required; without one, the `piano` command and sound signals still execute, but silence is normal.

## How the Key Subsystems Work

### Filesystem

The disk is divided into regions: superblock, free-block bitmap, inode table, and data blocks. Each file is described by an inode that stores an array of data block numbers (rather than a fixed address), allowing variable file sizes. A directory is also an inode with a directory flag; a file's membership in a directory is defined by a pointer to the parent inode. This is how the directory tree is built.

### Dynamic Memory

`Vec`, `String`, and `Box` come from the `alloc` crate. To make them work in `no_std`, a global allocator (`linked_list_allocator`) is defined over a static heap region. The allocator keeps a list of free blocks, allocates them on request, and merges adjacent blocks when they are freed. Deallocation happens automatically through the Drop mechanism.

### Custom Language Interpreter

It works as a real interpreter: the tokenizer splits the text into tokens, and the recursive-descent parser builds the expression. Operator precedence arises automatically from the nesting of parsing functions: addition calls multiplication for its operands, so multiplication binds more tightly. Variables are stored in a table.

### HTML Rendering

The parser walks through the text, finds tags, and treats the content between them as text. A current style is maintained (size, color, weight/style): an opening tag changes the style, while a closing tag restores it. Text between tags inherits the active style. The parser outputs a list of blocks, and the renderer draws each with its own font, color, and spacing. Headings are larger through scaling: each glyph pixel is drawn as an NxN square.

### Random Number Generation

On bare metal there is no OS-provided source of randomness. Entropy comes from the CPU cycle counter (`rdtsc`), whose lower bits depend on precise timing. This produces the seed, and the number stream is generated by the fast `xorshift64` PRNG.

## Demo Walkthrough

Sequence for demonstrating all features:

```
(login: root / iluminos)
help                    all commands
about                   about the author
mkdir projects
cd projects
pwd                     shows /projects
edit hello.rs           editor with syntax highlighting, write code, :wq
cat hello.rs
tree                    directory tree
cd /
df                      disk usage
mem                     heap
memtest                 dynamic memory in action
rand 100                random number
calc 2 + 3 * 4          calculator
wasm                    WebAssembly execution (42, 120, 55)
edit prog.txt           write: let x = 5 / print x * 10
run prog.txt            execute the custom language
htop                    system monitor (q to exit)
piano                   mini piano (Esc to exit)
lspci                   find the network card
nic                     read MAC address
ping 10.0.2.2           ping the QEMU gateway
edit page.html          write HTML: <h1>Hello</h1><p>text</p>
gui                     graphical mode
```

In graphical mode:

- click the Terminal icon — terminal in a window
- click Not-Google — browser, enter `page.html` and press Search to render
- click Clock — clock
- click Calc — calculator (click the buttons with the mouse)
- click Paint — draw with the mouse using the palette
- click `[x]` — close the window
- Esc — return to the console

## Project Structure

```
kernel/src/
  main.rs         entry point, initialization
  banner.rs       startup banner
  login.rs        login screen
  framebuffer.rs  graphics output
  port.rs         I/O ports
  keyboard.rs     keyboard driver
  mouse.rs        mouse driver
  ata.rs          disk driver
  fs.rs           filesystem
  allocator.rs    heap
  random.rs       random number generator
  time.rs         timekeeping
  shell.rs        command shell
  editor.rs       text editor
  sound.rs        sound through PC Speaker
  piano.rs        mini piano
  monitor.rs      system monitor
  wasm.rs         WebAssembly execution
  script.rs       custom language interpreter
  gui.rs          graphical shell
  html.rs         HTML parser
  apps.rs         GUI applications
  tcp/
    mod.rs        networking subsystem
    pci.rs        PCI bus scanner
    rtl8139.rs    network card driver
    device.rs     smoltcp adapter
    net.rs        smoltcp stack and ping
    interrupts.rs IDT/IRQ skeleton
  demo.wasm       embedded wasm module
```

Dependencies: `limine`, `spin`, `font8x8`, `linked_list_allocator`, `wasmi`, `smoltcp`.

## Possible Future Improvements

- interrupts (IDT table, handlers) instead of polling — the skeleton is already in place
- interrupt- and timer-based multitasking
- further networking: ARP/DHCP, TCP, DNS, simple HTTP client
- indirect blocks in inodes for large files
- clickable links in the browser, navigation between pages
- save Paint drawings to a file
- games as GUI applications

## Author

**Ibrokhim Nurullaev** — [github.com/IbrokhimN](https://github.com/IbrokhimN)

Educational project: an operating system demonstrating systems programming in Rust — from booting on bare metal to a graphical interface with a browser and a network stack.
