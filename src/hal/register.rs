use core::{
    marker::PhantomData,
    ops::{BitAnd, BitOr, BitOrAssign, Not, Shl},
};

/// Used to restrict what data types a register can be.
///
/// The registers on the 328P are only 8- or 16-bit, so it will only be implement
/// for `u8` and `u16`.
pub trait RegisterType:
    Copy
    + BitAnd<Output = Self>
    + BitOr<Output = Self>
    + Shl<Output = Self>
    + Not<Output = Self>
    + Eq
    + PartialEq
{
    const ZERO: Self;
    const ONE: Self;
}

impl RegisterType for u8 {
    const ZERO: Self = 0;
    const ONE: Self = 1;
}

impl RegisterType for u16 {
    const ZERO: Self = 0;
    const ONE: Self = 1;
}

/// This trait and types represent whether a bit is readable and/or writable in the type system.
pub trait Access {}
pub struct Readable;
impl Access for Readable {}
pub struct NotReadable;
impl Access for NotReadable {}

pub struct Writable;
impl Access for Writable {}
pub struct NotWritable;
impl Access for NotWritable {}

/// This allows us to say that access is only allowed if both bits have that access.
/// It is, essentially, a boolean AND operation, hence the name.
pub trait AccessAnd<Rhs> {
    type Output: Access;
}

impl AccessAnd<Readable> for Readable {
    type Output = Readable;
}
impl AccessAnd<NotReadable> for Readable {
    type Output = NotReadable;
}
impl AccessAnd<NotReadable> for NotReadable {
    type Output = NotReadable;
}
impl AccessAnd<Readable> for NotReadable {
    type Output = NotReadable;
}

impl AccessAnd<Writable> for Writable {
    type Output = Writable;
}
impl AccessAnd<NotWritable> for Writable {
    type Output = NotWritable;
}
impl AccessAnd<NotWritable> for NotWritable {
    type Output = NotWritable;
}
impl AccessAnd<Writable> for NotWritable {
    type Output = NotWritable;
}

/// Represents a bit in a register.
///
/// A bit needs to be associated with its parent register, and what read/write access the bit has, so incorrect usage
/// is prevented at compile-time.
pub trait Bit {
    type ReadAccess: Access;
    type WriteAccess: Access;
    type Register: Register;
    fn bit_id(&self) -> <Self::Register as Register>::DataType;
}

/// The BitBuilder allows the user to write expressions like `Bit1 | Bit2` to build up a bit pattern, while
/// still retaining the connection to the register and the restrictions on read/write access.
pub struct BitBuilder<Reg: Register, R: Access, W: Access> {
    data: Reg::DataType,
    _p: PhantomData<(Reg, R, W)>,
}

impl<Reg: Register> BitBuilder<Reg, Readable, Writable> {
    /// The default should be to have full read/write access. Any OR operations will restrict it as needed.
    pub fn new() -> Self {
        Self {
            data: Reg::DataType::ZERO,
            _p: PhantomData,
        }
    }
}

impl<Reg: Register, R: Access, W: Access> BitBuilder<Reg, R, W> {
    pub fn raw_value(&self) -> Reg::DataType {
        self.data
    }
}

/// This one's pretty gnarly. Basically, this lets the user do `bits | bits`, while continuing the connection
/// with the register. The resulting BitBuilder will have the most restrictive read/write access.
/// E.g. A BitBuilder with read/write access being ORed with one with only read access will result in a BitBuilder
/// with only read access.
impl<Reg, R1, W1, R2, W2> BitOr<BitBuilder<Reg, R2, W2>> for BitBuilder<Reg, R1, W1>
where
    R1: Access,
    R2: Access,
    W1: Access,
    W2: Access,
    Reg: Register,
    R2: AccessAnd<R1>,
    W2: AccessAnd<W1>,
{
    type Output = BitBuilder<Reg, <R2 as AccessAnd<R1>>::Output, <W2 as AccessAnd<W1>>::Output>;

    fn bitor(self, rhs: BitBuilder<Reg, R2, W2>) -> Self::Output {
        BitBuilder {
            data: self.data | rhs.data,
            _p: PhantomData,
        }
    }
}

