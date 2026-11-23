use std::{
    f32::consts::PI,
    io::Read,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use library::remote::RemoteSender;
use rustfft::{FftPlanner, num_complex::Complex};

pub const BARS: usize = 32;
pub const LEVEL_MAX: u32 = 100;

const SAMPLE_RATE: usize = 44_100;
const FFT_SIZE: usize = 2048;
const HOP: usize = 1024; // ~23ms per update, ~43 fps
const F_MIN: f32 = 50.0;
const F_MAX: f32 = 12_000.0;
const FLOOR_DB: f32 = -60.0; // anything quieter reads as 0

/// Bin range [lo, hi) per bar, log-spaced between F_MIN and F_MAX.
fn band_edges() -> Vec<(usize, usize)> {
    let bin = |f: f32| (f * FFT_SIZE as f32 / SAMPLE_RATE as f32).round() as usize;
    let mut prev = bin(F_MIN).max(1);
    (0..BARS)
        .map(|i| {
            let f_hi = F_MIN * (F_MAX / F_MIN).powf((i + 1) as f32 / BARS as f32);
            let lo = prev;
            let hi = bin(f_hi).max(lo + 1).min(FFT_SIZE / 2);
            prev = hi;
            (lo, hi)
        })
        .collect()
}

/// Runs on its own thread. Returns when the UI side is gone.
pub fn run(tx: RemoteSender<Vec<u32>>) {
    let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
        .collect();
    let bands = band_edges();

    let mut ring = vec![0.0f32; FFT_SIZE];
    let mut spectrum = vec![Complex::new(0.0f32, 0.0); FFT_SIZE];
    let mut smooth = [0.0f32; BARS];
    let mut last = vec![0u32; BARS];
    let mut bytes = vec![0u8; HOP * 4];

    loop {
        let spawned = Command::new("parec")
            .args([
                "--device=@DEFAULT_MONITOR@",
                "--raw",
                "--format=float32le",
                "--rate=44100",
                "--channels=1",
                "--latency-msec=20",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        let mut child = match spawned {
            Ok(c) => c,
            Err(e) => {
                eprintln!("music-popup: can't start parec: {e}");
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        let mut out = child.stdout.take().unwrap();

        while out.read_exact(&mut bytes).is_ok() {
            // Slide the window: drop the oldest HOP samples, append the new ones.
            ring.copy_within(HOP.., 0);
            for (dst, c) in ring[FFT_SIZE - HOP..].iter_mut().zip(bytes.chunks_exact(4)) {
                *dst = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            }

            for i in 0..FFT_SIZE {
                spectrum[i] = Complex::new(ring[i] * window[i], 0.0);
            }
            fft.process(&mut spectrum);

            for (b, &(lo, hi)) in bands.iter().enumerate() {
                let peak = spectrum[lo..hi]
                    .iter()
                    .map(|c| c.norm())
                    .fold(0.0f32, f32::max);
                // 4/N undoes the FFT scale and the Hann window gain: a full-scale sine reads 1.0.
                let amp = 4.0 * peak / FFT_SIZE as f32;
                let db = 20.0 * (amp + 1e-6).log10();
                let v = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
                // Fast attack, slow decay.
                smooth[b] = if v > smooth[b] {
                    v
                } else {
                    smooth[b] * 0.85 + v * 0.15
                };
            }

            let levels: Vec<u32> = smooth
                .iter()
                .map(|v| (v * LEVEL_MAX as f32).round() as u32)
                .collect();

            // Silence settles to all zeros and then stops sending.
            if levels != last {
                if tx.send(levels.clone()).is_err() {
                    let _ = child.kill();
                    return; // UI gone
                }
                last = levels;
            }
        }

        // parec died (restart, sink change): retry.
        let _ = child.kill();
        let _ = child.wait();
        thread::sleep(Duration::from_secs(1));
    }
}
