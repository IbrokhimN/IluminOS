#!/usr/bin/env fish
make run QEMUFLAGS="-m 2G -device piix3-ide,id=ide -drive id=disk,file=fs.img,format=raw,if=none -device ide-hd,drive=disk,bus=ide.0"
