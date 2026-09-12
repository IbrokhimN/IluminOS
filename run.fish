#!/usr/bin/env fish
make run QEMUFLAGS="-m 2G \
  -device piix3-ide,id=ide \
  -drive id=disk,file=fs.img,format=raw,if=none \
  -device ide-hd,drive=disk,bus=ide.0 \
  -device rtl8139,netdev=n0 \
  -netdev user,id=n0 \
  -object filter-dump,id=f0,netdev=n0,file=dump.pcap \
  -audiodev pipewire,id=snd \
  -machine pcspk-audiodev=snd"
