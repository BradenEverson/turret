//! Motor wrapper

use rppal::pwm::Pwm;

/// Minimum pulse width
const MIN_PULSE_WIDTH: f64 = 1f64;
/// Maximum pulse width
const MAX_PULSE_WIDTH: f64 = 2.5;

/// Motor Driving struct
pub struct MotorDriver {
    pwm: Pwm,
}

impl MotorDriver {
    /// Creates a new motor driver
    pub fn new(channel: rppal::pwm::Channel) -> Self {
        let pwm = Pwm::with_frequency(channel, 55f64, 0f64, rppal::pwm::Polarity::Normal, true)
            .expect("PWM Init");

        Self { pwm }
    }

    /// Sets new angle
    pub fn set_angle(&mut self, angle: f64) {
        let angle = angle.clamp(0f64, 180f64);

        let pulse_width = MIN_PULSE_WIDTH + (MAX_PULSE_WIDTH - MIN_PULSE_WIDTH) * (angle / 180f64);
        let duty_cycle = pulse_width / (1000f64 / 55f64);

        self.pwm
            .set_duty_cycle(duty_cycle)
            .expect("Duty cycle was bad");
    }

    /// Sets duty cycle
    pub fn set_duty(&mut self, duty: f64) {
        self.pwm.set_duty_cycle(duty).expect("Duty cycle was bad");
    }
}
