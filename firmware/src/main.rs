//! The main driver for the Rust firmware, uses V4L to continuously stream video data and perform
//! CNN analysis on it

use std::io::Cursor;

use firmware::server::service::ServerService;
use firmware::server::ServerState;
use firmware::turret::{Action, TurretComplex};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;

use rppal::gpio::Gpio;
use tokio::net::TcpListener;
use v4l::io::traits::CaptureStream;
use v4l::prelude::MmapStream;
use v4l::{buffer::Type, Device};

#[tokio::main]
async fn main() {
    let gpio = Gpio::new().expect("Get GPIO");
    let mut turret = TurretComplex::new(gpio, 21, 20).expect("Initialize peripherals");

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
            // Decode raw buffer into an RGB image
            if let Ok(image) = image::load_from_memory_with_format(&buf, image::ImageFormat::Jpeg) {
                // Resize the image to 254x254
                let resized_image = image.resize(254, 254, image::imageops::FilterType::Lanczos3);

                if let Ok(mut lock) = state.try_write() {
                    let mut resized_buffer = Cursor::new(Vec::new()); // Create a Cursor wrapping a Vec<u8>
                    if resized_image
                        .write_to(&mut resized_buffer, image::ImageFormat::Jpeg)
                        .is_ok()
                    {
                        let buffer = resized_buffer.into_inner(); // Extract the Vec<u8> from the Cursor
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

    while let Some(action) = action_receiver.recv().await {
        match action {
            Action::Left => turret.move_left(),
            Action::Right => turret.move_right(),
            Action::Shoot => turret.shoot(),
        }
    }
}

