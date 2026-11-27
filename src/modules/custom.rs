use gtk4::ShortcutType::Gesture;
use gtk4::prelude::*;
use gtk4::{gio, glib, Align, GestureClick, Label};
use serde::Deserialize;
use std::process::Command;
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::time::Duration;

#[derive(Clone, Copy)]
pub struct Button {
    pub name: &'static str,
    pub label: &'static str,
    pub tooltip: Option<&'static str>,
    pub on_click: Option<&'static str>,
    pub on_click_middle: Option<&'static str>,
    pub on_click_right: Option<&'static str>,
}

#[derive(Clone, Copy, Default)]
pub struct Custom {
    pub name: &'static str, // CSS class, e.g. "tailscale"
    pub format: &'static str,
    pub exec: &'static str,
    pub interval: u32,
    pub plain: bool,
    pub tooltip_format: Option<&'static str>,
    pub on_click: Option<&'static str>,
    pub on_click_right: Option<&'static str>,
}

#[derive(Deserialize)]
pub struct ScriptOutput {
    #[serde(default)]
    text: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    tooltip: Option<String>,
}

#[derive(Clone, Copy)]
pub struct JsonStream {
    pub name: &'static str,
    pub exec: &'static str,
    pub exec_if: Option<&'static str>,
    pub format: &'static str,
    pub icons: &'static [(&'static str, &'static str)],
    pub tooltip: Option<&'static str>,
    pub on_click: Option<&'static str>,
    pub on_click_middle: Option<&'static str>,
    pub on_click_right: Option<&'static str>,
}

#[derive(Deserialize, Default)]
struct StreamLine {
    #[serde(default)]
    text: String,
    #[serde(default)]
    alt: String,
    #[serde(default)]
    class: serde_json::Value,
}

fn classes_of(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::String(s)if !s.is_empty() => vec![s.clone()],
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

fn spawn_lines(exec: &'static str) -> async_channel::Receiver<String> {
    let (tx, rx) = async_channel::unbounded::<String>();
    std::thread::spawn(move || loop {
        if let Ok(mut child) = Command::new("sh")
            .args(["-c", exec])
            .stdout(Stdio::piped())
            .spawn()
        {
            if let Some(stdout) = child.stdout.take() {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if tx.send_blocking(line).is_err() {
                        return;
                    }
                }
            }
            let _ = child.wait();
        }
        std::thread::sleep(Duration::from_secs(1));
    });
    rx
}

fn attach_clicks(
    label: &Label,
    left: Option<&'static str>,
    middle: Option<&'static str>,
    right: Option<&'static str>,
) {
    let click = GestureClick::new();
    click.set_button(0);
    click.connect_released(move |gesture, _, _, _| {
        let cmd = match gesture.current_button() {
            1 => left,
            2 => middle,
            3 => right,
            _ => None,
        };
        if let Some(cmd) = cmd {
            run_detached(cmd);
        }
    });
    label.add_controller(click);
}


pub fn json_stream_module(cfg: JsonStream) -> Label {
    let label = Label::new(None);
    label.set_use_markup(true);
    label.add_css_class(cfg.name);
    label.set_valign(Align::Center);
    label.set_tooltip_text(cfg.tooltip);

    if let Some(check) = cfg.exec_if {
        let ok = Command::new("sh")
            .args(["-c", check])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            label.set_visible(false);
            return label;
        }
    }

    let rx = spawn_lines(cfg.exec);
    let l = label.clone();
    glib::spawn_future_local(async move {
        while let Ok(mut line) = rx.recv().await {
            while let Ok(newer) = rx.try_recv() {
                line = newer;
            }
            let Ok(out) = serde_json::from_str::<StreamLine>(&line) else {
                continue;
            };

            let icon = cfg
                .icons
                .iter()
                .find(|(alt, _)| *alt == out.alt)
                .map(|(_, icon)| *icon)
                .unwrap_or("");
            let text = glib::markup_escape_text(&out.text); // "escape": true
            l.set_markup(&cfg.format.replace("{icon}", icon).replace("{}", &text));

            let classes = classes_of(&out.class);
            let mut all = vec![cfg.name];
            all.extend(classes.iter().map(|s| s.as_str()));
            l.set_css_classes(&all);
        }
    });

    attach_clicks(&label, cfg.on_click, cfg.on_click_middle, cfg.on_click_right);

    label
}

fn run_script(cmd: &str, plain: bool) -> Option<ScriptOutput> {
    let out = Command::new("sh").args(["-c", cmd]).output().ok()?;
    if plain {
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Some(ScriptOutput { text, class: String::new(), tooltip: None })
    } else {
        serde_json::from_slice(&out.stdout).ok()
    }
}

pub fn run_detached(cmd: &'static str) {
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
            if let Ok(Some(out)) =
                gio::spawn_blocking(move || run_script(cfg.exec, cfg.plain)).await
            {
                l.set_text(&cfg.format.replace("{}", &out.text));

                if out.class.is_empty() {
                    l.set_css_classes(&[cfg.name]);
                } else {
                    l.set_css_classes(&[cfg.name, out.class.as_str()]);
                }

                let tip = match cfg.tooltip_format {
                    Some(f) => Some(f.replace("{}", &out.text)),
                    None => out.tooltip,
                };
                l.set_tooltip_text(tip.as_deref());
            }
            glib::timeout_future_seconds(cfg.interval.max(1)).await;
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

pub fn button_module(cfg: Button) -> Label {
    let label = Label::new(Some(cfg.label));
    label.add_css_class(cfg.name);
    label.set_valign(Align::Center);
    label.set_tooltip_text(cfg.tooltip);

    let click = GestureClick::new();
    click.set_button(0); // listen to every button, then match on which one fired
    click.connect_released(move |gesture, _, _, _| {
        let cmd = match gesture.current_button() {
            1 => cfg.on_click,
            2 => cfg.on_click_middle,
            3 => cfg.on_click_right,
            _ => None,
        };
        if let Some(cmd) = cmd {
            run_detached(cmd);
        }
    });
    label.add_controller(click);

    label
}

pub fn stream_module(name: &'static str, exec: &'static str) -> Label {
    let label = Label::new(None);
    label.add_css_class(name);
    label.set_valign(Align::Center);

    // Capacity 1 + try_send: if GTK falls behind, skip frames instead of
    // building up a backlog (which would show as lag in the visualizer)
    let (tx, rx) = async_channel::bounded::<String>(1);

    std::thread::spawn(move || loop {
        let child = Command::new("sh")
            .args(["-c", exec])
            .stdout(Stdio::piped())
            .spawn();

        if let Ok(mut child) = child {
            if let Some(stdout) = child.stdout.take() {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Err(async_channel::TrySendError::Closed(_)) = tx.try_send(line) {
                        return; // the bar is gone, stop the thread
                    }
                }
            }
            let _ = child.wait();
        }

        // Script exited or failed to start: wait a bit, then restart it
        std::thread::sleep(Duration::from_secs(1));
    });

    let l = label.clone();
    glib::spawn_future_local(async move {
        while let Ok(line) = rx.recv().await {
            l.set_text(&line);
        }
    });

    label
}