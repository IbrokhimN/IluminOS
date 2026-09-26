# Project Structure

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
