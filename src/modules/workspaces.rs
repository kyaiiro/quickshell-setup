use gtk4::prelude::*;
use gtk4::{
    gio, glib, Box as GtkBox, EventControllerScroll, EventControllerScrollFlags, GestureClick,
    Label, Orientation,
};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;

use crate::hypr;
use crate::modules::custom::run_detached;

const PERSISTENT: &[i32] = &[1, 2, 3, 8, 9];
const DEFAULT_ICON: &str = "\u{f2d0}";
const REWRITE_JSON: &str = include_str!("../../assets/window-rewrite.json");
const SCROLL_UP: &str = r#"hyprctl dispatch 'hl.dsp.focus({ workspace = "e+1" })'"#;
const SCROLL_DOWN: &str = r#"hyprctl dispatch 'hl.dsp.focus({ workspace = "e-1" })'"#;

struct Rule {
    class: Option<Regex>,
    title: Option<Regex>,
    icon: String,
}

fn anchored(pattern: &str) -> Option<Regex> {
    RegexBuilder::new(&format!("^(?:{pattern})$"))
        .case_insensitive(true)
        .build()
        .map_err(|e| eprintln!("bad window-rewrite pattern {pattern:?}: {e}"))
        .ok()
}

fn load_rules() -> Vec<Rule> {
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(REWRITE_JSON)
        .unwrap_or_else(|e| {
            eprintln!("window-rewrite.json: {e}");
            Default::default()
        });

    let key_re = Regex::new(r"^(?:class<(.*?)>)?\s*(?:title<(.*?)>)?$").unwrap();

    let mut rules: Vec<Rule> = map
        .iter()
        .filter_map(|(key, icon)| {
            let caps = key_re.captures(key)?;
            let class = match caps.get(1) {
                Some(m) => Some(anchored(m.as_str())?),
                None => None,
            };
            let title = match caps.get(2) {
                Some(m) => Some(anchored(m.as_str())?),
                None => None,
            };
            if class.is_none() && title.is_none() {
                return None;
            }
            Some(Rule {
                class,
                title,
                icon: icon.as_str()?.trim().to_string(),
            })
        })
        .collect();

    rules.sort_by_key(|r| match (&r.class, &r.title) {
        (Some(_), Some(_)) => 0,
        (None, Some(_)) => 1,
        _ => 2,
    });
    rules
}

fn icon_for<'a>(rules: &'a [Rule], class: &str, title: &str) -> &'a str {
    rules
        .iter()
        .find(|r| {
            r.class.as_ref().map_or(true, |re| re.is_match(class))
                && r.title.as_ref().map_or(true, |re| re.is_match(title))
        })
        .map(|r| r.icon.as_str())
        .unwrap_or(DEFAULT_ICON.trim())
}

#[derive(Deserialize)]
struct IdOnly {
    id: i32,
}

#[derive(Deserialize)]
struct Workspace {
    id: i32,
    #[serde(default)]
    monitor: String,
    #[serde(default)]
    lastwindow: String,
}

#[derive(Deserialize)]
struct Client {
    #[serde(default)]
    address: String,
    workspace: IdOnly,
    #[serde(default)]
    class: String,
    #[serde(default)]
    title: String,
}

#[derive(Deserialize)]
struct MonitorInfo {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "activeWorkspace")]
    active_workspace: IdOnly,
}

#[derive(Deserialize)]
struct WsRule {
    #[serde(rename = "workspaceString", default)]
    workspace: String,
    #[serde(default)]
    monitor: String,
}

struct Snapshot {
    workspaces: Vec<Workspace>,
    clients: Vec<Client>,
    monitors: Vec<MonitorInfo>,
    rules: Vec<WsRule>,
}

fn fetch() -> Option<Snapshot> {
    let workspaces: Vec<Workspace> =
        serde_json::from_str(&hypr::command("j/workspaces")?).ok()?;
    let clients: Vec<Client> = serde_json::from_str(&hypr::command("j/clients")?).ok()?;
    let monitors: Vec<MonitorInfo> =
        serde_json::from_str(&hypr::command("j/monitors")?).ok()?;
    let rules: Vec<WsRule> = hypr::command("j/workspacerules")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    Some(Snapshot { workspaces, clients, monitors, rules })
}

