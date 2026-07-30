#!/bin/sh

set -eu

printf '%s\n' 'USB device'
ioreg -p IOUSB -n 'USB Composite Device' -r -l -w 0 -d 2

printf '\n%s\n' 'HID collections'
hidutil list | awk '
  NR == 1 ||
  tolower($1) == "0x514c" && tolower($2) == "0x8850"
'
