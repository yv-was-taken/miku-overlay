use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::CssProvider;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::animation::MikuPaintable;

pub struct Overlay {
    window: gtk4::ApplicationWindow,
    sprite_width: i32,
}

impl Overlay {
    pub fn new(app: &gtk4::Application, paintable: &MikuPaintable) -> Self {
        let window = gtk4::ApplicationWindow::new(app);

        // Scale down slightly to fit the Helium toolbar
        const SCALE: f64 = 0.75;
        let sprite_width = (paintable.width() as f64 * SCALE) as i32;
        let sprite_height = (paintable.height() as f64 * SCALE) as i32;

        // Init layer shell before realization
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Top, true);
        window.set_exclusive_zone(-1);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_can_focus(false);
        window.set_focusable(false);

        window.set_default_size(sprite_width, sprite_height);

        // Add the sprite picture — allow shrinking so it scales to the window size
        let picture = gtk4::Picture::for_paintable(paintable);
        picture.set_can_shrink(true);
        picture.set_size_request(sprite_width, sprite_height);
        window.set_child(Some(&picture));

        // Transparent background via CSS
        let css_provider = CssProvider::new();
        css_provider.load_from_data("window { background: unset; }");
        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Could not get default display"),
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Click-through: set empty input region on realize
        window.connect_realize(|win| {
            if let Some(surface) = win.surface() {
                let region = gtk4::cairo::Region::create();
                surface.set_input_region(&region);
            }
        });

        Overlay {
            window,
            sprite_width,
        }
    }

    pub fn update_position(&self, helium_x: i32, helium_y: i32, helium_width: i32) {
        const RIGHT_INSET: i32 = 100;

        let x = helium_x + helium_width - self.sprite_width - RIGHT_INSET;
        let y = helium_y;
        self.window.set_margin(Edge::Left, x);
        self.window.set_margin(Edge::Top, y);
    }

    pub fn show(&self) {
        self.window.set_visible(true);
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }
}
