//! A very low-level interface for the TWI.
//!
//! Translated from the twi.c file in the Wire library, though the implementation is limited to
//! what was needed for this use case; specifically master transmit.

// Copyright (C) 2020 Stuart Haidon
// Ported from https://github.com/arduino/ArduinoCore-avr/blob/master/libraries/Wire/src/Wire.h

// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 2.1 of the License, or (at your option) any later version.

// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.

// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA


#![allow(dead_code)]

use crate::hal::{
    CPU_FREQ,
    register::Register,
    ports::registers::{PORTC, DDRC},
    progmem::ByteBundle,
};

pub const BUFFER_LEN: usize = 32;
const TWI_FREQ: u32 = 400_000;
const TWI_BIT_RATE: u8 = ((CPU_FREQ / TWI_FREQ - 16) / 2) as u8;

pub mod registers {
    reg! {
        /// TWI Bit Rate Register
        TWBR: u8 {
            addr: 0xB8,
            write mask: 0xFF,
        }
    }

    reg! {
        /// TWI Status Register
        TWSR: u8 {
            addr: 0xB9,
            write mask: 0b0000_0011,
            bits: {
                /// TWI Prescaler - Bit 0
                TWPS0 = 0, RW;
                /// TWI Prescaler - Bit 1
                TWPS1 = 1, RW;
                /// TWI Status - Bit 0
                TWS0 = 3, R;
                /// TWI Status - Bit 1
                TWS1 = 4, R;
                /// TWI Status - Bit 2
                TWS2 = 5, R;
                /// TWI Status - Bit 3
                TWS3 = 6, R;
                /// TWI Status - Bit 4
                TWS4 = 7, R;
            }
        }
    }

    // SAFETY: Must be a C-style enum, and MUST represent every state provided by the hardware.
    // See ATMega328P datasheet, section 21.7.
    #[derive(Copy, Clone, Eq, PartialEq)]
    #[repr(u8)]
    pub enum TWSRStatus {
        // All Master
        Start               = 0x08,
        RepStart            = 0x10,

        // Master Transmitter
        MtSlaAck            = 0x18,
        MtDataAck           = 0x28,
        MtSlaNack           = 0x20,
        MtDataNack          = 0x30,
        MtArbLost           = 0x38,

        // Master Receiver
        MrDataAck           = 0x50,
        MrSlaAck            = 0x40,
        MrDataNack          = 0x58,
        MrSlaNack           = 0x48,

        // Slave Receiver
        SrSlaAck            = 0x60,
        SrGCallAck          = 0x70,
        SrArbLostSlaAck     = 0x68,
        SrArbLostGCallAck   = 0x78,
        SrDataAck           = 0x80,
        SrGCallDataAck      = 0x90,
        SrStop              = 0xA0,
        SrDataNack          = 0x88,
        SrGCallDataNack     = 0x98,

        // Slave Transmitter
        StSlaAck            = 0xA8,
        StArbLostSlaAck     = 0xB0,
        StDataAck           = 0xB8,
        StDataNack          = 0xC0,
        StLastData          = 0xC8,
        
        // All
        NoInfo              = 0xF8,
        BusError            = 0x00,
    }

    impl TWSR {
        pub fn status() -> TWSRStatus {
            use crate::hal::register::Register;
            
            let mask = TWSR::TWS0 | TWSR::TWS1 | TWSR::TWS2 | TWSR::TWS3 | TWSR::TWS4;

            // SAFETY: The returned enum encompasses all states the hardware can supply as per the
            // ATMega328P datasheet, section 21.7.
            unsafe {
                let val = TWSR::get_value() & mask.raw_value();
                core::mem::transmute(val)
            }
        }
    }
    
    reg! {
        /// TWI (Slave) Address Register
        TWAR: u8 {
            addr: 0xBA,
            write mask: 0xFF,
            bits: {
                /// TWI General Call Recognition Enable
                TWGCE = 0, RW;
            }
        }
    }

    reg! {
        /// TWI Data Register
        TWDR: u8 {
            addr: 0xBB,
            write mask: 0xFF,
        }
    }

    reg! {
        /// TWI Control Register
        TWCR: u8 {
            addr: 0xBC,
            write mask: 0b1111_0101,
            bits: {
                /// TWI Interrupt Enable
                TWIE = 0, RW;
                /// TWI Enable
                TWEN = 2, RW;
                /// TWI Write Collision Flag
                TWWC = 3, R;
                /// TWI Stop Condition
                TWSTO = 4, RW;
                /// TWI Start Condition
                TWSTA = 5, RW;
                /// TWI Enable Acknowledge
                TWEA = 6, RW;
                /// TWI Interrupt Flag
                TWINT = 7, RW;
            }
        }
    }

    reg! {
        /// TWI Slave Address Mask Register
        TWAMR: u8 {
            addr: 0xBD,
            write mask: 0xFE,
        }
    }
}
use registers::*;

