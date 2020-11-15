#![no_std]
#![no_main]
#![feature(lang_items, llvm_asm, abi_avr_interrupt)]

mod hal;
mod no_std_stuff;
mod peripherals;
use peripherals::display::Display;
mod game;
use game::{rng::Rng, ContinueState, Game, Input};

use hal::{
    clock::{self, ClockError},
    twi,
    usart::{self, USARTError},
};

use derive_more::From;

/// This is the I2C bus address of the display I'm using.
///
/// I have no idea what model it is, but it's running on a SSD1306 display driver.
const DISPLAY_ADDR: u8 = 0x3C;

/// A little helper function to format a u8 into decimal.
///
/// Used for the game over screen. Bytes are big-endian ordered.
#[allow(dead_code)]
fn format_u8(val: u8) -> [u8; 3] {
    [
        (val / 100) + b'0',
        (val / 10) % 10 + b'0',
        (val % 10) + b'0',
    ]
}

#[derive(Copy, Clone, Eq, PartialEq, From)]
enum ErrorKind {
    TWI(twi::TWIError),
    USART(usart::USARTError),
    Clock(clock::ClockError),
}

fn run() -> Result<(), ErrorKind> {
    // We'll be needing interrupts for the TWI module and the timer used in the clock.
    hal::enable_interrupts();

    let clock = clock::Clock::init()?;

    let mut _usart = usart::USART::init()?;

    let mut twi = twi::TWI::init()?;
    twi.set_address(DISPLAY_ADDR)?;
    let mut display = Display::init(&mut twi)?;

    let mut input = Input::init();
    let mut game = Game::new();

    display.display_splash(&mut twi, Game::title_screen())?;

    // We'll be using the time spent on the title screen as the seed for the RNG.
    // Given the precision of the clock, that gives 2^16 game states.
    let now = clock.now();
    // Wait for player to press button
    loop {
        let now = clock.now();
        if input.update(now) {
            break;
        }
    }
    let seed = clock.now().elapsed(now);
    let mut rng = Rng::new(seed);

    loop {
        game.new_map(&mut rng);
        game.draw(&mut display, &mut twi)?;

        // A single game's main loop.
        loop {
            let now = clock.now();

            let had_input = input.update(now);

            if had_input {
                match game.update(&input) {
                    ContinueState::NewLevel => {
                        game.new_map(&mut rng);
                        game.draw(&mut display, &mut twi)?;
                        continue;
                    }
                    ContinueState::Continue => {}
                    ContinueState::GameOver => break,

                    // Happens if the player tries walking into a wall.
                    // Probably shouldn't be visible here.
                    ContinueState::RestartLoop => continue,
                }

                game.draw(&mut display, &mut twi)?;
            }
        }

        // Game over state.
        display.display_splash(&mut twi, Game::game_over_screen())?;
        let level = game.level();
        let numbers = format_u8(level);

        display.set_draw_coords(&mut twi, 10, 5)?;
        for digit in numbers.iter().filter(|d| **d != b'0') {
            let tile = Game::get_digit_tile(*digit);
            display.draw_tile(&mut twi, &tile)?;
        }

        // Wait for player to press button
        loop {
            let now = clock.now();
            if input.update(now) {
                break;
            }
        }

        game.reset();
    }
}

#[no_mangle]
pub extern "C" fn main() {
    match run() {
        Err(ErrorKind::TWI(twi::TWIError::BufferLenError)) => hal::blink_error_code(1),
        Err(ErrorKind::TWI(twi::TWIError::SendAddressNACK)) => hal::blink_error_code(2),
        Err(ErrorKind::TWI(twi::TWIError::SendDataNACK)) => hal::blink_error_code(3),
        Err(ErrorKind::TWI(twi::TWIError::NotReady)) => hal::blink_error_code(4),
        Err(ErrorKind::TWI(twi::TWIError::BusError)) => hal::blink_error_code(5),
        Err(ErrorKind::TWI(twi::TWIError::InitError)) => hal::blink_error_code(6),
        Err(ErrorKind::TWI(twi::TWIError::InvalidAddress)) => hal::blink_error_code(7),
        Err(ErrorKind::USART(USARTError::InitError)) => hal::blink_error_code(8),
        Err(ErrorKind::Clock(ClockError::InitError)) => hal::blink_error_code(9),
        Ok(()) => {}
    }
}
