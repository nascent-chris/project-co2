#![no_std]
#![no_main]

// pick a panicking behavior
// use panic_halt as _;
use panic_semihosting as _;
// you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use cortex_m::asm;
use cortex_m_rt::entry;
use stm32f1xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    // take ownership of the device peripherals singleton
    let p = pac::Peripherals::take().unwrap();

    // configure GPIOC pin 13 as a push-pull output to drive the on-board LED
    let mut gpioc = p.GPIOC.split();
    // push-pull output because we want to drive the LED
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    loop {
        // toggle the LED state
        led.toggle();

        // wait for 125ms
        // the external clock is 8MHz
        // so 8_000_000 cycles = 1 second
        // 1_000_000 cycles = 1/8 seconds = 125ms
        asm::delay(1_000_000);
    }
}
