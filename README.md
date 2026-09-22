# IluminOS 🦀

A 64-bit educational operating system written from scratch in Rust and running on bare metal (in QEMU). From boot and the login screen to a graphical interface with a browser that can reach real Gemini capsules over the network, everything is implemented from scratch, without the standard library.

Around 9,400 lines of custom code across 48 modules.

https://github.com/user-attachments/assets/54edbe24-f3ee-44f6-a634-697f1c2de1c1

<img width="1279" height="796" alt="изображение" src="https://github.com/user-attachments/assets/986c4fca-de99-4b3f-bcb9-a8a1650a98ce" />

## What IluminOS Can Do

* boots through the Limine bootloader in 64-bit mode
* login screen with username and password verification
* custom graphics output through a hand-written PSF font renderer (font, colors, cursor, scrolling, banner, themes)
* hand-written keyboard, mouse, and disk drivers
* filesystem with dynamic block allocation, inodes, and directories
* command shell with history, tab completion, and a large set of commands
* vim-style text editor with syntax highlighting
* real physical frame allocator and hand-written x86_64 page tables, with a kernel heap that grows on demand through a page fault handler
* hardware-based random number generator
* sound through the PC Speaker and a mini piano
* htop-style system monitor
* WebAssembly module execution (through the built-in `wasmi`)
* custom interpreted programming language
* flat-design windowed desktop shell with wallpaper, a taskbar, and per-app icons
* Not-Google browser with HTML parsing and rendering, plus a real Gemini protocol client with TLS 1.3
* a set of applications, terminal, clock, calculator, Paint, and a file manager
* networking: RTL8139 NIC driver, PCI scanner, smoltcp stack, a hand-written DNS resolver, working `ping`, and a Gemini (`gemini://`) client usable both from the console and from the GUI browser

## Quick Start
``` bash
chmod +x limine/limine
chmod +x install.sh 
./install.sh
```
Build and run it with a disk and network card like this.

```bash
make run QEMUFLAGS="-m 2G \
  -device piix3-ide,id=ide -drive id=disk,file=fs.img,format=raw,if=none -device ide-hd,drive=disk,bus=ide.0 \
  -netdev user,id=n0 -device rtl8139,netdev=n0"

```

Or just use the script.

```bash
chmod +x run.fish
./run.fish
```

The `-device rtl8139` flag is required for `lspci`, `nic`, `ping`, and `gemini`. Without it, the system works, but networking is unavailable.

A login screen appears on startup. Demo account: `root` / `iluminos`.

## Shell Commands

### Filesystem

| Command | Description |
| --- | --- |
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
| --- | --- |
| `run <file>` | execute a script written in the custom language |
| `wasm` | run the built-in WebAssembly module |
| `calc <expr>` | quick arithmetic |
| `mem` / `memtest` | heap status and dynamic memory test |

### System and Utilities

| Command | Description |
| --- | --- |
| `help` | list commands |
| `about` | about the author and system |
| `clear` / `cls` | clear the screen |
| `echo <text>` | print text |
| `rand [max]` | random number (hardware-generated) |
| `dice` | roll a six-sided die (hardware-generated) |
| `cowsay <text>` | ASCII cow |
| `uptime` / `date` | system uptime |
| `whoami` / `hostname` | system identity |
| `theme dark|light|everforest` | switch theme |
| `history` | command history |
| `htop` / `monitor` | system monitor |
| `piano` | mini piano |
| `beep` | short test tone through the PC Speaker |
| `sleep <n>` | pause for `n` seconds (1 to 60) |
| `banner` | show the startup ASCII banner again |
| `neofetch` | system summary with a small ASCII logo |
| `lock` | return to the login screen without powering off |
| `reboot` | reset the machine through the keyboard controller |
| `shutdown` | power off through the QEMU ACPI port |
| `gui` | launch graphical mode |

### Networking

| Command | Description |
| --- | --- |
| `lspci` | find the RTL8139 network card on the PCI bus |
| `nic` | initialize the card and read its MAC address |
| `ping <ip>` | ICMP echo (e.g. `ping 10.0.2.2`, the QEMU gateway) |
| `gemini` / `gem [url]` | open the console Gemini browser, optionally straight to a `gemini://` url |

## Architecture

The project is split into subsystem folders under `src/`, kcore, mem, drivers, fs, gui, apps, and shell, each owning its own part of the kernel.

