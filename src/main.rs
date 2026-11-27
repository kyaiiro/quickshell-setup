mod modules;
use gtk4::prelude::*;
use gtk4::{gdk, Application, ApplicationWindow, CenterBox, CssProvider};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use modules::{left, center};

fn main() -> gtk4::glib::ExitCode {
    let app = Application::builder()
        .application_id("dev.yourname.bar")
        .build();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_bar);
    app.run()
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("../assets/style.css"));
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("no display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_bar(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_height(32)
        .build();
    window.add_css_class("bar");

    window.init_layer_shell();
    window.set_layer(Layer::Top);
    for edge in [Edge::Top, Edge::Left, Edge::Right] {
        window.set_anchor(edge, true);
    }
    window.auto_exclusive_zone_enable();

    let layout = CenterBox::new();
    layout.set_start_widget(Some(&left::main_module()));
    layout.set_center_widget(Some(&center::main_module()));
    window.set_child(Some(&layout));

    window.present();
}