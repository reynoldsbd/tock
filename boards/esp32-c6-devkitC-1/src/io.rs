// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2024.

use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::ptr;

use kernel::debug;
use kernel::utilities::io_write::IoWrite;

use esp32_c6::rom;

use crate::PANIC_RESOURCES;

struct RomWriter;

impl Write for RomWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        rom::write_bytes(s.as_bytes());

        Ok(())
    }
}

impl IoWrite for RomWriter {
    fn write(&mut self, buf: &[u8]) -> usize {
        rom::write_bytes(buf);

        buf.len()
    }
}

static mut WRITER: RomWriter = RomWriter;

#[panic_handler]
fn panic_fmt(pi: &PanicInfo) -> ! {
    unsafe {
        let writer = &mut *ptr::addr_of_mut!(WRITER);

        debug::panic_print_old(writer, pi, &rv32i::support::nop, PANIC_RESOURCES.get());
    }

    loop {
        rv32i::support::nop();
    }
}
