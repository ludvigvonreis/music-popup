#![allow(dead_code)]
use std::{
    io::Read,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use library::{
    Anchor, Weight,
    app::App,
    color::Color,
    reactive::RwSignal,
    remote::{RemoteSender, remote_signal, spawn},
    style::Decorators,
    types::RawImage,
    vector::Vec2,
    view::{IntoView, view_iter::IntoViewIter},
    views::{div, label, remote_image},
    window,
};
use mpris::{PlaybackStatus, PlayerFinder};

const W: f32 = 380.0;
const H: f32 = 84.0;
const ART_PX: u32 = 96;
const BAR_WIDTH: f32 = 364.0;

const BACKGROUND: &str = "#211f25";
const ART_BACKGROUND: &str = "#39313b";
const TEXT: &str = "#eee8e8";
const MUTED: &str = "#b9aeb5";
const TRACK: &str = "#443941";
const ACCENT: &str = "#d5a0b9";
const ACCENT_PAUSED: &str = "#856a7b";
/// None = always visible. Some(d) = hide d after the last track/play-state change.
const AUTO_HIDE: Option<Duration> = None;
/// MPRIS poll interval in ms. Redraws only happen when a value changes.
const POLL_MS: u32 = 500;

fn main() {
    App::new("music-popup")
        .size(380, 80)
        .margin(16, 16, 0, 0)
        .anchor(Anchor::Top | Anchor::Right)
        .output("HDMI-A-1")
        .run(music_status);
}

/// What the UI reads. Signals are `Copy` and `!Send`, so they stay on the
/// main thread; the MPRIS thread only holds `RemoteSender`s.
#[derive(Clone, Copy)]
struct Player {
    title: RwSignal<String>,
    artist: RwSignal<String>,
    progress: RwSignal<f32>, // 0.0..=1.0
    playing: RwSignal<bool>,
    visible: RwSignal<bool>,
    art: RwSignal<Option<Arc<RawImage>>>,
}

impl Player {
    fn connect() -> Self {
        let (title, title_tx) = remote_signal(String::new());
        let (artist, artist_tx) = remote_signal(String::new());
        let (progress, progress_tx) = remote_signal(0.0_f32);
        let (playing, playing_tx) = remote_signal(false);
        let (visible, visible_tx) = remote_signal(AUTO_HIDE.is_none());
        let (art, art_tx) = remote_signal::<Option<Arc<RawImage>>>(None);

        spawn(move || {
            let mut feed = Feed {
                title: title_tx,
                artist: artist_tx,
                progress: progress_tx,
                playing: playing_tx,
                visible: visible_tx,
                art: art_tx,
                last: Last {
                    visible: AUTO_HIDE.is_none(),
                    ..Default::default()
                },
            };
            mpris_loop(&mut feed);
        });

        Player {
            title,
            artist,
            progress,
            playing,
            visible,
            art,
        }
    }
}

// ---------------------------------------------------------------------------
// MPRIS side (runs entirely on its own thread)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Last {
    title: String,
    artist: String,
    playing: bool,
    visible: bool,
    bar_px: i32,
    art_url: String,
}

struct Feed {
    title: RemoteSender<String>,
    artist: RemoteSender<String>,
    progress: RemoteSender<f32>,
    playing: RemoteSender<bool>,
    visible: RemoteSender<bool>,
    art: RemoteSender<Option<Arc<RawImage>>>,
    last: Last,
}

impl Feed {
    /// Only sends what changed, so an idle popup causes no UI work at all.
    /// Err means the UI side is gone and the thread should stop.
    fn push(
        &mut self,
        title: &str,
        artist: &str,
        playing: bool,
        fraction: f32,
        visible: bool,
        art_url: &str,
    ) -> Result<(), ()> {
        if !visible {
            // Hidden: don't wake the UI with content, just the hide itself.
            return self.set_visible(false);
        }

        if title != self.last.title || artist != self.last.artist {
            self.title.send(clip(title, 32)).map_err(|_| ())?;
            self.artist.send(clip(artist, 40)).map_err(|_| ())?;
            self.last.title = title.to_string();
            self.last.artist = artist.to_string();
        }
        if playing != self.last.playing {
            self.playing.send(playing).map_err(|_| ())?;
            self.last.playing = playing;
        }
        // Quantize to whole pixels of the bar; sub-pixel changes are invisible.
        let px = (fraction * BAR_WIDTH).round() as i32;
        if px != self.last.bar_px {
            self.progress.send(fraction).map_err(|_| ())?;
            self.last.bar_px = px;
        }

        if art_url != self.last.art_url {
            // Remember the URL even on failure so a bad URL isn't retried every poll.
            self.last.art_url = art_url.to_string();
            let img = Self::load_art(art_url).map(Arc::new);
            self.art.send(img).map_err(|_| ())?;
        }
        // Content first, visibility last: no flash of stale info on show.
        self.set_visible(true)
    }

