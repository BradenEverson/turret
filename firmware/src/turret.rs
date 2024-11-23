//! Turret control system:

use std::{thread, time::Duration};

use jetgpio::{
    gpio::{pins::OutputPin, valid_pins::ValidPin, Result},
    pwm::valid_pwm::ValidPwmPin,
    Gpio, Pwm,
};

/// A turret control complex, controls a single servo motor for pulling the trigger and a stepper
/// motor's direction and step pins on a driver
#[derive(Clone, Copy)]
pub struct TurretComplex {
    step: OutputPin,
    dir: OutputPin,
    trigger: Pwm,
}

impl TurretComplex {
    /// Creates a new turret complex from two gpio pins and a pwm pin
    pub fn new<GPIO1: ValidPin, GPIO2: ValidPin, PWM: ValidPwmPin + ValidPin>(
        gpio: Gpio,
        step: GPIO1,
        dir: GPIO2,
        trigger: PWM,
    ) -> Result<Self> {
        let step = gpio.get_output(step)?;
        let dir = gpio.get_output(dir)?;
        let trigger = Pwm::new(trigger)?;

        Ok(Self { step, dir, trigger })
    }

    /// Moves the turret left
    pub fn move_left(&mut self) -> Result<()> {
        Ok(())
    }

    /// Moves the turret left
    pub fn move_right(&mut self) -> Result<()> {
        self.step.set_high()?;
        self.step.set_low()?;
        Ok(())
    }

    /// Shoots the turret
    pub fn shoot(&mut self) -> Result<()> {
        println!("SHOOTING");
        Ok(())
    }
}

/// An action the turret can take
pub enum Action {
    /// Move left
    Left,
    /// Move right
    Right,
    /// SHOOT
    Shoot,
}
