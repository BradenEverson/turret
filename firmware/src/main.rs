//! The main driver for the Rust firmware, uses V4L to continuously stream video data and perform
//! CNN analysis on it

use std::io::Cursor;
use std::time::Duration;

use firmware::server::service::ServerService;
use firmware::server::ServerState;
use firmware::turret::{Action, TurretComplex};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;

use rppal::gpio::Gpio;
use rppal::pwm::{Channel, Pwm};
use tokio::net::TcpListener;
use v4l::io::traits::CaptureStream;
use v4l::prelude::MmapStream;
use v4l::{buffer::Type, Device};

#[tokio::main]
async fn main() {
    let gpio = Gpio::new().expect("Get GPIO");
    let mut turret = TurretComplex::new(gpio, 21, 20).expect("Initialize peripherals");
    let pwm = Pwm::with_frequency(Channel::Pwm0, 50.0, 0.0, rppal::pwm::Polarity::Normal, true)
        .expect("Initialize PWM");

    let listener = TcpListener::bind("0.0.0.0:7878").await.unwrap();

    println!(
        "Listening on http://localhost:{}",
        listener.local_addr().unwrap().port()
    );

    let state = ServerState::default().to_async();
    let (service, mut action_receiver) = ServerService::new(state.clone());

    tokio::spawn(async move {
        let dev = Device::new(0).expect("Failed to open camera");
        let mut stream =
            MmapStream::new(&dev, Type::VideoCapture).expect("Failed to create buffer stream");

        while let Ok((buf, _)) = stream.next() {
            if let Ok(image) = image::load_from_memory_with_format(&buf, image::ImageFormat::Jpeg) {
                let resized_image = image.resize(254, 254, image::imageops::FilterType::Lanczos3);

                if let Ok(mut lock) = state.try_write() {
                    let mut resized_buffer = Cursor::new(Vec::new());
                    if resized_image
                        .write_to(&mut resized_buffer, image::ImageFormat::Jpeg)
                        .is_ok()
                    {
                        let buffer = resized_buffer.into_inner();
                        let _ = lock.send_buffer(&buffer).await;
                    }
                }
            }
        }
    });

    tokio::spawn(async move {
        loop {
            let (socket, _) = listener
                .accept()
                .await
                .expect("Error accepting incoming connection");

            let io = TokioIo::new(socket);
            let service = service.clone();

            tokio::spawn(async move {
                if let Err(e) = http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    eprintln!("Error serving connection: {}", e);
                }
            });
        }
    });

    let min_duty_cycle = 2.5;
    let max_duty_cycle = 12.5;
    let steps = 180;
    let step_duration = Duration::from_millis(20);

    while let Some(action) = action_receiver.recv().await {
        match action {
            Action::Left => turret.move_left(),
            Action::Right => turret.move_right(),
            Action::Shoot => {
                for i in 0..=steps {
                    let duty_cycle = min_duty_cycle
                        + (i as f64) / (steps as f64) * (max_duty_cycle - min_duty_cycle);
                    pwm.set_duty_cycle(duty_cycle).expect("Set duty cycle");
                    std::thread::sleep(step_duration);
                }

                for i in (0..=steps).rev() {
                    let duty_cycle = min_duty_cycle
                        + (i as f64) / (steps as f64) * (max_duty_cycle - min_duty_cycle);
                    pwm.set_duty_cycle(duty_cycle).expect("Set duty cycle");
                    std::thread::sleep(step_duration);
                }

                pwm.set_duty_cycle(0.0).expect("Set duty cycle back to 0");
            }
        }
    }
}