### Boot and Output

**main.rs** is the entry point through `kmain`. It enables SSE at the CPU level (needed for the crypto code pulled in by the Gemini client), declares Limine requests (framebuffer), and initializes subsystems in order, memory management, the random number generator, timekeeping, and the filesystem, then displays the login screen and starts the shell.

**gui/framebuffer.rs** handles all screen output. Limine provides a graphics framebuffer (an array of pixels), and text is drawn through a hand-written PSF1/PSF2 font parser reading an embedded bitmap font file, instead of a fixed ASCII-only font table. It supports colors, cursor, scrolling, several themes (including an "everforest" palette), and GUI drawing primitives (rectangles, borders, text at arbitrary positions, scalable text).

**kcore/banner.rs** draws the startup IluminOS ASCII banner with a color gradient.

**kcore/login.rs** is the login screen. It is drawn pixel by pixel, a card with username and password fields, account verification, a shake animation on failure, and a sound chord on success. The `lock` shell command returns to this screen without shutting the system down.

### Drivers

**drivers/port.rs** reads and writes I/O ports through inline assembly (`inb`/`outb`, as well as `inw`/`outw` and 32-bit `inl`/`outl` for PCI). It was written manually instead of using an external crate to remove a dependency incompatible with the modern compiler.

**drivers/keyboard.rs** is the PS/2 keyboard driver using polling. It reads scan codes, converts them to characters, and handles Shift, Caps Lock, and arrow keys. It distinguishes keyboard and mouse bytes using a status bit.

**drivers/mouse.rs** is the PS/2 mouse driver. It initializes the controller's second channel, reads 3-byte packets (buttons and offsets), and moves the cursor.

**drivers/ata.rs** is the disk driver (ATA PIO). It reads and writes sectors through ports with timeouts and works with the piix3-ide controller.

**drivers/sound.rs** produces sound through the PC Speaker by configuring the PIT to the desired frequency, used by `beep`, `piano`, and the login/shutdown chords.

### Data Storage

**fs/mod.rs** is the filesystem. It uses dynamic block allocation through a bitmap, inodes with variable file sizes, and a directory hierarchy through a parent pointer. It supports a current working directory and path construction.

### Memory Management

**mem/allocator.rs** is the physical frame allocator. It reads the memory map handed over by the Limine bootloader and tracks free and used 4 KB frames in a bitmap, on top of the higher-half direct map (HHDM) Limine also provides.

**mem/paging.rs** is a hand-written x86_64 4-level page table implementation. It walks and builds PML4/PDPT/PD/PT tables directly, maps and unmaps pages with the usual present/writable/no-execute flags, and invalidates stale TLB entries with `invlpg`.

**mem/fault.rs** sets up its own IDT with handlers for page fault, general protection fault, and double fault. A page fault caused by a missing page inside the kernel heap region is treated as a request to grow the heap rather than a crash; anything else is reported and the CPU halts.

**mem/heap.rs** is the dynamically growing kernel heap that backs `Vec`, `String`, and `Box`. It starts small and commits new physical pages lazily as `mem/fault.rs` reports page faults inside its address range, up to a fixed maximum.

### Shell and Editor

**shell/mod.rs** is the command shell (REPL). It reads a command, executes it, and prints the result. The prompt shows the command counter and current path. It includes history (up/down arrows), Tab completion, a blinking cursor, and the full command set listed above.

**apps/editor.rs** is a vim-style text editor. It has three modes (normal, insert, command), hjkl navigation, `dd`, `dw`, `x`, `o` commands, Rust syntax highlighting, and saving through `:w` / `:wq`.

### Sound

**apps/piano.rs** is the mini piano in the console. Keys are converted into notes and played through the PC Speaker.

### Monitoring

**apps/monitor.rs** is the htop-style system monitor. It shows uptime, heap and disk usage as graphical bars, file count, and CPU cycles with periodic updates.

### Program Execution

**apps/wasm.rs** runs WebAssembly through the built-in `wasmi` interpreter. A compiled module with `add`, `factorial`, and `fib` functions runs inside the OS on bare metal.

**apps/script.rs** is the interpreter for the custom mini-language. It has a tokenizer and a recursive-descent parser with correct operator precedence, and it supports variables (`let`), output (`print`), and arithmetic with parentheses. The same parser powers the `calc` command.

