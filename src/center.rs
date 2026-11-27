use gtk4::prelude::*;
use gtk4::{glib, Align, Box as GtkBox, Label, Orientation};

use crate::custom::{custom_module, Custom};

fn status_module() -> Label {
    custom_module(Custom {
        name: "status",
        format: "",
        exec: "$HOME/.config/hypr/scripts/server.sh",
        interval: 5,
        on_click: Some("sh $HOME/.config/hypr/scripts/server-toggle.sh"),
        on_click_right: Some("alacritty -e ssh $(qp -r '.server_ip' $HOME/.config/settings.json)"),
        ..Default::default()
    })
}

fn clock_module() -> Label {
    let label = Label::new(None);
    label.add_css_class("clock");
    label.set_valign(Align::Center);

    let update = {
        let label = label.clone();
        move || {
            let now = glib::DateTime::now_local().expect("no local time");
            label.set_text(&now.format(" %I:%M %p").expect("bad format"));
        }
    };

    update();
    glib::timeout_add_seconds_local(1, move || {
        update();
        glib::ControlFlow::Continue
    });

    label
}

fn tailscale_module() -> Label {
    custom_module(Custom {
        name: "tailscale",
        format: "🖧",
        exec: "python $HOME/.config/hypr/scripts/tailscale.py -s",
        interval: 5,
        on_click: Some("python $HOME/.config/hypr/scripts/tailscale.py -t"),
        on_click_right: None,
        ..Default::default()
    })
}

fn updater_module() -> Label {
    custom_module(Custom {
        name: "updater",
        format: "\u{f019} {}",
        exec: "$HOME/.config/hypr/scripts/systemupdate.sh --check",
        interval: 86400,
        on_click: Some("$HOME/.config/hypr/scripts/systemupdate.sh --update"),
        on_click_right: None,
        ..Default::default()
    })
}

pub fn main_module() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 0);
    container.add_css_class("center");
    container.set_halign(Align::Center);

    container.append(&status_module());
    container.append(&tailscale_module());
    container.append(&clock_module());
    container.append(&updater_module());

    container
}