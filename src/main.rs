mod animation;
mod overlay;
mod tracker;

use animation::MikuPaintable;
use overlay::Overlay;
use tracker::HeliumGeometry;

use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let app = gtk4::Application::builder()
        .application_id("dev.miku.overlay")
        .build();

    app.connect_activate(|app| {
        // Decode the embedded APNG sprite
        let paintable = match MikuPaintable::new() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to decode embedded sprite: {e}");
                std::process::exit(1);
            }
        };

        // Create the overlay window
        let overlay = Rc::new(Overlay::new(app, &paintable));

        // Shared geometry state
        let geo: Rc<RefCell<Option<HeliumGeometry>>> = Rc::new(RefCell::new(None));

        // Initial lookup: find Helium in the sway tree
        if let Ok(mut conn) = swayipc::Connection::new() {
            if let Some(g) = tracker::find_helium(&mut conn) {
                eprintln!(
                    "Found Helium at ({}, {}) {}x{}",
                    g.x, g.y, g.width, g.height
                );
                overlay.update_position(g.x, g.y, g.width);
                if g.visible && !g.fullscreen {
                    overlay.show();
                }
                *geo.borrow_mut() = Some(g);
            } else {
                eprintln!("Helium not found, waiting for it to appear...");
            }
        }

        // Start animation
        paintable.start_animation();

        // Set up IPC event listener
        let (sender, receiver) = async_channel::unbounded();
        tracker::start_event_listener(sender);
        tracker::handle_events(receiver, overlay.clone(), geo.clone());

        // Start fallback poll
        tracker::start_fallback_poll(overlay.clone(), geo.clone());
    });

    app.run_with_args::<&str>(&[]);
}
