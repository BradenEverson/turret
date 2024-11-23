//! The main driver for the Rust firmware, usese V4L to continuously stream video data and perform
//! CNN analysis on it

use std::time::Duration;

use firmware::server::service::ServerService;
use firmware::server::ServerState;
use firmware::turret::{Action, TurretComplex};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;

use rppal::gpio::Gpio;
use rppal::pwm::Channel;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{channel, Sender};
use v4l::io::traits::CaptureStream;
use v4l::prelude::MmapStream;
use v4l::{buffer::Type, Device};

#[tokio::main]
async fn main() {
    let gpio = Gpio::new().expect("Get GPIO");
    let mut turret =
        TurretComplex::new(gpio, 26, 19, Channel::Pwm0).expect("Initialize peripherals");

    let listener = TcpListener::bind("0.0.0.0:7878").await.unwrap();

    println!(
        "Listening on http://localhost:{}",
        listener.local_addr().unwrap().port()
    );

    let (sender, mut receiver): (Sender<Vec<u8>>, _) = channel(16);
    let state = ServerState::default().to_async();
    let (service, mut action_receiver) = ServerService::new(state.clone());

    tokio::spawn(async move {
        let dev = Device::new(0).expect("Failed to open camera");
        let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4)
            .expect("Failed to create buffer stream");

        while let Ok((buf, _)) = stream.next() {
            let _ = sender
                .try_send(buf.to_vec())
                .map_err(|_| println!("Queue full, frame dropping"));
            std::thread::sleep(Duration::from_millis(33));
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

    tokio::spawn(async move {
        while let Some(action) = action_receiver.recv().await {
            match action {
                Action::Left => turret.move_left(),
                Action::Right => turret.move_right(),
                Action::Shoot => turret.shoot(),
            }
        }
    });

    while let Some(buf) = receiver.recv().await {
        {
            let _ = state.write().await.send_buffer(&buf).await;
        }
    }
}
