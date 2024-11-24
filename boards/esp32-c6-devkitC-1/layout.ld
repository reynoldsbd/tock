/* Licensed under the Apache License, Version 2.0 or the MIT License. */
/* SPDX-License-Identifier: Apache-2.0 OR MIT                         */
/* Copyright Tock Contributors 2024.                                  */

MEMORY
{
  ram (rwx) : ORIGIN = 0x40800000, LENGTH = 0x0006e610
  rom (rx)  : ORIGIN = 0x42000020, LENGTH = 0x001fffe0
  prog (rx) : ORIGIN = 0x42200000, LENGTH = 0x00200000
}

SECTIONS {
  /**
   * Bootloader really wants to have separate segments for ROTEXT and RODATA
   * Thus, we need to force a gap here.
   */
  .text_gap (NOLOAD): {
    . = . + 4;
    . = ALIGN(4) + 0x20;
  } > rom
}
INSERT BEFORE .storage;

INCLUDE tock_kernel_layout.ld

/* todo: either don't use rom funcs, or do it more gracefully */
uart_tx_one_char = 0x40000058;