#[derive(PartialEq)]
struct View {
    id: i32,
    text: String,
    active: bool,
    empty: bool,
}

fn build_views(snap: &Snapshot, rules: &[Rule], monitor: &str) -> Vec<View> {
    let mon = snap.monitors.iter().find(|m| m.name == monitor);
    let active = mon.map(|m| m.active_workspace.id);
    let desc = mon.map(|m| format!("desc:{}", m.description));

    let existing = snap
        .workspaces
        .iter()
        .filter(|w| w.monitor == monitor)
        .map(|w| w.id);

    let bound = snap
        .rules
        .iter()
        .filter(|r| r.monitor == monitor || Some(&r.monitor) == desc.as_ref())
        .filter_map(|r| {
            r.workspace
                .strip_prefix("name:")
                .unwrap_or(&r.workspace)
                .parse::<i32>()
                .ok()
        })
        .filter(|id| PERSISTENT.contains(id));

    let mut ids: Vec<i32> = existing.chain(bound).collect();
    ids.retain(|&id| id > 0);
    ids.sort_unstable();
    ids.dedup();

    ids.into_iter()
        .map(|id| {
            let on_ws = || snap.clients.iter().filter(move |c| c.workspace.id == id);
            let last = snap
                .workspaces
                .iter()
                .find(|w| w.id == id)
                .map(|w| w.lastwindow.as_str());

            let main = on_ws()
                .find(|c| Some(c.address.as_str()) == last)
                .or_else(|| on_ws().next());

            let (text, empty) = match main {
                Some(c) => (icon_for(rules, &c.class, &c.title).to_string(), false),
                None => (id.to_string(), true),
            };

            View { id, text, active: Some(id) == active, empty }
        })
        .collect()
}

fn render(container: &GtkBox, views: &[View]) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    for v in views {
        let label = Label::new(Some(&v.text));
        label.add_css_class("workspace");
        label.set_tooltip_text(Some(&format!("Workspace {}", v.id)));
        if v.active {
            label.add_css_class("active");
        }
        if v.empty {
            label.add_css_class("empty");
        }

        let id = v.id;
        let click = GestureClick::new();
        click.connect_released(move |_, _, _, _| {
            let _ = gio::spawn_blocking(move || {
                hypr::command(&format!("dispatch workspace {id}"))
            });
        });
        label.add_controller(click);

        container.append(&label);
    }
}

pub fn workspaces_module(monitor: &str) -> GtkBox {
    let monitor = monitor.to_string();

    let container = GtkBox::new(Orientation::Horizontal, 0);
    container.add_css_class("workspaces");

    let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
    scroll.connect_scroll(|_, _, dy| {
        run_detached(if dy < 0.0 { SCROLL_UP } else { SCROLL_DOWN });
        glib::Propagation::Stop
    });
    container.add_controller(scroll);

    let (tx, rx) = async_channel::unbounded::<Snapshot>();
    std::thread::spawn(move || {
        let Some(events) = hypr::events() else {
            eprintln!("workspaces: can't reach Hyprland (is HYPRLAND_INSTANCE_SIGNATURE set?)");
            return;
        };
        if let Some(s) = fetch() {
            if tx.try_send(s).is_err() {
                return;
            }
        }
        for (name, _) in events {
            if name == "activewindow" {
                continue;
            }
            if let Some(s) = fetch() {
                if tx.try_send(s).is_err() {
                    return;
                }
            }
        }
    });

    let rules = load_rules();
    let c = container.clone();
    glib::spawn_future_local(async move {
        let mut last: Vec<View> = Vec::new();
        while let Ok(mut snap) = rx.recv().await {
            while let Ok(newer) = rx.try_recv() {
                snap = newer;
            }
            let views = build_views(&snap, &rules, &monitor);
            if views != last {
                render(&c, &views);
                last = views;
            }
        }
    });

    container
}