    fn load_art(url: &str) -> Option<RawImage> {
        if url.is_empty() {
            return None;
        }

        let bytes: Vec<u8> = if let Some(path) = url.strip_prefix("file://") {
            let path = percent_encoding::percent_decode_str(path)
                .decode_utf8()
                .ok()?;
            std::fs::read(path.as_ref())
                .map_err(|e| eprintln!("art read failed: {e}"))
                .ok()?
        } else if url.starts_with("http://") || url.starts_with("https://") {
            // keep the ureq code that compiles for you here, with .map_err(|e| eprintln!("art http failed: {e}")) before each .ok()?
            todo!()
        } else {
            eprintln!("art: unsupported scheme");
            return None;
        };

        let img = image::load_from_memory(&bytes)
            .map_err(|e| eprintln!("art decode failed: {e}"))
            .ok()?
            .resize_to_fill(ART_PX, ART_PX, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        Some(RawImage {
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        })
    }

    fn set_visible(&mut self, v: bool) -> Result<(), ()> {
        if v != self.last.visible {
            self.visible.send(v).map_err(|_| ())?;
            self.last.visible = v;
        }
        Ok(())
    }
}

fn mpris_loop(feed: &mut Feed) {
    let finder = match PlayerFinder::new() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("music-popup: can't connect to D-Bus: {e}");
            return;
        }
    };

    loop {
        if let Ok(player) = finder.find_active() {
            if follow(&player, feed).is_err() {
                return; // UI gone
            }
        }
        // No player (or it quit).
        let res = if AUTO_HIDE.is_none() {
            feed.push("Nothing playing", "", false, 0.0, true, "")
        } else {
            feed.set_visible(false)
        };
        if res.is_err() {
            return; // UI gone
        }
        thread::sleep(Duration::from_secs(1));
    }
}

/// Follows one player until it quits. Err = UI side disposed.
fn follow(player: &mpris::Player, feed: &mut Feed) -> Result<(), ()> {
    let Ok(mut tracker) = player.track_progress(POLL_MS) else {
        return Ok(());
    };
    let mut shown_at = Instant::now();

    loop {
        let tick = tracker.tick();
        if tick.player_quit {
            return Ok(());
        }
        let p = tick.progress;
        let md = p.metadata();

        let title = md.title().unwrap_or("Unknown").to_string();
        let artist = md.artists().map(|a| a.join(", ")).unwrap_or_default();
        let playing = p.playback_status() == PlaybackStatus::Playing;
        let fraction = match p.length() {
            Some(len) if !len.is_zero() => {
                (p.position().as_secs_f32() / len.as_secs_f32()).clamp(0.0, 1.0)
            }
            _ => 0.0,
        };

        // A new track or a play/pause toggle (re)starts the show timer.
        let changed =
            title != feed.last.title || artist != feed.last.artist || playing != feed.last.playing;
        if changed {
            shown_at = Instant::now();
        }
        let visible = AUTO_HIDE.map_or(true, |d| shown_at.elapsed() < d);

        feed.push(
            &title,
            &artist,
            playing,
            fraction,
            visible,
            md.art_url().unwrap_or(""),
        )?;
    }
}

/// Char-based truncation; the label doesn't clip on its own.
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

fn music_status() -> impl IntoView {
    let player = Player::connect();

    window::bind_visible(player.visible);

    transparent_rect((album_art(player), track_info(player), progress_bar(player)))
}

fn track_info(p: Player) -> impl IntoView {
    let title = label(move || p.title.get()).style(|s| {
        s.text_color(Color::from_hex(TEXT))
            .font_size(16.0)
            .weight(Weight::SEMIBOLD)
            .position(Vec2::new(68.0, 13.0))
    });

    let artist = label(move || p.artist.get()).style(|s| {
        s.text_color(Color::from_hex(MUTED))
            .font_size(13.0)
            .weight(Weight::NORMAL)
            .position(Vec2::new(68.0, 35.0))
    });

    div((title, artist)).style(|s| s.size(Vec2::new(300.0, 54.0)))
}

fn album_art(p: Player) -> impl IntoView {
    div((
        div(()).style(|s| {
            s.background_color(Color::from_hex(ART_BACKGROUND))
                .border_radius(8.0)
                .position(Vec2::new(0.0, 0.0))
                .size(Vec2::new(48.0, 48.0))
        }),
        remote_image(move || p.art.get()).style(|s| {
            s.border_radius(8.0)
                .position(Vec2::new(0.0, 0.0))
                .size(Vec2::new(48.0, 48.0))
        }),
    ))
    .style(|s| {
        s.position(Vec2::new(10.0, 10.0))
            .size(Vec2::new(48.0, 48.0))
    })
}

fn progress_bar(p: Player) -> impl IntoView {
    div((
        div(()).style(|s| {
            s.background_color(Color::from_hex(TRACK))
                .border_radius(3.0)
                .position(Vec2::new(10.0, 66.0))
                .size(Vec2::new(BAR_WIDTH, 4.0))
        }),
        div(()).style(move |s| {
            let progress = p.progress.get().clamp(0.0, 1.0);
            let width = BAR_WIDTH * progress;

            let fill = if p.playing.get() {
                ACCENT
            } else {
                ACCENT_PAUSED
            };

            s.background_color(Color::from_hex(fill))
                .border_radius(3.0)
                .position(Vec2::new(10.0, 66.0))
                .size(Vec2::new(width, 4.0))
        }),
    ))
}

fn transparent_rect(children: impl IntoViewIter) -> impl IntoView {
    div(children).style(|s| {
        s.border_radius(12.0)
            .position(Vec2::new(0.0, 0.0))
            .size(Vec2::new(W, H))
            .background_color(Color::from_hex(BACKGROUND))
    })
}
