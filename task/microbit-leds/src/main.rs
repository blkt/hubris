// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]

use core::ptr::write_volatile;

// Make sure we actually link in userlib, despite not using any of it explicitly
// - we need it for our _start routine.
extern crate userlib;

#[export_name = "main"]
fn main() -> ! {
    loop {
	const GPIO0_PINCNF21_ROW1_ADDR: *mut u32 = 0x5000_0754 as *mut u32;
	const GPIO0_PINCNF28_COL1_ADDR: *mut u32 = 0x5000_0770 as *mut u32;

	const DIR_OUTPUT_POS: u32 = 0;
	const PINCNF_DRIVE_LED: u32 = 1 << DIR_OUTPUT_POS;

	unsafe {
	    write_volatile(GPIO0_PINCNF21_ROW1_ADDR, PINCNF_DRIVE_LED);
	    write_volatile(GPIO0_PINCNF28_COL1_ADDR, PINCNF_DRIVE_LED);
	}

	const GPIO0_OUT_ADDR: *mut u32 = 0x5000_0504 as *mut u32;
	const GPIO0_OUT_ROW1_POS: u32 = 21;

	let mut is_on: bool = false;
	loop {
	    unsafe {
		write_volatile(GPIO0_OUT_ADDR, (is_on as u32) << GPIO0_OUT_ROW1_POS);
	    }

	    for _ in 0..1_000_000 {
		cortex_m::asm::nop();
	    }
	    is_on = !is_on;
	}
    }
}
