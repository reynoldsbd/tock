// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2024.

#![no_std]
#![no_main]

use core::ptr;

use kernel::capabilities::{MainLoopCapability, MemoryAllocationCapability};
use kernel::component::Component;
use kernel::ipc::{self, IPC};
use kernel::platform::{KernelResources, SyscallDriverLookup};
use kernel::process::Process;
use kernel::scheduler::cooperative::CooperativeSched;
use kernel::{create_capability, static_init, Kernel};

use capsules_system::process_printer::ProcessPrinterText;

use components::cooperative_component_static;
use components::sched::cooperative::CooperativeComponent;

use esp32_c6::chip::{Esp32C6, Esp32C6DefaultPeripherals};
use esp32_c6::rom;

mod io;

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x800] = [0; 0x800];

/// Reference to the chip for panic dumps.
static mut CHIP: Option<&'static Esp32C6<Esp32C6DefaultPeripherals>> = None;

/// Number of concurrent processes this platform supports.
const NUM_PROCS: usize = 4;

/// Actual memory for holding the active process structures.
static mut PROCESSES: [Option<&'static dyn Process>; NUM_PROCS] = [None; NUM_PROCS];

/// Static reference to process printer for panic dumps.
static mut PROCESS_PRINTER: Option<&'static ProcessPrinterText> = None;

/// Structure representing the ESP32-C6-DevKitC-1 development board
struct Platform {
    scheduler: &'static CooperativeSched<'static>,
    ipc: IPC<{ NUM_PROCS as u8 }>,
}

impl SyscallDriverLookup for Platform {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn kernel::syscall::SyscallDriver>) -> R,
    {
        match driver_num {
            ipc::DRIVER_NUM => f(Some(&self.ipc)),
            _ => f(None),
        }
    }
}

impl KernelResources<Esp32C6<'static, Esp32C6DefaultPeripherals<'static>>> for Platform {
    type SyscallDriverLookup = Self;
    fn syscall_driver_lookup(&self) -> &Self::SyscallDriverLookup {
        self
    }

    type SyscallFilter = ();
    fn syscall_filter(&self) -> &Self::SyscallFilter {
        &()
    }

    type ProcessFault = ();
    fn process_fault(&self) -> &Self::ProcessFault {
        &()
    }

    type ContextSwitchCallback = ();
    fn context_switch_callback(&self) -> &Self::ContextSwitchCallback {
        &()
    }

    type Scheduler = CooperativeSched<'static>;
    fn scheduler(&self) -> &Self::Scheduler {
        self.scheduler
    }

    type SchedulerTimer = ();
    fn scheduler_timer(&self) -> &Self::SchedulerTimer {
        &()
    }

    type WatchDog = ();
    fn watchdog(&self) -> &Self::WatchDog {
        &()
    }
}

/// This is in a separate, inline(never) function so that its stack frame is
/// removed when this function returns. Otherwise, the stack space used for
/// these static_inits is wasted.
#[inline(never)]
unsafe fn start() -> (
    &'static Kernel,
    Platform,
    &'static Esp32C6<'static, Esp32C6DefaultPeripherals<'static>>,
) {
    rom::println("start");

    let peripherals = static_init!(Esp32C6DefaultPeripherals, Esp32C6DefaultPeripherals::new());

    let chip = static_init!(
        Esp32C6<Esp32C6DefaultPeripherals>,
        Esp32C6::new(peripherals)
    );
    CHIP = Some(chip);

    let kernel = static_init!(Kernel, Kernel::new(&*ptr::addr_of!(PROCESSES)));

    let memory_allocation_capability = create_capability!(MemoryAllocationCapability);
    let ipc = IPC::new(kernel, ipc::DRIVER_NUM, &memory_allocation_capability);

    let scheduler = CooperativeComponent::new(&*ptr::addr_of!(PROCESSES))
        .finalize(cooperative_component_static!(NUM_PROCS));

    let platform = Platform { scheduler, ipc };

    (kernel, platform, chip)
}

#[no_mangle]
pub unsafe fn main() {
    rom::println("main");

    let main_loop_capability = create_capability!(MainLoopCapability);

    let (board_kernel, platform, chip) = start();
    board_kernel.kernel_loop(&platform, chip, Some(&platform.ipc), &main_loop_capability);
}
