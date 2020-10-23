//! This stuff is needed because the project is no_std. It's stuck in here because Rust-Analyser shows
//! "duplicate lang item" errors, and I didn't want to see them in my main.rs.

use crate::hal::{delay_millis, ports::{Port, PortB, PinMode}};

#[lang = "eh_personality"]
extern fn eh_personality() {}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // There's nothing we can do to handle a panic, so just go into a loop and blink the LED.
    PortB::set_pin_mode(PortB::PB5, PinMode::Output);
    loop {
        PortB::set_pin_toggle(PortB::PB5);
        delay_millis(500);
    }
}

#[no_mangle]
#[inline(always)]
pub unsafe extern "C" fn abort() {
    PortB::set_pin_mode(PortB::PB5, PinMode::Output);
    crate::hal::blink_error_code(0);
}

// I'm not linking to libc, so we need these functions. They may not be the best implementation, but they work.
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i16, n: usize) {
    let c = c as u8;
    let n = n as isize;
    
    for i in 0..n {
        s.offset(i).write(c);
    }
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(s: *mut u8, d: *mut u8, n: usize) {
    let n = n as isize;

    for i in 0..n {
        s.offset(i).write(d.offset(i).read());
    }
}