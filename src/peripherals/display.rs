use crate::hal::{
    twi,
    progmem::{ByteBundle, PGMSlice},
};
use core::marker::PhantomData;

pub const WIDTH: u8 = 128;
pub const HEIGHT: u8 = 64;

const SSD1306_COMMAND: u8 = 0x00;
const SSD1306_DATA: u8 = 0x40;

const SSD1306_MEMORYMODE: u8 = 0x20;
const SSD1306_COLUMNADDR: u8 = 0x21;
const SSD1306_PAGEADDR: u8 = 0x22;
const SSD1306_CHARGEPUMP: u8 = 0x8D;
const SSD1306_SEGREMAP: u8 = 0xA0;
const SSD1306_DISPLAYOFF: u8 = 0xAE;
const SSD1306_DISPLAYON: u8 = 0xAF;
const SSD1306_COMSCANDEC: u8 = 0xC8;
const SSD1306_SETPRECHARGE: u8 = 0xD9;
const SSD1306_SETVCOMDETECT: u8 = 0xDB;

pub struct Display(PhantomData<()>);

impl Display {
    pub fn init(twi: &mut twi::TWI) -> Result<Display, twi::TWIError> {
        // The initialization sequence for the SSD1306 driver.
        let init = [
            SSD1306_COMMAND,
            SSD1306_DISPLAYOFF,

            SSD1306_CHARGEPUMP,
            0x14,

            SSD1306_MEMORYMODE,
            0x00, // Horizontal addressing.
            SSD1306_SEGREMAP | 0x1, // Left-to-right mapping
            SSD1306_COMSCANDEC,
            SSD1306_COMMAND,
            SSD1306_SETPRECHARGE,
            0xF1,

            SSD1306_SETVCOMDETECT,
            0x40,
            SSD1306_DISPLAYON,
        ];
        twi.write(init.as_ref())?;

        Ok(Self(PhantomData))
    }

    pub fn clear_display(&mut self, twi: &mut twi::TWI) -> Result<(), twi::TWIError> {
        let commands = [
            SSD1306_COMMAND,
            SSD1306_PAGEADDR,
            0x00,   // Page start address
            0x07,   // Page end
            SSD1306_COLUMNADDR,
            0x00,   // Column start address
            WIDTH - 1,
        ];
            twi.write(commands.as_ref())?;

        let mut buf = [0x00; twi::BUFFER_LEN];
        buf[0] = SSD1306_DATA;

        for _ in 0..128 {
            twi.write(buf.as_ref())?;
        }

        Ok(())
    }

    /// Assumes that the splash screen starts with the data flag byte every `twi::BUFFER_LEN -1`th bytes.
    pub fn display_splash(&mut self, twi: &mut twi::TWI, splash: PGMSlice) -> Result<(), twi::TWIError> {
        let commands = [
            SSD1306_COMMAND,
            SSD1306_PAGEADDR,
            0,   // Page start address
            7,   // Page end
            SSD1306_COLUMNADDR,
            0,   // Column start address
            127,    // Column end address
        ];
        twi.write(commands.as_ref())?;

        for chunk in splash.chunks(twi::BUFFER_LEN) {
            twi.write(&chunk)?;
        }

        Ok(())
    }

    pub fn draw_tile<T: ByteBundle>(&mut self, twi: &mut twi::TWI, tile: &T, x: u8, y: u8) -> Result<(), twi::TWIError> {
        let commands = [
            SSD1306_COMMAND,
            SSD1306_PAGEADDR,
            y,   // Page start address
            y,   // Page end
            SSD1306_COLUMNADDR,
            x*8,   // Column start address
            (x*8+7),    // Column end address
        ];
        twi.write(commands.as_ref())?;
        twi.write(tile)
    }
}