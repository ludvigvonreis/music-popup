#![allow(dead_code)]

use library::{
    Anchor, Weight,
    app::App,
    color::Color,
    layout::{AlignItems, JustifyContent},
    style::{Decorators, pct, px},
    view::{IntoView, view_iter::IntoViewIter},
    views::{div, label, remote_image},
    window,
};
mod player;
use player::Player;

const FONT: &str = "Inter";
const OUTER_BG: &str = "#1b1d1e";
const INNER_BG: &str = "#1f2122";
const RIM: &str = "#ffffff1c";
const ART_BACKGROUND: &str = "#2a2c2d";
const TEXT: &str = "#ffffff";
const SUB: &str = "#a9abac";
const ARTIST: &str = "#c4c6c7";
const TRACK: &str = "#343637";
const FILL: &str = "#e8e8e8";
const FILL_PAUSED: &str = "#8a8c8d";

fn main() {
    App::new("music-popup")
        .size(384, 104)
        .margin(16, 16, 0, 0)
        .anchor(Anchor::Top | Anchor::Right)
        //.output("DP-2")
        .output("HDMI-A-1")
        .run(music_status);
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------
fn music_status() -> impl IntoView {
    let player = Player::connect();
    window::bind_visible(player.visible);

    card((album_art(player), track_info(player)))
}

fn card(children: impl IntoViewIter) -> impl IntoView {
    div(children).style(|s| {
        s.flex_row()
            .padding(6.0)
            .gap(12.0)
            .border_radius(20.0)
            .background_color(Color::from_hex(INNER_BG))
            .border(1.0, Color::from_hex(RIM))
    })
}

fn album_art(p: Player) -> impl IntoView {
    div((remote_image(move || p.art.get())
        .style(|s| s.width(pct(1.0)).height(pct(1.0)).border_radius(14.0)),))
    .style(|s| {
        s.size(90.0, 90.0)
            .shrink(0.0)
            .border_radius(14.0)
            .background_color(Color::from_hex(ART_BACKGROUND))
    })
}

fn track_info(p: Player) -> impl IntoView {
    let title = label(move || p.title.get()).style(|s| {
        s.text_color(Color::from_hex(TEXT))
            .font_family(FONT.to_string())
            .font_size(14.0)
            .weight(Weight::SEMIBOLD)
    });
    let album = label(move || p.album.get()).style(|s| {
        s.text_color(Color::from_hex(SUB))
            .font_family(FONT.to_string())
            .font_size(11.0)
            .weight(Weight::NORMAL)
    });
    let artist = label(move || p.artist.get()).style(|s| {
        s.text_color(Color::from_hex(ARTIST))
            .font_family(FONT.to_string())
            .font_size(11.0)
            .weight(Weight::SEMIBOLD)
    });

    div((title, album, artist, progress_bar(p))).style(|s| {
        s.flex_column()
            .align(AlignItems::Center)
            .justify(JustifyContent::Center)
            .gap(2.0)
            .basis(px(0.0))
            .grow(1.0)
            .min_width(px(0.0))
    })
}

fn progress_bar(p: Player) -> impl IntoView {
    let fill = div(()).style(move |s| {
        let progress = p.progress.get().clamp(0.0, 1.0);
        let color = if p.playing.get() { FILL } else { FILL_PAUSED };
        s.basis(px(0.0))
            .grow(progress)
            .height(px(3.0))
            .border_radius(1.5)
            .background_color(Color::from_hex(color))
    });

    let knob = div(()).style(move |s| {
        let color = if p.playing.get() { FILL } else { FILL_PAUSED };
        s.size(6.0, 6.0)
            .shrink(0.0)
            .border_radius(3.0)
            .background_color(Color::from_hex(color))
    });

    let rest = div(()).style(move |s| {
        let progress = p.progress.get().clamp(0.0, 1.0);
        s.basis(px(0.0))
            .grow(1.0 - progress)
            .height(px(3.0))
            .border_radius(1.5)
            .background_color(Color::from_hex(TRACK))
    });

    div((fill, knob, rest)).style(|s| {
        s.flex_row()
            .align(AlignItems::Center)
            .width(pct(1.0))
            .height(px(6.0))
            .shrink(0.0)
    })
}