/// This just lets us OR a BitBuilder with a Bit from the same register. As with above, the most restrictive access
/// applies.
impl<Reg, R1, W1, B> BitOr<B> for BitBuilder<Reg, R1, W1>
where
    R1: Access,
    W1: Access,
    Reg: Register,
    B: Bit<Register = Reg>,
    B::ReadAccess: AccessAnd<R1>,
    B::WriteAccess: AccessAnd<W1>,
{
    type Output = BitBuilder<
        Reg,
        <B::ReadAccess as AccessAnd<R1>>::Output,
        <B::WriteAccess as AccessAnd<W1>>::Output,
    >;
    // Shut up, clippy, I know it's complex!

    fn bitor(self, rhs: B) -> Self::Output {
        BitBuilder {
            data: self.data | (Reg::DataType::ONE << rhs.bit_id()),
            _p: PhantomData,
        }
    }
}

/// This lets us do things like:
///
/// ```
/// let mut bits = TWEN | TWIE | TWINT;
/// if ack {
///     bits |= TWEA;
/// }
/// TWCR::set_value(bits);
/// ```
///
/// The restriction is that both read and write access of the new bit must exactly match the access of the BitBuilder
/// as it can't alter the return type.
impl<Reg, R1, W1, B> BitOrAssign<B> for BitBuilder<Reg, R1, W1>
where
    R1: Access,
    W1: Access,
    Reg: Register,
    B: Bit<Register = Reg, ReadAccess = R1, WriteAccess = W1>,
{
    fn bitor_assign(&mut self, rhs: B) {
        self.data = self.data | (Reg::DataType::ONE << rhs.bit_id());
    }
}

/// This trait is to fix an irritating papercut.
///
/// If we don't have it and have the register functions take a BitBuilder, it would mean that we could do this:
///
/// ```
/// TWCR::set_value(TWEN | TWIE);
/// ```
///
/// But not this:
///
/// ```
/// TWCR::set_value(TWEN);
/// ```
///
/// Which feels incredibly inconsistent.
pub trait SetValueType {
    type Register: Register;
    type WriteAccess: Access;

    fn value(&self) -> <Self::Register as Register>::DataType;
}

impl<Reg, R, W> SetValueType for BitBuilder<Reg, R, W>
where
    Reg: Register,
    R: Access,
    W: Access,
{
    type Register = Reg;
    type WriteAccess = W;

    fn value(&self) -> Reg::DataType {
        self.data
    }
}

/// Abstracts away the messy volatile pointer accesses and bit-twiddling needed for the MMIO
/// register.
///
/// The type needs to be tracked, as does a write mask, as some bits should never be written to.
///
/// One thing that does need to be provided is the ability to set a raw value, as some registers
/// are data registers, not control/status registers. Bit-twiddling these makes no sense.
pub trait Register: Sized {
    type DataType: RegisterType;
    const ADDR: *mut Self::DataType;

    // Some bits should always be written 0 when writing, this allows that.
    const WRITE_MASK: Self::DataType;

    unsafe fn set_raw_value(val: Self::DataType) {
        Self::ADDR.write_volatile(val & Self::WRITE_MASK);
    }

    unsafe fn set_value<V>(val: V)
    where
        V: SetValueType<Register = Self, WriteAccess = Writable>,
    {
        let value = val.value();
        Self::set_raw_value(value);
    }

    unsafe fn get_value() -> Self::DataType {
        Self::ADDR.read_volatile()
    }

    unsafe fn get_bit<B>(bit: B) -> bool
    where
        B: Bit<Register = Self, ReadAccess = Readable>,
    {
        let bit = Self::DataType::ONE << bit.bit_id();

        (Self::get_value() & bit) != Self::DataType::ZERO
    }

    unsafe fn set_bits<V>(bits: V)
    where
        V: SetValueType<Register = Self, WriteAccess = Writable>,
    {
        let val = Self::get_value();
        Self::set_raw_value(val | bits.value());
    }

    unsafe fn clear_bits<V>(bits: V)
    where
        V: SetValueType<Register = Self, WriteAccess = Writable>,
    {
        let val = Self::get_value();
        Self::set_raw_value(val & !bits.value());
    }

    unsafe fn replace_bits(
        mask: BitBuilder<Self, Readable, Writable>,
        new_val: BitBuilder<Self, Readable, Writable>,
    ) {
        let reg_val = Self::get_value() & !mask.data;
        let masked_val = new_val.data & mask.data;
        Self::set_raw_value(reg_val | masked_val);
    }
}

// Unfortunately, all the traits above make actually defining a register and its bits something that no sane
// person should do. Fortunately, because the definitions are very regular, macros can be used.

/// Used in the `reg_named_bits` macro for expanding the read access declarations. It allows the other macros
/// to just match the access flags as a token tree, and have this expand it.
macro_rules! expand_read_access {
    (R) => {
        type ReadAccess = crate::hal::register::Readable;
    };
    (W) => {
        type ReadAccess = crate::hal::register::NotReadable;
    };
    (RW) => {
        type ReadAccess = crate::hal::register::Readable;
    };
}

