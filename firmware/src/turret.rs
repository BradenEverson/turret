//! Turret control system:

use std::{thread, time::Duration};

use rppal::{
    gpio::{Gpio, OutputPin},
    pwm::{Channel, Pwm},
};

/// A turret control complex, controls a single servo motor for pulling the trigger and a stepper
/// motor's direction and step pins on a driver
pub struct TurretComplex {
    step: OutputPin,
    dir: OutputPin,
    trigger: Pwm,
}

impl TurretComplex {
    /// Creates a new turret complex from two gpio pins and a pwm pin
    pub fn new(gpio: Gpio, step: u8, dir: u8, trigger: Channel) -> Option<Self> {
        let step = gpio.get(step).ok()?.into_output();
        let dir = gpio.get(dir).ok()?.into_output();
        let trigger = Pwm::new(trigger).ok()?;

        Some(Self { step, dir, trigger })
    }

    /// Moves the turret left
    pub fn move_left(&mut self) {
        self.dir.set_high();
        self.step.set_high();
        thread::sleep(Duration::from_millis(10));
        self.step.set_low();
        self.dir.set_low();
    }

    /// Moves the turret left
    pub fn move_right(&mut self) {
        self.step.set_high();
        thread::sleep(Duration::from_millis(10));
        self.step.set_low();
    }

    /// Shoots the turret
    pub fn shoot(&mut self) {
        println!("{:?}", self.trigger)
    }
}

/// An action the turret can take
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Action {
    /// Move left
    Left,
    /// Move right
    Right,
    /// SHOOT
    Shoot,
}
