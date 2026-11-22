use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use library::{
    reactive::RwSignal,
    remote::{RemoteSender, remote_signal, spawn},
    types::RawImage,
};
use mpris::{PlaybackStatus, PlayerFinder};

const AUTO_HIDE: Option<Duration> = None;
const ART_PX: u32 = 192;
const BAR_WIDTH: f32 = 150.0;
const TITLE_MAX: usize = 64;
const SUB_MAX: usize = 21;
const POLL_MS: u32 = 100;

#[derive(Clone, Copy)]
pub struct Player {
    pub title: RwSignal<String>,
    pub artist: RwSignal<String>,
    pub album: RwSignal<String>,
    pub progress: RwSignal<f32>, // 0.0..=1.0
    pub playing: RwSignal<bool>,
    pub visible: RwSignal<bool>,
    pub art: RwSignal<Option<Arc<RawImage>>>,
}

impl Player {
    pub fn connect() -> Self {
        let (title, title_tx) = remote_signal(String::new());
        let (artist, artist_tx) = remote_signal(String::new());
        let (album, album_tx) = remote_signal(String::new());
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
                album: album_tx,
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
            album,
        }
    }
}

// ---------------------------------------------------------------------------
// MPRIS side (runs entirely on its own thread)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Last {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub visible: bool,
    pub bar_px: i32,
    pub art_url: String,
}

pub struct Feed {
    pub title: RemoteSender<String>,
    pub artist: RemoteSender<String>,
    pub album: RemoteSender<String>,
    pub progress: RemoteSender<f32>,
    pub playing: RemoteSender<bool>,
    pub visible: RemoteSender<bool>,
    pub art: RemoteSender<Option<Arc<RawImage>>>,
    pub last: Last,
}

impl Feed {
    /// Only sends what changed, so an idle popup causes no UI work at all.
    /// Err means the UI side is gone and the thread should stop.
    pub fn push(
        &mut self,
        title: &str,
        artist: &str,
        album: &str,
        playing: bool,
        fraction: f32,
        visible: bool,
        art_url: &str,
    ) -> Result<(), ()> {
        if !visible {
            // Hidden: don't wake the UI with content, just the hide itself.
            return self.set_visible(false);
        }

        if title != self.last.title || artist != self.last.artist || album != self.last.album {
            self.title.send(clip(title, TITLE_MAX)).map_err(|_| ())?;
            self.album.send(clip(album, SUB_MAX)).map_err(|_| ())?;
            self.artist.send(clip(artist, SUB_MAX)).map_err(|_| ())?;
            self.last.title = title.to_string();
            self.last.artist = artist.to_string();
            self.last.album = album.to_string();
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

    pub fn load_art(url: &str) -> Option<RawImage> {
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

    pub fn set_visible(&mut self, v: bool) -> Result<(), ()> {
        if v != self.last.visible {
            self.visible.send(v).map_err(|_| ())?;
            self.last.visible = v;
        }
        Ok(())
    }
}

pub fn mpris_loop(feed: &mut Feed) {
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
            feed.push("Nothing playing", "", "", false, 0.0, true, "")
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
pub fn follow(player: &mpris::Player, feed: &mut Feed) -> Result<(), ()> {
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
        let album = md.album_name().unwrap_or("").to_string();
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
            &album,
            playing,
            fraction,
            visible,
            md.art_url().unwrap_or(""),
        )?;
    }
}

/// Char-based truncation; the label doesn't clip on its own.
pub fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
