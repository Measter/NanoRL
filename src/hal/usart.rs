//! A basic USART implementation to enable sending serial data for debugging.

#![allow(dead_code)]
use crate::hal::{register::Register, CPU_FREQ};
use core::marker::PhantomData;

pub mod registers {
    reg! {
        /// USART 0 I/O Data Register
        UDR0: u8 {
            addr: 0xC6,
            write mask: 0xFF,
        }
    }

    reg! {
        /// USART 0 Control and Status Register A
        UCSR0A: u8 {
            addr: 0xC0,
            write mask: 0b0100_0011,
            bits: {
                /// USART 0 Multi-processor Communication Mode
                MPCM0 = 0, RW;
                /// USART 0 Double Transmission Speed
                U2X0 = 1, RW;
                /// USART 0 Parity Error
                UPE0 = 2, R;
                /// USART 0 Data OverRun
                DOR0 = 3, R;
                /// USART 0 Frame Error
                FE0 = 4, R;
                /// USART 0 Data Register Empty
                UDRE0 = 5, R;
                /// USART 0 Transmit Complete
                TXC0 = 6, RW;
                /// USART 0 Receive Complete
                RXC0 = 7, R;
            }
        }
    }

    reg! {
        /// USART 0 Control and Status Register B
        UCSR0B: u8 {
            addr: 0xC1,
            write mask: 0b1111_1101,
            bits: {
                /// USART 0 Transmit Data Bit 8
                TXB80 = 0, RW;
                /// USART 0 Receive Data Bit 8
                RBX80 = 1, R;
                /// USART 0 Character Size
                UCSZ02 = 2, RW;
                /// USART 0 Transmitter Enable
                TXEN0 = 3, RW;
                /// USART 0 Receiever Enable
                RXEN0 = 4, RW;
                /// USART 0 Data Register Empty Interrupt Enable
                UDRIE0 = 5, RW;
                /// USART 0 TX Complete Interrupt Enable
                TXCIE0 = 6, RW;
                /// USART 0 RX Complete Interrupt Enable
                RXCIE0 = 7, RW;
            }
        }
    }

    reg! {
        /// USART 0 Control and Status Register C
        UCSR0C: u8 {
            addr: 0xC2,
            write mask: 0xFF,
            bits: {
                /// USART 0 Clock Polarity
                UCPOL0 = 0, RW;
                /// USART 0 Character Size - Bit 0
                UCSZ00 = 1, RW;
                /// USART 0 Character Size - Bit 1
                UCSZ01 = 2, RW;
                /// USART 0 Stop Bit Select
                USBS0 = 3, RW;
                /// USART 0 Parity Mode - Bit 0
                UPM00 = 4, RW;
                /// USART 0 Parity Mode - Bit 1
                UPM01 = 5, RW;
                /// USART 0 Mode Select - Bit 0
                UMSEL00 = 6, RW;
                /// USART 0 Mode Select - Bit 1
                UMSEL01 = 7, RW;
            }
        }
    }

    reg! {
        /// USART 0 Baud Rate Register
        UBRR0: u16 {
            addr: 0xC4,
            write mask: 0x0FFF,
        }
    }
}

use registers::*;

/// The baud rate we'll use for serial.
///
/// 9600 is the default for PuTTY, so I've just used that value.
const BAUD_RATE: u32 = 9600;
/// The calculated value to put into the UBBR register to set the baud rate.
const UBBR_VAL: u16 = ((CPU_FREQ / 8 / BAUD_RATE) - 1) as u16;

/// Tracks whether the USART has been initilised.
static mut HAS_INIT: bool = false;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum USARTError {
    InitError,
}

/// Provides an interface to the USART.
///
/// implements the minimum needed to synchronously send data to a host PC, and is intended
/// for debugging.
///
/// Configured for:
///
/// * 9600 baud
/// * 8-bit characters
/// * 1 stop bit
/// * No parity bit
///
/// Only one instance can live at a time.
pub struct USART(PhantomData<()>);

impl USART {
    pub fn init() -> Result<USART, USARTError> {
        unsafe {
            if HAS_INIT {
                Err(USARTError::InitError)
            } else {
                // Set our baud rate.
                UBRR0::set_raw_value(UBBR_VAL);

                // Configure for:
                // * 2x speed
                // * 8-bit characters
                // * 1 stop bit
                // * No parity
                // * Async mode,
                // * Enable RX/TX
                UCSR0A::set_value(UCSR0A::U2X0 | UCSR0A::MPCM0);
                UCSR0B::set_value(UCSR0B::RXEN0 | UCSR0B::TXEN0);
                UCSR0C::set_value(UCSR0C::UCSZ01 | UCSR0C::UCSZ00);

                HAS_INIT = true;
                Ok(USART(PhantomData))
            }
        }
    }

    pub fn send_byte(&mut self, data: u8) {
        unsafe {
            // Wait for the data register to become available.
            while !UCSR0A::get_bit(UCSR0A::UDRE0) {}

            UDR0::set_raw_value(data);
        }
    }

    pub fn send<T: AsRef<[u8]>>(&mut self, data: T) {
        fn inner(usart: &mut USART, data: &[u8]) {
            data.iter().for_each(|&b| usart.send_byte(b));
        }
        inner(self, data.as_ref());
    }
}

impl Drop for USART {
    fn drop(&mut self) {
        unsafe {
            UCSR0B::clear_bits(UCSR0B::TXEN0 | UCSR0B::RXEN0);
            HAS_INIT = false;
        }
    }
}
