# Key Technical Decisions

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
