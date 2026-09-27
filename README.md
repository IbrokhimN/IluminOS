# IluminOS 🦀

[![build](https://github.com/IbrokhimN/IluminOS/actions/workflows/main.yml/badge.svg)](https://github.com/IbrokhimN/IluminOS/actions/workflows/main.yml)
![Rust](https://img.shields.io/badge/rust-nightly-orange?logo=rust)
![License](https://img.shields.io/github/license/IbrokhimN/IluminOS)

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

## Documentation

The full shell command reference, architecture breakdown, key technical decisions, internals of the main subsystems, demo walkthrough, and project structure live in the `docs` folder, to keep this README short.

* [docs/COMMANDS.md](docs/COMMANDS.md), the full shell command reference
* [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), the subsystem folders and what each file does
* [docs/DESIGN_DECISIONS.md](docs/DESIGN_DECISIONS.md), key technical decisions and why they were made
* [docs/INTERNALS.md](docs/INTERNALS.md), how the key subsystems work under the hood
* [docs/DEMO.md](docs/DEMO.md), a demo walkthrough sequence for showing off all features
* [docs/PROJECT_STRUCTURE.md](docs/PROJECT_STRUCTURE.md), the full source tree and dependencies

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
