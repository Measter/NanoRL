use crate::hal::{
    ports::{Port, PinMode},
    clock::Instant,
};

const DEBOUNCE_TIME: u16 = 2;

/// This type implements some software debouncing of the button input, as well
/// as keeps track of whether the state was changed in the previous update.
pub struct Button<PinPort: Port> {
    is_pressed: bool,
    had_state_change: bool,
    last_press_time: Instant,
    pin: PinPort::ValidPins,
}

impl<PinPort: Port> Button<PinPort> {
    /// Initialises the port to Input with the internal pullup enabled.
    pub fn new(pin: PinPort::ValidPins) -> Self {
        PinPort::set_pin_mode(pin, PinMode::InputPullup);

        Self {
            is_pressed: false,
            had_state_change: false,
            last_press_time: Default::default(),
            pin
        }
    }

    /// Updates the current state of the pin, and returns whether or not the current state of the button changed.
    pub fn update(&mut self, cur_time: Instant) -> bool {
        // The state here is the state of the pin, not whether the button is pressed.
        // Because we're using the pullup resister, pressing the button will ground the
        // pin, causing it to read *low* when pressed.
        // I'm inverting here because I find it easier to reason about if true = pressed.
        let cur_pressed = !PinPort::get_pin_state(self.pin);

        self.had_state_change = false;

        let time_dif = cur_time.elapsed(self.last_press_time);
        // If the the switch is open, but our current state is closed, then we need to update.
        // Likewise, if the switch is closed, and our current state is open, and we've given time for the 
        // button to stop bouncing, then we need to update.
        if (!cur_pressed && self.is_pressed) || (cur_pressed && !self.is_pressed && time_dif >= DEBOUNCE_TIME) {
            self.is_pressed = cur_pressed;
            self.last_press_time = cur_time;
            self.had_state_change = true;
        }

        self.had_state_change
    }

    /// Returns whether the button was pressed since the last update.
    pub fn was_pressed(&self) -> bool {
        self.had_state_change && self.is_pressed
    }
}