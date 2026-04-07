#!/usr/bin/env bash
set -euo pipefail

CHIP="STM32H747xi"
ELF="${1:?usage: flash-cm4.sh path/to/cm4.elf}"


probe-rs download --chip "$CHIP" "$ELF"
probe-rs reset --chip "$CHIP"

echo "Connect RTT to primary CM7 core for output"