/// Used in the `reg_named_bits` macro for expanding the write access declarations. It allows the other macros
/// to just match the access flags as a token tree, and have this expand it.
macro_rules! expand_write_access {
    (R) => {
        type WriteAccess = crate::hal::register::NotWritable;
    };
    (W) => {
        type WriteAccess = crate::hal::register::Writable;
    };
    (RW) => {
        type WriteAccess = crate::hal::register::Writable;
    };
}

/// Because each bit is represented as a unique type, actually declaring them by hand is a tedious nightmare. This
/// macro allows the user to declare a whole register's bits using an enum-like representation.
#[macro_export]
macro_rules! reg_named_bits {
    (
        $reg:ident : $type:ty {
            $( $(#[$bit_doc:meta])* $bit:ident = $id:expr, $acc:tt;)+
        }
    ) => {
        $(
            $(#[$bit_doc])*
            #[derive(Copy, Clone)]
            pub struct $bit;
            impl crate::hal::register::Bit for $bit {
                type Register = $reg;
                expand_read_access!{$acc}
                expand_write_access!{$acc}
                fn bit_id(&self) -> $type {
                    $id
                }
            }

            impl crate::hal::register::SetValueType for $bit {
                type Register = $reg;
                expand_write_access!{$acc}

                fn value(&self) -> $type {
                    use crate::hal::register::Bit;
                    1 << self.bit_id()
                }
            }

            // Oh god... what have I written here?!
            impl<B2> core::ops::BitOr<B2> for $bit
                where Self: crate::hal::register::Bit,
                    B2: crate::hal::register::Bit<Register = <Self as crate::hal::register::Bit>::Register>,
                    B2::ReadAccess: crate::hal::register::AccessAnd<<Self as crate::hal::register::Bit>::ReadAccess>,
                    B2::WriteAccess: crate::hal::register::AccessAnd<<Self as crate::hal::register::Bit>::WriteAccess>,
            {
                type Output = crate::hal::register::BitBuilder<
                    <Self as crate::hal::register::Bit>::Register,
                    <B2::ReadAccess as crate::hal::register::AccessAnd<<Self as crate::hal::register::Bit>::ReadAccess>>::Output,
                    <B2::WriteAccess as crate::hal::register::AccessAnd<<Self as crate::hal::register::Bit>::WriteAccess>>::Output,
                >;

                fn bitor(self, rhs: B2) -> Self::Output {
                    crate::hal::register::BitBuilder::new() | self | rhs
                }
            }
        )*
    };
}

/// There are a lot of bits, with rather opaque names. This macro defines constants on the parent register that
/// allow the user to find the bit through the register.
#[macro_export]
macro_rules! reg_bit_consts {
    (
        $struct_name:ident {
            $( $(#[$bit_doc:meta])* $bit:ident ),+ $(,)*
        }
    ) => {
        impl $struct_name {
            $(
                $(#[$bit_doc])*
                #[allow(dead_code)]
                pub const $bit: $bit = $bit;
            )*
        }
    }
}

/// Declaring a register and its bits is a verbose, tedious process. This macro provides a way to declare them
/// using a structure similar to declaring a struct in Rust.
#[macro_export]
macro_rules! reg {
    (
        $(#[$reg_doc:meta])*
        $name:ident : $type:ty {
            addr: $addr:expr,
            write mask: $mask:expr $(,)*
        }
    ) => {
        $(#[$reg_doc])*
        #[allow(dead_code)]
        pub struct $name;
        impl crate::hal::register::Register for $name {
            type DataType = $type;
            const WRITE_MASK: $type = $mask;
            const ADDR: *mut $type = $addr as *mut $type;
        }
    };

    (
        $(#[$reg_doc:meta])*
        $name:ident: $type:ty {
            addr: $addr:expr,
            write mask: $mask:expr,
            bits: {
                $( $(#[$bit_doc:meta])* $bit:ident = $id:expr, $acc:tt;)+
            }
        }
    ) => {
        reg_named_bits! {
            $name: $type {
                $( $(#[$bit_doc])* $bit = $id, $acc; )+
            }
        }

        reg! {
            $(#[$reg_doc])*
            $name: $type {
                addr: $addr,
                write mask: $mask,
            }
        }

        reg_bit_consts! {
            $name {
                $( $(#[$bit_doc])* $bit ),+
            }
        }
    };
}
