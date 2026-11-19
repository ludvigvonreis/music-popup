#![allow(dead_code)]
mod app;
mod backend;
mod color;
mod reactive;
mod remote;
mod style;
mod types;
mod vector;
mod view;
mod views;

use std::{thread, time::Duration};

use crate::{
    app::App,
    color::Color,
    reactive::RwSignal,
    remote::{remote_signal, spawn},
    style::Decorators,
    vector::Vec2,
    view::{IntoView, view_iter::IntoViewIter},
    views::{div, label, text},
};

use glyphon::Weight;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

fn main() {
    App::new("music-popup")
        .size(380, 80)
        .margin(540 - 150, 960 - 350 / 2, 0, 0)
        .anchor(Anchor::Top | Anchor::Right)
        //.run(component);
        .run(music_status);
}

/// What the UI reads. Signals are `Copy` and `!Send`, so this stays on the
/// main thread; the threads that feed it only ever hold `RemoteSender`s.
#[derive(Clone, Copy)]
struct Player {
    title: RwSignal<String>,
    artist: RwSignal<String>,
    progress: RwSignal<f32>, // 0.0..=1.0
}

impl Player {
    /// Creates the signals and starts the source thread. Components don't know
    /// threads exist. Swap the body of the thread for your MPRIS listener.
    fn connect() -> Self {
        let (title, title_tx) = remote_signal("Virtual Insanity".to_string());
        let (artist, artist_tx) = remote_signal("Jamiroquai".to_string());
        let (progress, progress_tx) = remote_signal(0.0_f32);

        spawn(move || {
            let mut pos = 0.0_f32;
            let mut n = 0u32;
            loop {
                thread::sleep(Duration::from_secs(1));
                pos = (pos + 0.02) % 1.0;
                n += 1;
                // Err means the UI side was disposed: stop the thread.
                if progress_tx.send(pos).is_err() {
                    break;
                }
                if n % 10 == 0 {
                    // Both land in the same batch, so one frame.
                    let _ = title_tx.send(format!("Track {n}"));
                    let _ = artist_tx.send("Demo artist".to_string());
                }
            }
        });

        Player {
            title,
            artist,
            progress,
        }
    }
}

fn music_status() -> impl IntoView {
    let player = Player::connect();
    transparent_rect((album_art(), track_info(player), progress_bar(player)))
}

fn track_info(p: Player) -> impl IntoView {
    let title = label(move || p.title.get()).style(|s| {
        s.text_color(Color::from_hex("#d9d7ce")) // foreground
            .font_size(17.0)
            .weight(Weight::SEMIBOLD)
            .position(Vec2::new(62.0, 10.0))
    });
    let artist = label(move || p.artist.get()).style(|s| {
        s.text_color(Color::from_hex("#b5b3ac")) // color8 (muted)
            .font_size(13.0)
            .weight(Weight::NORMAL)
            .position(Vec2::new(62.0, 30.0))
    });
    div((title, artist)).style(|s| s.size(Vec2::new(360.0, 36.0)))
}

fn album_art() -> impl IntoView {
    div(()).style(|s| {
        s.background_color(Color::from_hex("#343f4c")) // selection_background
            .border_radius(4.0)
            .position(Vec2::new(8.0, 8.0))
            .size(Vec2::new(44.0, 44.0))
    })
}

fn progress_bar(p: Player) -> impl IntoView {
    div((
        div(()).style(|s| {
            s.background_color(Color::from_hex("#343f4c")) // selection_background (track)
                .border_radius(4.0)
                .position(Vec2::new(8.0, 60.0))
                .size(Vec2::new(364.0, 5.0))
        }),
        div(()).style(move |s| {
            let width = 364.0 * p.progress.get().clamp(0.0, 1.0);
            s.background_color(Color::from_hex("#cfbafa")) // cursor/accent (fill)
                .border_radius(4.0)
                .position(Vec2::new(8.0, 60.0))
                .size(Vec2::new(width, 5.0))
        }),
    ))
}

fn transparent_rect(children: impl IntoViewIter) -> impl IntoView {
    div(children).style(|s| {
        s.border_radius(10.0)
            .position(Vec2::new(0.0, 0.0))
            .size(Vec2::new(380.0, 80.0))
            .background_color(Color::from_hex("#212733cc"))
    })
}
