//! A basic way of accessing bytes stored in PROGMEM.
//!
//! The image data sent to the screen is too large to sit in RAM, so it needs to be stored in
//! the program memory, which is in a completely separate memory space and requires a different
//! instruction to read from (`LPM`) than reading from RAM (`LD`).
//! Unfortunately, the compiler is dumb, and doesn't know that different memory spaces exist
//! and will just emit the `LD` instruction when you try to read from PROGMEM data the usual
//! way.
//!
//! Because of that, we need to provide a different representation so we don't need to scatter
//! inline assembly all over the place whenever we want to read PROGMEM. I also want to have
//! the option to use a chunk of bytes from RAM in the same places, so I needed a trait
//! to represent the shared behaviour.
//!
//! I cannot use the Index trait from corelib here, because Index requires the return type
//! be a reference, not the data itself.

pub trait ByteBundle {
    fn get(&self, idx: usize) -> u8;
    fn length(&self) -> usize;
}

impl ByteBundle for [u8] {
    fn get(&self, idx: usize) -> u8 {
        self[idx]
    }

    fn length(&self) -> usize {
        self.len()
    }
}

/// Represents a slice of bytes stored in the PROGMEM memory space.
///
/// Runtime representation should be basically the same as a slice. I chose not to include
/// a lifetime because any data used with this will be in PROGMEM, and therefore always 
/// 'static.
#[derive(Copy, Clone)]
pub struct PGMSlice {
    addr: *const u8,
    len: usize,
}

impl PGMSlice {
    // SAFETY: The input pointer MUST be pointing at PROGMEM, not RAM.
    pub const unsafe fn from_raw_parts(addr: *const u8, len: usize) -> Self {
        Self {
            addr,
            len,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn chunks(&self, len: usize) -> PGMChunks {
        PGMChunks {
            slice: *self,
            chunk_len: len
        }
    }
}

impl ByteBundle for PGMSlice {
    fn get(&self, idx: usize) -> u8 {
        if idx >= self.len {
            panic!("Index out of bounds: {}", idx);
        }

        let idx = idx as isize;

        unsafe {
            let addr = self.addr.offset(idx) as usize;
            let out;

            llvm_asm!{
                "lpm $0, Z"
                : "=r"(out)
                : "{r31}"(addr >> 8),"{r30}"(addr)
                : "r31", "r30"
                : "volatile"
            }

            out
        }
    }

    fn length(&self) -> usize {
        self.len()
    }
}

/// I want to send an entire screen's worth of pixel data (1kb) at the screen, but my TWI buffer is
/// only 32 bytes long. Being able to split the PGMSlice into chunks like you can with a slice so I
/// can use the usual iterator stuff would make that a lot easier.
pub struct PGMChunks {
    slice: PGMSlice,
    chunk_len: usize,
}

impl Iterator for PGMChunks {
    type Item = PGMSlice;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.len == 0 {
            None
        } else if self.slice.len < self.chunk_len {
            let ret = self.slice;
            self.slice.len = 0; // No need to faff around with the pointer here.
            Some(ret)
        } else {
            let mut ret_slice = self.slice;
            ret_slice.len = self.chunk_len;

            // We need to adjust the pointer of our slice, as well as its length, to account
            // for the chunk we just removed.
            unsafe {
                let offset = self.chunk_len as isize;
                
                self.slice.addr = self.slice.addr.offset(offset);
                self.slice.len -= self.chunk_len;
            }

            Some(ret_slice)
        }
    }
}