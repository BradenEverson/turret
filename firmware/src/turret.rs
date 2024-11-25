//! Turret control system:

use std::{thread, time::Duration};

use motor::MotorDriver;
use rppal::{
    gpio::{Gpio, OutputPin},
    pwm::Channel,
};

pub mod motor;

/// A turret control complex, controls a single servo motor for pulling the trigger and a stepper
/// motor's direction and step pins on a driver
pub struct TurretComplex {
    step: OutputPin,
    dir: OutputPin,
    motor: MotorDriver,
}

impl TurretComplex {
    /// Creates a new turret complex from two gpio pins and a pwm pin
    pub fn new(gpio: Gpio, step: u8, dir: u8, channel: Channel) -> Option<Self> {
        let step = gpio.get(step).ok()?.into_output();
        let dir = gpio.get(dir).ok()?.into_output();
        let motor = MotorDriver::new(channel);

        Some(Self { step, dir, motor })
    }

    /// Moves the turret left
    pub fn move_left(&mut self) {
        for _ in 0..50 {
            self.step.set_high();
            thread::sleep(Duration::from_millis(10));
            self.step.set_low();
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Moves the turret left
    pub fn move_right(&mut self) {
        self.dir.set_high();
        for _ in 0..50 {
            self.step.set_high();
            thread::sleep(Duration::from_millis(10));
            self.step.set_low();
            thread::sleep(Duration::from_millis(10));
        }
        self.dir.set_low();
    }

    /// Shoots the turret
    pub fn shoot(&mut self) {
        self.motor.set_angle(160f64);

        thread::sleep(Duration::from_millis(700));

        self.motor.set_angle(50f64);
        thread::sleep(Duration::from_millis(700));
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