/// A simple FIFO buffer for storing the data to be sent over TWI.
struct Buffer {
    idx: u8,
    len: u8,
    buf: [u8; BUFFER_LEN],
}

impl Buffer {
    const fn new() -> Self {
        Self {
            idx: 0,
            len: 0,
            buf: [0; BUFFER_LEN],
        }
    }

    /// Replaces the contents of the buffer with the input data.
    fn set<T: ByteBundle + ?Sized> (&mut self, data: &T) -> Result<(), TWIError> {
        if BUFFER_LEN < data.length() {
            Err(TWIError::BufferLenError)
        } else {
            unsafe {
                // SAFETY: data.length() must never be greater than BUFFER_LEN.
                for i in 0..data.length() {
                    *self.buf.get_unchecked_mut(i) = data.get(i);
                }
                self.len = data.length() as u8;
                self.idx = 0;
            }
            Ok(())
        }
    }

    /// Pops a single value off the front of the buffer.
    fn pop(&mut self) -> Option<u8> {
        if self.idx == self.len {
            None
        } else {
            // SAFETY: self.idx and self.len must never be greater than BUFFER_LEN.
            let val = unsafe { *self.buf.get_unchecked(self.idx as usize) };
            self.idx += 1;
            Some(val)
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum TWIState {
    None,
    Ready,
    Transmitting,
}

// An error as returned to a user.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TWIError {
    BufferLenError,
    SendAddressNACK,
    SendDataNACK,
    NotReady,
    BusError,

    InitError,
    InvalidAddress,
}

/// This type used to store the global data for communication between the interrupt and normal code.
#[repr(C)]
struct TWIGlobalData {
    state: TWIState,
    address: u8,
    error: TWSRStatus,

    buffer: Buffer,
}

/// Mmm.... boilerplate...
///
/// These functions are simple wrappers to make volatile reading and writing less messy.
impl TWIGlobalData {
    unsafe fn state(&mut self) -> TWIState {
        (&mut self.state as *mut TWIState).read_volatile()
    }

    unsafe fn set_state(&mut self, new: TWIState) {
        (&mut self.state as *mut TWIState).write_volatile(new);
    }

    unsafe fn error(&mut self) -> TWSRStatus {
        (&mut self.error as *mut TWSRStatus).read_volatile()
    }

    unsafe fn set_error(&mut self, new: TWSRStatus) {
        (&mut self.error as *mut TWSRStatus).write_volatile(new);
    }
}

/// This is where we store the data shared between the interrupt and non-interrupt code.
static mut TWI_GLOBAL: TWIGlobalData = TWIGlobalData {
    state: TWIState::None,
    address: 0,
    error: TWSRStatus::NoInfo,
    buffer: Buffer::new(),
};

/// Tracks whether the TWI has been initialized so only one TWI live at a time.
static mut HAS_INIT: bool = false;

/// A wrapper around the TWI module.
///
/// Provides a simplified interface over the TWI module, which only allows the transmission
/// of data as a master on the TWI bus.
///
/// Because we're representing a hardware module, we should ensure that only one of these
/// exists at any one time.
pub struct TWI(core::marker::PhantomData<()>);

impl TWI {
    /// Initializes the TWI module with a 1 prescale and 400KHz transmission rate.
    /// Sets the SDA and SCL pins to input, and enables the internal pullups.
    /// Enables the ACK pulse, and TWI interrupt.
    pub fn init() -> Result<TWI, TWIError> {
        unsafe {
            if HAS_INIT {
                Err(TWIError::InitError)
            } else {
                HAS_INIT = true;
                TWI_GLOBAL.set_state(TWIState::Ready);

                // Set the SDA and SCL pins to input, and enable the internal pullups.
                DDRC::clear_bits(DDRC::DDRC4 | DDRC::DDRC5);
                PORTC::set_bits(PORTC::PORTC4 | PORTC::PORTC5);

                // Initiliazing the prescaler to 1, and bit rate for a 400KHz transmission rate.
                TWSR::clear_bits(TWSR::TWPS0 | TWSR::TWPS1);
                TWBR::set_raw_value(TWI_BIT_RATE);

                // Enable the TWI module, and the ACK.
                TWCR::set_value(TWCR::TWEN | TWCR::TWEA);

                Ok(TWI(core::marker::PhantomData))
            }
        }
    }

    pub fn set_address(&mut self, addr: u8) -> Result<(), TWIError> {
        if addr > 127 {
            Err(TWIError::InvalidAddress)
        } else {
            unsafe {
                TWI_GLOBAL.address = addr << 1;
            }
            Ok(())
        }
    }

    /// Attempts to become TWI master and write a series of bytes to a device on the bus.
    ///
    /// Waits for the transmission to finish before returning.
    pub fn write<T: ByteBundle + ?Sized>(&mut self, data: &T) -> Result<(), TWIError> {
        // SAFETY: Assumes that there is only one instance of TWI.
        unsafe { 
            // In our limited implementation, the bus should be in the Ready state when we get here.
            if TWI_GLOBAL.state() != TWIState::Ready {
                return Err(TWIError::NotReady);
            }

            TWI_GLOBAL.buffer.set(data)?;

            TWI_GLOBAL.set_state(TWIState::Transmitting);
            TWI_GLOBAL.set_error(TWSRStatus::NoInfo);

            // Enable the interrupt and Start signal.
            TWCR::set_value(TWCR::TWINT | TWCR::TWEA | TWCR::TWEN | TWCR::TWIE | TWCR::TWSTA);

            // Wait for the write operation to complete.
            // Because an interrupt will be changing the state, we need to do a volatile read
            // otherwise the optimizer will helpfully decide that state can't possible change
            // between each check as we don't change it here, and optimize us into an infinite
            // loop.
            // This is undesireable.
            while TWI_GLOBAL.state() == TWIState::Transmitting {
            }

            match TWI_GLOBAL.error() {
                TWSRStatus::NoInfo => Ok(()),
                TWSRStatus::MtDataNack => Err(TWIError::SendDataNACK),
                TWSRStatus::MtSlaNack => Err(TWIError::SendAddressNACK),
                _ => Err(TWIError::BusError),
            }
        }
    }
}

impl Drop for TWI {
    fn drop(&mut self) {
        // SAFETY: Assumes only one TWI instance exists.
        unsafe {
            HAS_INIT = false;
            // Disable TWI, turn off interrupt, turn off ACK.
            TWCR::clear_bits(TWCR::TWEN | TWCR::TWIE | TWCR::TWEA);

            // Disable the internal pullups for the SDA and SCL pins.
            PORTC::clear_bits(PORTC::PORTC4 | PORTC::PORTC5);
        }
    }
}

unsafe fn send_reply(ack: bool) {
    let mut bits = TWCR::TWEN | TWCR::TWIE | TWCR::TWINT;
    if ack {
        bits |= TWCR::TWEA;
    }

    TWCR::set_value(bits);
}

unsafe fn stop() {
    TWCR::set_value(TWCR::TWEN | TWCR::TWEA | TWCR::TWINT | TWCR::TWSTO);

    // Wait for stop condition to be executed on bus.
    // TWINT is not set after a stop condition!
    while TWCR::get_bit(TWCR::TWSTO) {
        continue;
    }

    TWI_GLOBAL.set_state(TWIState::Ready);
}

unsafe fn release_bus() {
    TWCR::set_value(TWCR::TWEN | TWCR::TWEA | TWCR::TWINT);

    TWI_GLOBAL.set_state(TWIState::Ready);
}

/// TWI interrupt handler.
#[no_mangle]
pub unsafe extern "avr-interrupt" fn __vector_24() {
    match TWSR::status() {
        /////////////////////
        // All Master
        /////////////////////

          TWSRStatus::Start     // Sent start condition.
        | TWSRStatus::RepStart  // Sent repeated start condition.
        => {
            // Copy device address and R/W bit to output register and ACK.
            // In this case we know that we're always writing, which is a 0 in the R/W bit.
            TWDR::set_raw_value(TWI_GLOBAL.address);
            send_reply(true);
        },

        /////////////////////
        // Master transmitter
        /////////////////////

          TWSRStatus::MtSlaAck  // Slave receiver ACKed address.
        | TWSRStatus::MtDataAck // Slave receiever ACKed data.
        => {
            // If there is data to send, send it, otherwise stop.
            if let Some(byte) = TWI_GLOBAL.buffer.pop() {
                TWDR::set_raw_value(byte);
                send_reply(true);
            } else {
                stop();
            }
        },

        // Address Sent, NACK received.
        TWSRStatus::MtSlaNack => {
            TWI_GLOBAL.set_error(TWSRStatus::MtSlaNack);
            stop();
        },
        // Data Sent, NACK received.
        TWSRStatus::MtDataNack => {
            TWI_GLOBAL.set_error(TWSRStatus::MtDataNack);
            stop();
        },
        // Lost bus arbitration.
        TWSRStatus::MtArbLost => {
            TWI_GLOBAL.set_error(TWSRStatus::MtArbLost);
            release_bus();
        },

        /////////////////////
        // All
        /////////////////////

        // No state information.
        TWSRStatus::NoInfo => {},
        // Bus error, illegal stop/start
        TWSRStatus::BusError => {
            TWI_GLOBAL.set_error(TWSRStatus::BusError);
            stop();
        }

        /////////////////////
        // Acting as a master receiver or a slave device
        // is not implemented. Just NACK everything.
        /////////////////////

        _ => {
            send_reply(false);
        }
    }
}