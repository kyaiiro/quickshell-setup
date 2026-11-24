use gtk4::prelude::*;
use gtk4::{gio, glib, Align, Box as GtkBox, GestureClick, Label, Orientation};
use serde::Deserialize;
use std::process::Command;

#[derive(Clone, Copy)]
pub struct Custom {
    pub name: &'static str, // CSS class, e.g. "tailscale"
    pub format: &'static str,
    pub exec: &'static str,
    pub interval: u32,
    pub on_click: Option<&'static str>,
    pub on_click_right: Option<&'static str>,
}

#[derive(Deserialize)]
struct ScriptOutput {
    #[serde(default)]
    text: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    tooltip: Option<String>,
}

fn run_script(cmd: &str) -> Option<ScriptOutput> {
    let out = Command::new("sh").args(["-c", cmd]).output().ok()?;
    serde_json::from_slice(&out.stdout).ok()
}

fn run_detached(cmd: &'static str) {
    let _ = gio::spawn_blocking(move || {
        let _ = Command::new("sh").args(["-c", cmd]).status();
    });
}

pub fn custom_module(cfg: Custom) -> Label {
    let label = Label::new(Some(&cfg.format.replace("{}", "")));
    label.set_css_classes(&[cfg.name]);
    label.set_valign(Align::Center);

    let l = label.clone();
    glib::spawn_future_local(async move {
        loop {
            if let Ok(Some(out)) = gio::spawn_blocking(move || run_script(cfg.exec)).await {
                l.set_text(&cfg.format.replace("{}", &out.text));

                if out.class.is_empty() {
                    l.set_css_classes(&[cfg.name]);
                } else {
                    l.set_css_classes(&[cfg.name, out.class.as_str()]);
                }
                l.set_tooltip_text(out.tooltip.as_deref());
            }
            glib::timeout_future_seconds(cfg.interval).await;
        }
    });

    let click = GestureClick::new();
    click.set_button(0);
    click.connect_released(move |gesture, _, _, _| match gesture.current_button() {
        1 => cfg.on_click.map(run_detached).unwrap_or(()),
        3 => cfg.on_click_right.map(run_detached).unwrap_or(()),
        _ => {}
    });
    label.add_controller(click);

    label
}

fn status_module() -> Label {
    custom_module(Custom {
        name: "status",
        format: "",
        exec: "$HOME/.config/hypr/scripts/server.sh",
        interval: 5,
        on_click: Some("sh $HOME/.config/hypr/scripts/server-toggle.sh"),
        on_click_right: Some("alacritty -e ssh $(qp -r '.server_ip' $HOME/.config/settings.json)")
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