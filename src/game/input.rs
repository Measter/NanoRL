//! Manages all of the input for the game.
//!
//! The game doesn't need to know exactly how the input is obtained, just that input happens,
//! so we'll hide it away in here.

use crate::{
    hal::{clock::Instant, ports::PortD},
    peripherals::button::Button,
};

pub struct Input {
    left: Button<PortD>,
    right: Button<PortD>,
    up: Button<PortD>,
    down: Button<PortD>,
}

impl Input {
    pub fn init() -> Input {
        Input {
            left: Button::new(PortD::PD3),
            right: Button::new(PortD::PD4),
            down: Button::new(PortD::PD5),
            up: Button::new(PortD::PD6),
        }
    }

    /// Returns whether any of the inputs were pressed.
    pub fn update(&mut self, now: Instant) -> bool {
        self.left.update(now);
        self.right.update(now);
        self.up.update(now);
        self.down.update(now);

        self.left() | self.right() | self.up() | self.down()
    }

    pub fn left(&self) -> bool {
        self.left.was_pressed()
    }

    pub fn right(&self) -> bool {
        self.right.was_pressed()
    }

    pub fn up(&self) -> bool {
        self.up.was_pressed()
    }

    pub fn down(&self) -> bool {
        self.down.was_pressed()
    }
}
