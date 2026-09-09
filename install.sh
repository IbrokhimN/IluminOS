#!/bin/bash

set -e

echo "Installing dependencies..."

which qemu-system-x86_64 > /dev/null 2>&1 || {
  echo "Downloading QEMU..."
  apt-get update -qq
  apt-get install -y -qq qemu-system-x86 qemu-utils > /dev/null 2>&1
}

which rustc > /dev/null 2>&1 || {
  echo "Downloading Rust..."
  apt-get install -y -qq rustc cargo > /dev/null 2>&1
}

which make > /dev/null 2>&1 || {
  echo "Downloading Make..."
  apt-get install -y -qq make > /dev/null 2>&1
}

echo "Done"
