//! A simple abstraction over the basic port IO.
//!
//! Allows the user to handle the three registers on each port as a single unit,
//! as well as providing more meaningful names to the operations.
//!
//! Each port structure has associated constants for each of the pins on that pin
//! for ease of use.

#![allow(dead_code)]
use crate::hal::register::Register;

pub mod registers {
    reg! {
        /// Port B Data Register
        PORTB: u8 {
            addr: 0x25,
            write mask: 0xFF,
            bits: {
                PORTB0 = 0, RW;
                PORTB1 = 1, RW;
                PORTB2 = 2, RW;
                PORTB3 = 3, RW;
                PORTB4 = 4, RW;
                PORTB5 = 5, RW;
                PORTB6 = 6, RW;
                PORTB7 = 7, RW;
            }
        }
    }
    reg! {
        /// Port B Data Direction Register
        DDRB: u8 {
            addr: 0x24,
            write mask: 0xFF,
            bits: {
                DDB0 = 0, RW;
                DDB1 = 1, RW;
                DDB2 = 2, RW;
                DDB3 = 3, RW;
                DDB4 = 4, RW;
                DDB5 = 5, RW;
                DDB6 = 6, RW;
                DDB7 = 7, RW;
            }
        }
    }
    reg! {
        /// Port B Input Pins Register
        PINB: u8 {
            addr: 0x23,
            write mask: 0xFF,
            bits: {
                PINB0 = 0, RW;
                PINB1 = 1, RW;
                PINB2 = 2, RW;
                PINB3 = 3, RW;
                PINB4 = 4, RW;
                PINB5 = 5, RW;
                PINB6 = 6, RW;
                PINB7 = 7, RW;
            }
        }
    }

    reg! {
        /// Port C Data Register
        PORTC: u8 {
            addr: 0x28,
            write mask: 0x7F,
            bits: {
                PORTC0 = 0, RW;
                PORTC1 = 1, RW;
                PORTC2 = 2, RW;
                PORTC3 = 3, RW;
                PORTC4 = 4, RW;
                PORTC5 = 5, RW;
                PORTC6 = 6, RW;
            }
        }
    }
    reg! {
        /// Port C Data Direction Register
        DDRC: u8 {
            addr: 0x27,
            write mask: 0x7F,
            bits: {
                DDRC0 = 0, RW;
                DDRC1 = 1, RW;
                DDRC2 = 2, RW;
                DDRC3 = 3, RW;
                DDRC4 = 4, RW;
                DDRC5 = 5, RW;
                DDRC6 = 6, RW;
            }
        }
    }
    reg! {
        /// Port C Input Pins Register
        PINC: u8 {
            addr: 0x26,
            write mask: 0x7F,
            bits: {
                PINC0 = 0, RW;
                PINC1 = 1, RW;
                PINC2 = 2, RW;
                PINC3 = 3, RW;
                PINC4 = 4, RW;
                PINC5 = 5, RW;
                PINC6 = 6, RW;
            }
        }
    }

    reg! {
        /// Port D Data Register
        PORTD:  u8 {
            addr: 0x2B,
            write mask: 0xFF,
            bits: {
                PORTD0 = 0, RW;
                PORTD1 = 1, RW;
                PORTD2 = 2, RW;
                PORTD3 = 3, RW;
                PORTD4 = 4, RW;
                PORTD5 = 5, RW;
                PORTD6 = 6, RW;
                PORTD7 = 7, RW;
            }
        }
    }
    reg! {
        /// Port D Data Direction Register
        DDRD:   u8 {
            addr: 0x2A,
            write mask: 0xFF,
            bits: {
                DDD0 = 0, RW;
                DDD1 = 1, RW;
                DDD2 = 2, RW;
                DDD3 = 3, RW;
                DDD4 = 4, RW;
                DDD5 = 5, RW;
                DDD6 = 6, RW;
                DDD7 = 7, RW;
            }
        }
    }
    reg! {
        /// Port D Input Pins Register
        PIND:   u8 {
            addr: 0x29,
            write mask: 0xFF,
            bits: {
                PIND0 = 0, RW;
                PIND1 = 1, RW;
                PIND2 = 2, RW;
                PIND3 = 3, RW;
                PIND4 = 4, RW;
                PIND5 = 5, RW;
                PIND6 = 6, RW;
                PIND7 = 7, RW;
            }
        }
    }
}
use registers::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PinMode {
    Output,
    Input,
    InputPullup,
}

// Unfortunately, the bit abstraction doesn't really work well here, because I want
// to represent each port as a single module, instead of three separate registers. The
// abstraction for the registers requires that each bit be associated with its parent
// register, meaning I can't do that.