### Console Gemini Client

**apps/gemini.rs** is a console Gemini browser in the style of clients like amfora. It keeps its own navigation history, follows redirects, handles servers that ask for text input, and renders pages with a built-in pager (scroll with the arrow keys, jump to a link by typing its number).

### Graphical Interface

**gui/wm.rs** is the window manager. It defines a common `Widget` trait that every app implements (draw, click, key press, drag, periodic tick), and draws the shared window frame, title bar, and close button around whichever app is currently open.

**gui/desktop.rs** draws the desktop background and wallpaper, the row of app icons, and a taskbar, and runs the main event loop that opens an app on icon click and routes further clicks, key presses, and drags to it.

**gui/style.rs** is the shared visual language for the windowed interface, a dark flat color palette with rounded rectangles, drawn through `embedded-graphics` primitives.

**gui/html.rs** is the parser for an HTML subset. It supports h1-h6 headings, paragraphs, formatting (b, i, u, code), color (`font color`), lists (ul, ol, li), links, quotes, and separators. It produces a list of blocks for rendering.

**gui/gemtext.rs** parses the `text/gemini` format used by Gemini pages into the same kind of block list `gui/html.rs` produces, so the browser can lay out and draw a Gemini capsule with the same code it already uses for HTML.

**gui/widgets/browser.rs** is the Not-Google browser window. It can run a fake local search, open local `.html` files parsed by `gui/html.rs`, and fetch real `gemini://` capsules through the network client, following redirects and rendering clickable links.

**gui/widgets/files.rs** is a graphical file manager. It browses the real on-disk filesystem with folder navigation, per-type icons, and buttons to create or delete files and directories.

**gui/widgets/term.rs**, **gui/widgets/clock.rs**, **gui/widgets/calc.rs**, and **gui/widgets/paint.rs** are the remaining desktop apps, a terminal window running the same shell commands, a clock, a calculator, and a raster paint program with a palette and mouse drawing.

### Networking

**drivers/net/pci.rs** is the PCI bus scanner. It finds a device by vendor/device, reads BAR0 and IRQ, and enables bus mastering.

**drivers/net/rtl8139.rs** is the RTL8139 network card driver using polling (without interrupts). It handles reset, the receive ring buffer, and frame transmission and reception, and it reads the MAC address.

**drivers/net/device.rs** is the layer between the driver and the smoltcp stack through the `Device` trait.

**drivers/net/net.rs** is the smoltcp stack on top of the card. It handles interface configuration, the ICMP socket, and the `ping` command.

**drivers/net/dns.rs** is a small hand-written DNS resolver built on a raw UDP socket. It sends a single A-record query and parses the answer by hand, enough to turn a Gemini capsule's hostname into an IPv4 address.

**drivers/net/gemini_proto.rs** is the Gemini protocol client. It parses `gemini://` urls, opens a blocking TCP connection through smoltcp, performs a real TLS 1.3 handshake over it (through `embedded-tls`), and speaks the Gemini request/response format. Used by both the console client and the GUI browser.

**drivers/net/interrupts.rs** is the IDT and IRQ skeleton for switching the network card from polling to interrupts (not connected yet).

## Key Technical Decisions

### Why Limine Instead of a Custom Bootloader

Writing a bootloader from scratch is a large project of its own (switching CPU modes, setting up memory pages, parsing ELF). The project originally used `bootloader 0.9`, but it proved incompatible with the modern compiler because of its dependency chain. After several failed builds, the project switched to Limine, which depends only on `core` and the lightweight `limine` crate. A lesson in the fragility of the bare-metal ecosystem on an unstable compiler.

### Why a Hand-Written PSF Font Instead of the Old ASCII Table

The kernel used to embed the `font8x8` crate, which only covers ASCII. Text is now drawn through a small PSF1/PSF2 parser reading an embedded bitmap font file that also carries extended glyphs, so the renderer itself is no longer limited to plain ASCII the way it used to be.

### Why a Real Page Table Implementation Instead of a Static Heap

The heap used to be a single static array handed to `linked_list_allocator`, with a fixed upper size. The kernel now tracks physical memory with its own frame bitmap, builds real x86_64 page tables, and grows the kernel heap lazily by mapping new pages when a page fault lands inside its address range. This needed an IDT with real page fault, general protection fault, and double fault handlers, so parts of the interrupt machinery are now in active use even though device IRQs (keyboard, mouse, network card) still use polling.

