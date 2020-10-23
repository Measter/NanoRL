//! A simple random number generator using a Galois Linear Feedback Shift Register.

pub struct Rng(u16);

impl Rng {
    pub fn new(seed: u16) -> Rng {
        Rng(seed)
    }

    pub fn next(&mut self) -> u8 {
        let lsb = self.0 & 0x1;
        self.0 >>= 1;
        if lsb != 0 {
            self.0 ^= 0xB400;
        }

        (self.0 >> 8) as u8
    }

    pub fn next_range(&mut self, min: u8, max: u8) -> u8 {
        let range = max - min;

        if range == 0 {
            min
        } else {
            (self.next() % range) + min
        }
    }
}