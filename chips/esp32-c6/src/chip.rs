// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use core::fmt::Write;

use rv32i::csr;
use rv32i::syscall::SysCall;

use kernel::platform::chip::{Chip, InterruptService};
use kernel::utilities::registers::interfaces::Readable;

pub struct Esp32C6DefaultPeripherals<'a> {
    // todo
    _a: core::marker::PhantomData<&'a ()>,
}

impl<'a> Esp32C6DefaultPeripherals<'a> {
    pub fn new() -> Self {
        Self {
            _a: core::marker::PhantomData,
        }
    }
}

impl InterruptService for Esp32C6DefaultPeripherals<'_> {
    unsafe fn service_interrupt(&self, _interrupt: u32) -> bool {
        todo!()
    }
}

pub struct Esp32C6<'a, I: InterruptService + 'a> {
    userspace_kernel_boundary: SysCall,
    interrupt_service: &'a I,
}

impl<'a, I: InterruptService + 'a> Esp32C6<'a, I> {
    pub unsafe fn new(interrupt_service: &'a I) -> Self {
        let userspace_kernel_boundary = SysCall::new();
        Self {
            userspace_kernel_boundary,
            interrupt_service,
        }
    }
}

impl<'a, I: InterruptService + 'a> Chip for Esp32C6<'a, I> {
    type MPU = ();
    fn mpu(&self) -> &Self::MPU {
        &()
    }

    type UserspaceKernelBoundary = SysCall;
    fn userspace_kernel_boundary(&self) -> &Self::UserspaceKernelBoundary {
        &self.userspace_kernel_boundary
    }

    fn has_pending_interrupts(&self) -> bool {
        todo!()
    }

    fn service_pending_interrupts(&self) {
        todo!()
    }

    fn sleep(&self) {
        unsafe {
            rv32i::support::wfi();
        }
    }

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        rv32i::support::atomic(f)
    }

    unsafe fn print_state(&self, writer: &mut dyn Write) {
        let mcval: csr::mcause::Trap = core::convert::From::from(csr::CSR.mcause.extract());
        let _ = writer.write_fmt(format_args!("\r\n---| RISC-V Machine State |---\r\n"));
        let _ = writer.write_fmt(format_args!("Last cause (mcause): "));
        rv32i::print_mcause(mcval, writer);
        let interrupt = csr::CSR.mcause.read(csr::mcause::mcause::is_interrupt);
        let code = csr::CSR.mcause.read(csr::mcause::mcause::reason);
        let _ = writer.write_fmt(format_args!(
            " (interrupt={}, exception code={:#010X})",
            interrupt, code
        ));
        let _ = writer.write_fmt(format_args!(
            "\r\nLast value (mtval):  {:#010X}\
         \r\n\
         \r\nSystem register dump:\
         \r\n mepc:    {:#010X}    mstatus:     {:#010X}\
         \r\n mtvec:   {:#010X}",
            csr::CSR.mtval.get(),
            csr::CSR.mepc.get(),
            csr::CSR.mstatus.get(),
            csr::CSR.mtvec.get()
        ));
        let mstatus = csr::CSR.mstatus.extract();
        let uie = mstatus.is_set(csr::mstatus::mstatus::uie);
        let sie = mstatus.is_set(csr::mstatus::mstatus::sie);
        let mie = mstatus.is_set(csr::mstatus::mstatus::mie);
        let upie = mstatus.is_set(csr::mstatus::mstatus::upie);
        let spie = mstatus.is_set(csr::mstatus::mstatus::spie);
        let mpie = mstatus.is_set(csr::mstatus::mstatus::mpie);
        let spp = mstatus.is_set(csr::mstatus::mstatus::spp);
        let _ = writer.write_fmt(format_args!(
            "\r\n mstatus: {:#010X}\
         \r\n  uie:    {:5}  upie:   {}\
         \r\n  sie:    {:5}  spie:   {}\
         \r\n  mie:    {:5}  mpie:   {}\
         \r\n  spp:    {}",
            mstatus.get(),
            uie,
            upie,
            sie,
            spie,
            mie,
            mpie,
            spp
        ));
    }
}

#[export_name = "_start_trap_rust_from_kernel"]
pub unsafe extern "C" fn start_trap_rust() {
    todo!()
}
