# Demo Walkthrough

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