### Why Polling Instead of Interrupts for Devices

The keyboard, mouse, and network card are still read using polling rather than interrupts. Polling is simpler (no handler registration or queues needed for them specifically), although the CPU wastes cycles doing so. The interrupt framework for networking (`drivers/net/interrupts.rs`) is already outlined as a direction for future development.

### How Networking Works

Networking is built in layers. The hand-written RTL8139 driver moves raw Ethernet frames through the card's ring buffer, a thin layer passes those frames to the smoltcp stack, and smoltcp builds IP, ICMP, UDP, and TCP packets from them. Reception uses polling, so `ping`, the DNS resolver, and the Gemini client all run their own `poll` loop, send a request, and wait for a reply. The addresses are hard-coded for QEMU user mode (ours is `10.0.2.15`, gateway `10.0.2.2`, DNS forwarder `10.0.2.3`), so `ping 10.0.2.2` responds immediately and hostnames resolve without any extra setup.

### How the Gemini Client Reaches Real Capsules

`gemini://` is a small, TLS-only protocol, so reaching a real capsule needs the same three ingredients as reaching a real website, DNS, TCP, and TLS, all written by hand for this kernel. A request line is a single url followed by a line break; the response is a two-digit status code, a meta field, and a body read until the connection closes. Because the TLS crate used here cannot verify certificate chains in a `no_std` build, the handshake is genuinely encrypted but the server certificate is currently accepted unconditionally, so this protects against passive eavesdropping but not an active man in the middle.

### Why WebAssembly Is Embedded Instead of Loaded from Disk

Files in the filesystem are size-limited, and there is no way to load a binary module into the OS from outside. Therefore, the demo wasm module is embedded into the kernel using `include_bytes`.

### Why Double Buffering Is Not Used

Full double buffering requires copying the entire screen every frame. On bare metal without graphics acceleration, this copying is performed by the CPU and is too slow (causing a noticeable drop in frame rate). Therefore, partial redraw is used, only the changed area is updated (for example, the area under the mouse cursor).

### Why Sound May Not Be Audible

Sound is produced through the PC Speaker (PIT timer). To hear it in QEMU, a suitable audio backend is required. Without one, the `piano` command, `beep`, and other sound signals still execute, but silence is normal.

## How the Key Subsystems Work

### Filesystem

The disk is divided into regions, the superblock, the free-block bitmap, the inode table, and the data blocks. Each file is described by an inode that stores an array of data block numbers (rather than a fixed address), allowing variable file sizes. A directory is also an inode with a directory flag; a file's membership in a directory is defined by a pointer to the parent inode. This is how the directory tree is built.

### Dynamic Memory

`Vec`, `String`, and `Box` come from the `alloc` crate. The global allocator (`linked_list_allocator`) sits on top of a region of virtual address space that starts small and grows on demand, rather than a single fixed-size static array. When the allocator runs out of already-mapped memory, the resulting page fault is caught by `mem/fault.rs`, which asks `mem/heap.rs` to map another physical frame at that address and lets the faulting instruction simply retry. The allocator itself still keeps a list of free blocks and merges adjacent ones on deallocation, which happens automatically through the Drop mechanism.

### Custom Language Interpreter

It works as a real interpreter. The tokenizer splits the text into tokens, and the recursive-descent parser builds the expression. Operator precedence arises automatically from the nesting of parsing functions, addition calls multiplication for its operands, so multiplication binds more tightly. Variables are stored in a table.

### HTML and Gemini Rendering

The HTML parser walks through the text, finds tags, and treats the content between them as text. A current style is maintained (size, color, weight/style). An opening tag changes the style, while a closing tag restores it. Text between tags inherits the active style. The parser outputs a list of blocks, and the same renderer draws each with its own font, color, and spacing regardless of whether the blocks came from an HTML file or from a Gemini page, since the Gemini parser produces the same block list. Headings are larger through scaling, each glyph pixel is drawn as an NxN square.

### Random Number Generation

On bare metal there is no OS-provided source of randomness. Entropy comes from the CPU cycle counter (`rdtsc`), whose lower bits depend on precise timing. This produces the seed, and the number stream is generated by the fast `xorshift64` PRNG. The same generator backs `rand`, `dice`, and the ephemeral keys used during a Gemini TLS handshake.

