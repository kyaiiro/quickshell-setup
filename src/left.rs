use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation};
use crate::custom::{button_module, Button, custom_module, Custom, stream_module};

fn settings_module() -> Label {
    button_module(Button {
        name: "settings",
        label: "\u{f303}",
        tooltip: Some("Open Settings"),
        on_click: Some("alacritty -e nvim $HOME/.config/settings.json"),
        on_click_middle: None,
        on_click_right: None,
    })
}

fn dnd_module() -> Label {
    button_module(Button {
        name: "dnd",
        label: "DND",
        tooltip: Some("Toggle DND lights"),
        on_click: Some("dnd"),
        on_click_middle: Some("notify-send 'DND Status:' $(cat $HOME/.config/DND/dndstatus.txt)"),
        on_click_right: Some("dnd off"),
    })
}

fn clipboard_module() -> Label {
    custom_module(Custom {
        name: "clipboard",
        format: "\u{f014c} {}",
        exec: "$HOME/.config/hypr/scripts/cliphist.sh l",
        interval: 1,
        plain: true,
        tooltip_format: Some("You have {} items copied"),
        on_click: Some("$HOME/.config/hypr/scripts/cliphist.sh c"),
        on_click_right: Some("$HOME/.config/hypr/scripts/cliphist.sh w"),
    })
}

fn cava_module() -> Label {
    stream_module("cava", "$HOME/.config/hypr/scripts/cava.sh")
}

pub fn main_module() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 0);
    container.add_css_class("left");
    container.set_halign(Align::Start);

    container.append(&settings_module());
    container.append(&dnd_module());
    container.append(&clipboard_module());
    container.append(&cava_module());

    container
}