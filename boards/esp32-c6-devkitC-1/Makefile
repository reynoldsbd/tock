# Licensed under the Apache License, Version 2.0 or the MIT License.
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Copyright Tock Contributors 2024.

# Makefile for building the tock kernel for the ESP32-C6-DevKitC-1 platform

include ../Makefile.common

# todo: flash, not run
run: $(TOCK_ROOT_DIRECTORY)target/$(TARGET)/release/$(PLATFORM).elf
	$(Q)probe-rs run --chip=esp32c6 $^

# todo: flash app using probe-rs
