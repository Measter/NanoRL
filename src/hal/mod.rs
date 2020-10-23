#[macro_use]
pub mod register;
pub mod ports;
pub mod twi;
pub mod usart;
pub mod clock;
pub mod progmem;

const CPU_FREQ: u32 = 16_000_000;

pub fn enable_interrupts() {
    unsafe {
        llvm_asm!{
            "sei"
            :
            :
            :
            : "volatile"
        }
    }
}

pub fn disable_interrupts() {
    unsafe {
        llvm_asm!{
            "cli"
            :
            :
            :
            : "volatile"
        }
    }
}

/// Causes a delay of the given number of milliseconds.
///
/// Intended usage is for blinking the error codes.
///
/// With an entered delay of 100ms, actual time delayed seems to be 100.043ms.
/// Close enough to blink an LED.
#[inline(never)]
pub fn delay_millis(mut ms: u16) {
    const MAX_MS_PER_CALL: u16 = 16;
    const US_PER_MS: u16 = 1000;

    while ms > MAX_MS_PER_CALL {
        delay_micros(MAX_MS_PER_CALL * US_PER_MS);
        ms -= MAX_MS_PER_CALL;
    }

    delay_micros(ms * US_PER_MS);
}

/// This function allows the user to busy-loop for a given number of microseconds.
///
/// Its intended use is for the `delay_millis` function, so that it doesn't depend
/// on a timer being configured.
///
/// This was copied from the Arduino library's implementation, with the parts for 
/// other CPU frequencies ripped out.
#[inline(never)]
pub fn delay_micros(mut us: u16) {

    // This function assumes a 16MHz clock.
    // Call = 4 cycles + 2 to 4 cycles to init `us` (2 for constant delay, 4 for variable)
    
    // For a one-microsecond delay, simply return. The overhoad of the function call
    // takes 14 (16) cycles, which is 1us.

    if us <= 1 {
        return; // = 3 cycles ( 4 when true )
    }

    // The following loop takes 1/4 of a microsecond (4 cycles) per iteration, so
    // execute it four times for each microsecond of delay requested.
    us <<= 2;

    // Account for the time taken in the preceeding commands. We just burned 19 (21)
    // cycles above, remove 5, (5*4 = 20)us is at least 8 so we can subtract 5.
    us -= 5;

    // Busy wait.
    unsafe {
        llvm_asm!{
            "1: sbiw $0,1
            brne 1b"
            : "={r24}"(us)
            : "{r25}"(us >> 8),"{r24}"(us)
            : "r25", "r24"
            : "volatile"
        }
    }

    let _ = us;

    // Return = 4 cycles.
}

/// Blinks an error code at the user.
///
/// The code is the first 6 bits of the input number, with a short blink for 0,
/// long blink for 1, and a 1 second delay between sequences.
///
/// It uses pin PB5, which my Arduino Nano has an onboard LED connected to.
pub fn blink_error_code(code: u8) -> ! {
    use ports::{PortB, Port, PinMode};
    PortB::set_pin_mode(PortB::PB5, PinMode::Output);

    loop {
        let mut code = code;

        for _ in 0..6 {
            let bit = code & 0x1;
            let blink_len = if bit == 0 {
                100
            } else {
                250
            };

            PortB::set_port_high(PortB::PB5);
            delay_millis(blink_len);
            PortB::set_port_low(PortB::PB5);
            delay_millis(200);

            code >>= 1;
        }
        delay_millis(1000);
    }
}