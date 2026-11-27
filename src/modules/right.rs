use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation};
use crate::modules::custom::{button_module, Button, json_stream_module, JsonStream};
use crate::modules::workspaces::workspaces_module;

fn power_module() -> Label {
    button_module(Button {
        name: "power",
        label: "\u{f011}",
        tooltip: None,
        on_click: Some("$HOME/.config/hypr/scripts/powermenu.sh"),
        on_click_middle: None,
        on_click_right: None,
    })
}

fn notify_module() -> Label {
    json_stream_module(JsonStream {
        name: "notify",
        exec: "swaync-client -swb",
        exec_if: Some("which swaync-client"),
        format: "{} {icon}",
        icons: &[
            ("notification", "\u{f0a2}<span foreground='red'><sup>\u{f444}</sup></span>"),
            ("none", "\u{f0a2}"),
            ("dnd-notification", "\u{f1f7}<span foreground='red'><sup>\u{f444}</sup></span>"),
            ("dnd-none", "\u{f1f7}"),
            ("inhibited-notification", "\u{f0a2}<span foreground='red'><sup>\u{f444}</sup></span>"),
            ("inhibited-none", "\u{f0a2}"),
            ("dnd-inhibited-notification", "\u{f1f7}<span foreground='red'><sup>\u{f444}</sup></span>"),
            ("dnd-inhibited-none", "\u{f1f7}"),
        ],
        tooltip: Some("Left Click: Launch Notification Center\nRight Click: Do not Disturb"),
        on_click: Some("sleep 0.1 && swaync-client -t -sw"),
        on_click_middle: None,
        on_click_right: Some("swaync-client -d -sw"),
    })
}

// fn network_module() -> Label {

// }

// fn audio_module() -> Label {

// }

pub fn main_module(monitor: &str) -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 0);
    container.add_css_class("right");
    container.set_halign(Align::End);

    container.append(&notify_module());
    container.append(&workspaces_module(monitor));
    container.append(&power_module());

    container
}