// Because the bits on each of the three registers have exactly the same meaning, and
// are all read-write, I'm representing them with an enum instead.
pub trait PortBit {
    fn bit(&self) -> u8;
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PortBPins {
    PB0,
    PB1,
    PB2,
    PB3,
    PB4,
    PB5,
    PB6,
    PB7,
}

impl PortBit for PortBPins {
    fn bit(&self) -> u8 {
        use PortBPins::*;
        match self {
            PB0 => 0,
            PB1 => 1,
            PB2 => 2,
            PB3 => 3,
            PB4 => 4,
            PB5 => 5,
            PB6 => 6,
            PB7 => 7,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PortCPins {
    PC0,
    PC1,
    PC2,
    PC3,
    PC4,
    PC5,
    PC6,
}

impl PortBit for PortCPins {
    fn bit(&self) -> u8 {
        use PortCPins::*;
        match self {
            PC0 => 0,
            PC1 => 1,
            PC2 => 2,
            PC3 => 3,
            PC4 => 4,
            PC5 => 5,
            PC6 => 6,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PortDPins {
    PD0,
    PD1,
    PD2,
    PD3,
    PD4,
    PD5,
    PD6,
    PD7,
}

impl PortBit for PortDPins {
    fn bit(&self) -> u8 {
        use PortDPins::*;
        match self {
            PD0 => 0,
            PD1 => 1,
            PD2 => 2,
            PD3 => 3,
            PD4 => 4,
            PD5 => 5,
            PD6 => 6,
            PD7 => 7,
        }
    }
}

// Unfortunately, the utility functions on the Register trait expect the bits defined
// on for the register, and won't work for the shared bits above.
// We need to re-implement the bit-twiddling of a single bit.
unsafe fn set_reg_bit<R: Register<DataType = u8>>(bit: u8) {
    let mut val = R::get_value();
    val |= 1 << bit;
    R::set_raw_value(val);
}

unsafe fn clear_reg_bit<R: Register<DataType = u8>>(bit: u8) {
    let mut val = R::get_value();
    val &= !(1 << bit);
    R::set_raw_value(val);
}

unsafe fn get_reg_bit<R: Register<DataType = u8>>(bit: u8) -> bool {
    (R::get_value() & (1 << bit)) != 0
}

/// Shared behaviour on the three ports available to the Atmega328P.
pub trait Port {
    type ValidPins: PortBit + Copy;
    type PORT: Register<DataType = u8>;
    type DDR: Register<DataType = u8>;
    type PIN: Register<DataType = u8>;

    /// Sets the DDR and PORT registers to the appropriate values for the given pin mode.
    ///
    /// Does not alter the PORT value when set to Output.
    fn set_pin_mode(pin: Self::ValidPins, mode: PinMode) {
        unsafe {
            let bit = pin.bit();
            match mode {
                PinMode::Output => {
                    set_reg_bit::<Self::DDR>(bit);
                }
                PinMode::Input => {
                    set_reg_bit::<Self::DDR>(bit);
                    clear_reg_bit::<Self::PORT>(bit);
                }
                PinMode::InputPullup => {
                    set_reg_bit::<Self::DDR>(pin.bit());
                    set_reg_bit::<Self::PORT>(bit);
                }
            }
        }
    }

    /// Sets the appropriate bit in the PORT register to high. In Output mode, will drive the pin high;
    /// in Input mode will enable the internal pullup resistor.
    fn set_port_high(pin: Self::ValidPins) {
        unsafe {
            set_reg_bit::<Self::PORT>(pin.bit());
        }
    }

    /// Sets the appropriate bit in the PORT register to low. In Output mode, will pull the pin low;
    /// in Input mode will disable the internal pullup resistor.
    fn set_port_low(pin: Self::ValidPins) {
        unsafe {
            clear_reg_bit::<Self::PORT>(pin.bit());
        }
    }

    /// Sets the given bit in the PIN register, toggling the current state of the PORT register.
    fn set_pin_toggle(pin: Self::ValidPins) {
        unsafe {
            set_reg_bit::<Self::PIN>(pin.bit());
        }
    }

    /// Reads from the PIN register, returning the current state of the pin.
    fn get_pin_state(pin: Self::ValidPins) -> bool {
        unsafe { get_reg_bit::<Self::PIN>(pin.bit()) }
    }
}

pub struct PortB;
impl Port for PortB {
    type PORT = PORTB;
    type PIN = PINB;
    type DDR = DDRB;
    type ValidPins = PortBPins;
}

impl PortB {
    pub const PB0: PortBPins = PortBPins::PB0;
    pub const PB1: PortBPins = PortBPins::PB1;
    pub const PB2: PortBPins = PortBPins::PB2;
    pub const PB3: PortBPins = PortBPins::PB3;
    pub const PB4: PortBPins = PortBPins::PB4;
    pub const PB5: PortBPins = PortBPins::PB5;
    pub const PB6: PortBPins = PortBPins::PB6;
    pub const PB7: PortBPins = PortBPins::PB7;
}

pub struct PortC;
impl Port for PortC {
    type PORT = PORTC;
    type PIN = PINC;
    type DDR = DDRC;
    type ValidPins = PortCPins;
}

impl PortC {
    pub const PC0: PortCPins = PortCPins::PC0;
    pub const PC1: PortCPins = PortCPins::PC1;
    pub const PC2: PortCPins = PortCPins::PC2;
    pub const PC3: PortCPins = PortCPins::PC3;
    pub const PC4: PortCPins = PortCPins::PC4;
    pub const PC5: PortCPins = PortCPins::PC5;
    pub const PC6: PortCPins = PortCPins::PC6;
}

pub struct PortD;
impl Port for PortD {
    type PORT = PORTD;
    type PIN = PIND;
    type DDR = DDRD;
    type ValidPins = PortDPins;
}

impl PortD {
    pub const PD0: PortDPins = PortDPins::PD0;
    pub const PD1: PortDPins = PortDPins::PD1;
    pub const PD2: PortDPins = PortDPins::PD2;
    pub const PD3: PortDPins = PortDPins::PD3;
    pub const PD4: PortDPins = PortDPins::PD4;
    pub const PD5: PortDPins = PortDPins::PD5;
    pub const PD6: PortDPins = PortDPins::PD6;
    pub const PD7: PortDPins = PortDPins::PD7;
}