## Demo Walkthrough

Sequence for demonstrating all features:

```
(login: root / iluminos)
help                    all commands
about                   about the author
neofetch                system summary
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
dice                    roll a die
calc 2 + 3 * 4          calculator
wasm                    WebAssembly execution (42, 120, 55)
edit prog.txt           write: let x = 5 / print x * 10
run prog.txt            execute the custom language
htop                    system monitor (q to exit)
piano                   mini piano (Esc to exit)
lspci                   find the network card
nic                     read MAC address
ping 10.0.2.2           ping the QEMU gateway
gemini gemini://geminiprotocol.net   browse a real capsule (q to exit)
edit page.html          write HTML: <h1>Hello</h1><p>text</p>
theme everforest        switch theme
gui                     graphical mode

```

In graphical mode you can do the following.

* click the Terminal icon, terminal in a window running the same shell
* click Not-Google, browser, enter `page.html` and press Search to render it, or type a `gemini://` url to browse a real capsule with clickable links
* click Files, browse, create, and delete files and folders with the mouse
* click Clock, clock
* click Calc, calculator (click the buttons with the mouse)
* click Paint, draw with the mouse using the palette
* click `[x]`, close the window
* Esc, return to the console

## Project Structure

```
kernel/src/
  main.rs               entry point, initialization
  kcore/
    mod.rs               module wiring
    banner.rs            startup banner
    login.rs             login screen
    random.rs            random number generator
    time.rs              timekeeping
  mem/
    mod.rs               module wiring
    allocator.rs         physical frame allocator
    paging.rs            hand-written x86_64 page tables
    fault.rs             IDT and fault handlers
    heap.rs              dynamically growing kernel heap
  drivers/
    mod.rs               module wiring
    port.rs               I/O ports
    keyboard.rs           keyboard driver
    mouse.rs               mouse driver
    ata.rs                 disk driver
    sound.rs               sound through PC Speaker
    net/
      mod.rs               module wiring
      pci.rs               PCI bus scanner
      rtl8139.rs            network card driver
      device.rs             smoltcp adapter
      net.rs                 smoltcp stack and ping
      dns.rs                 hand-written DNS resolver
      gemini_proto.rs        Gemini protocol client (url, TCP, TLS)
      interrupts.rs          IDT/IRQ skeleton for the network card
  fs/
    mod.rs                filesystem
  gui/
    mod.rs                module wiring
    framebuffer.rs         graphics output, PSF font renderer
    html.rs                 HTML parser
    gemtext.rs               text/gemini parser
    wm.rs                    window manager
    desktop.rs                desktop, wallpaper, taskbar
    style.rs                  shared window styling
    widgets/
      mod.rs                 module wiring
      browser.rs             Not-Google browser (HTML + Gemini)
      files.rs                file manager
      term.rs                  terminal window
      clock.rs                 clock
      calc.rs                   calculator
      paint.rs                  paint
  apps/
    mod.rs                console apps wiring
    editor.rs             text editor
    monitor.rs              system monitor
    piano.rs                 mini piano
    script.rs                 custom language interpreter
    wasm.rs                    WebAssembly execution
    gemini.rs                   console Gemini browser
    demo.wasm                    embedded wasm module
  shell/
    mod.rs                command shell

```

Dependencies are `limine`, `spin`, `linked_list_allocator`, `wasmi`, `smoltcp`, `embedded-graphics`, `embedded-text`, `embedded-tls`, and `embedded-io`.

## Possible Future Improvements

* interrupts for devices (keyboard, mouse, network card) instead of polling, the skeleton is already in place for networking
* interrupt- and timer-based multitasking
* real certificate verification for the Gemini TLS client
* further networking, ARP/DHCP, a simple HTTP client
* indirect blocks in inodes for large files
* overlapping/movable windows instead of one active app at a time
* save Paint drawings to a file
* games as GUI applications

## Author

**Ibrokhim Nurullaev**, [github.com/IbrokhimN](https://github.com/IbrokhimN)  
**Kamron Burkhanov**, [github.com/kbur-coder](https://github.com/kbur-coder)


An educational project, an operating system demonstrating systems programming in Rust, from booting on bare metal to a graphical interface with a browser that can reach real capsules over its own network stack.
