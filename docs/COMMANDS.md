# Shell Commands

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
