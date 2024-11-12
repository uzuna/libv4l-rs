use std::io;
use tokio::time::Instant;
use v4l::buffer::Type;
use v4l::io::traits::AsyncCaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;

fn main() -> io::Result<()> {
    let path = "/dev/video0";
    println!("Using device: {}\n", path);

    // Capture 4 frames by default
    let count = 20;

    // Allocate 4 buffers by default
    let buffer_count = 4;

    let dev = Device::with_path(path)?;
    let format = dev.format()?;
    let params = dev.params()?;
    println!("Active format:\n{}", format);
    println!("Active parameters:\n{}", params);

    // Setup a buffer stream and grab a frame, then print its data
    let stream = MmapStream::with_buffers(&dev, Type::VideoCapture, buffer_count)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(run(stream, count))?;
    Ok(())
}

async fn run(mut stream: MmapStream<'_>, count: usize) -> io::Result<()> {
    // warmup
    stream.poll_next().await?;

    let start = Instant::now();
    let mut megabytes_ps: f64 = 0.0;
    for i in 0..count {
        let t0 = Instant::now();
        let (buf, _meta) = stream.poll_next().await?;
        let duration_us = t0.elapsed().as_micros();

        let cur = buf.len() as f64 / 1_048_576.0 * 1_000_000.0 / duration_us as f64;
        if i == 0 {
            megabytes_ps = cur;
        } else {
            // ignore the first measurement
            let prev = megabytes_ps * (i as f64 / (i + 1) as f64);
            let now = cur * (1.0 / (i + 1) as f64);
            megabytes_ps = prev + now;
        }
    }

    println!();
    println!("FPS: {}", count as f64 / start.elapsed().as_secs_f64());
    println!("MB/s: {}", megabytes_ps);

    Ok(())
}
