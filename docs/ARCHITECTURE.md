# Architecture

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
