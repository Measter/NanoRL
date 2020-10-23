//! Provides a trivial monotonic clock that ticks at 1khz
//!
//! The timer is configured for Clear Timer on Compare Match mode (CTC)
//! and has the OCR0A match interrupt enabled.
//!
//! This results in the match handler being executed when TCNT0 matchs
//! OCR0A, and then TCNT0 being reset.
//!
//! The interface is modelled after the Rust stdlib's Instant in that 
//! it's an opaque thing representing a moment in time.
//!
//! Cannot represent durations of more than 2^16 ms (approx. 65 seconds),
//! which is enough for this application.

use crate::hal::{CPU_FREQ, register::Register};
use core::marker::PhantomData;

pub mod registers {
    reg! {
        /// Timer/Counter 0 Control Register A
        TCCR0A: u8 {
            addr: 0x44,
            write mask: 0b1111_0011,
            bits: {
                /// Timer/Counter 0 Waveform Generation Mode - Bit 0
                WGM00 = 0, RW;
                /// Timer/Counter 0 Waveform Generation Mode - Bit 1
                WGM01 = 1, RW;
                /// Timer/Counter 0 Channel B Compare Output Mode - Bit 0
                COM0B0 = 4, RW;
                /// Timer/Counter 0 Channel B Compare Output Mode - Bit 1
                COM0B1 = 5, RW;
                /// Timer/Counter 0 Channel A Compare Output Mode - Bit 0
                COM0A0 = 6, RW;
                /// Timer/Counter 0 Channel A Compare Output Mode - Bit 1
                COM0A1 = 7, RW;
            }
        }
    }

    reg! {
        /// Timer/Counter 0 Control Register B
        TCCR0B: u8 {
            addr: 0x45,
            write mask: 0b1100_1111,
            bits: {
                /// Timer/Counter 0 Clock Select - Bit 0
                CS00 = 0, RW;
                /// Timer/Counter 0 Clock Select - Bit 1
                CS01 = 1, RW;
                /// Timer/Counter 0 Clock Select - Bit 2
                CS02 = 2, RW;
                /// Timer/Counter 0 Waveform Generation Mode - Bit 2
                WGM02 = 3, RW;
                /// Timer/Counter 0 Force Output Compare B
                FOC0B = 6, W;
                /// Timer/Counter 0 Force Output Compare A
                FOC0A = 7, W;
            }
        }
    }

    reg! {
        /// Timer/Counter 0 Interrupt Mask Register
        TIMSK0: u8 {
            addr: 0x6E,
            write mask: 0b0000_0111,
            bits: {
                /// Timer/Counter 0 Overflow Interrupt Enable
                TOIE = 0, RW;
                /// Timer/Counter 0 Output Compare A Match Interrupt Enable
                OCIEA = 1, RW;
                /// Timer/Counter 0 Output Compare B Match Interrupt Enable
                OCIEB = 2, RW;
            }
        }
    }

    reg! {
        /// Timer/Counter 0 Counter Value Register
        TCNT0: u8 {
            addr: 0x46,
            write mask: 0xFF,
        }
    }

    reg! {
        /// Timer/Counter 0 Output Compare Register A
        OCR0A: u8 {
            addr: 0x47,
            write mask: 0xFF,
        }
    }

    reg! {
        /// Timer/Counter 0 Output Compare Register B
        OCR0B: u8 {
            addr: 0x48,
            write mask: 0xFF,
        }
    }

    reg! {
        /// Timer/Counter 0 Interrupt Flag Register
        TIFR0: u8 {
            addr: 0x35,
            write mask: 0b0000_0111,
            bits: {
                /// Timer/Counter 0 Overflow Flag
                TOV = 0, RW;
                /// Timer/Counter 0 Output Compare A Match Flag
                OCFA = 1, RW;
                /// Timer/Counter 0 Output Compare B Match Flag
                OCFB = 2, RW;
            }
        }
    }
}
use registers::*;

/// The prescale setting we're using on the timer.
const PRESCALE: u32 = 64;

/// The Compare Match A value we'll use to control the frequence of the interrupt trigger.
const OCR0A_VALUE: u8 = (CPU_FREQ / 1000 / PRESCALE) as u8;

/// How many milliseconds have passed.
///
/// Because we don't use the absolute value anywhere, only the difference
/// between two instances, having it be u16 is fine enough for this purpose.
static mut TICKS: u16 = 0;

/// Keeps track of whether the clock has been initialised.
static mut HAS_INIT: bool = false;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ClockError {
    InitError,
}

/// A monotonic clock, to track the passing of time.
///
/// Only one instance of this can be alive at any one time.
pub struct Clock(PhantomData<()>);

impl Clock {
    pub fn init() -> Result<Clock, ClockError> {
        unsafe {
            if HAS_INIT {
                Err(ClockError::InitError)
            } else {
                // Setting CTC mode.
                TCCR0A::set_value(TCCR0A::WGM01);
                // Enable the OCRA interrupt.
                TIMSK0::set_bits(TIMSK0::OCIEA);
                
                TCNT0::set_raw_value(0);
                OCR0A::set_raw_value(OCR0A_VALUE);
                
                // Configure the prescaler for 64.
                TCCR0B::set_value(TCCR0B::CS00 | TCCR0B::CS01);
                HAS_INIT = true;

                Ok(Clock(PhantomData))
            }
        }
    }

    pub fn now(&self) -> Instant {
        // Because we're dealing with a multi-byte value which is updated in an interrupt
        // we need to make certain that we disable the interrupts while we do a volatile
        // read so that the value isn't updated in the middle of reading the value.
        unsafe {
            crate::hal::disable_interrupts();

            let ticks = &mut TICKS as *mut u16;
            let value = ticks.read_volatile();

            // Don't forget to re-enable interrupts or you break the clock and TWI.
            crate::hal::enable_interrupts();

            Instant(value)
        }
    }
}

impl Drop for Clock {
    fn drop(&mut self) {
        unsafe {
            // Disable the clock source.
            TCCR0B::clear_bits(TCCR0B::CS00 | TCCR0B::CS01);
            // Disable the interrupt.
            TIMSK0::clear_bits(TIMSK0::OCIEA);

            HAS_INIT = false;
        }
    }
}

/// Represents an instant in time.
#[derive(Copy, Clone, Default)]
pub struct Instant(u16);

impl Instant {
    pub fn elapsed(self, start: Instant) -> u16 {
        self.0.wrapping_sub(start.0)
    }
}

/// Timer/Counter 0 Output Compare A Match interrupt.
#[no_mangle]
pub unsafe extern "avr-interrupt" fn __vector_14() {
    // No need to disable interrupts here, as interrupts can't trigger
    // while in an interrupt handler.
    TICKS += 1;
}