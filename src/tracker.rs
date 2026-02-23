use crate::overlay::Overlay;
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;
use swayipc::{Connection, Event, EventType, Node, NodeType, WindowChange};

const TARGET_APP_ID: &str = "helium";

#[derive(Clone, Debug, Default)]
pub struct HeliumGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fullscreen: bool,
    pub visible: bool,
}

pub fn find_helium(conn: &mut Connection) -> Option<HeliumGeometry> {
    let tree = conn.get_tree().ok()?;
    // Collect all Helium windows, prefer the visible one
    let mut all = Vec::new();
    collect_helium_nodes(&tree, &mut all);
    all.into_iter()
        .max_by_key(|g| g.visible as u8)
}

fn collect_helium_nodes(node: &Node, results: &mut Vec<HeliumGeometry>) {
    if matches!(node.node_type, NodeType::Con | NodeType::FloatingCon) {
        if let Some(ref app_id) = node.app_id {
            if app_id == TARGET_APP_ID {
                let rect = &node.rect;
                results.push(HeliumGeometry {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    fullscreen: node.fullscreen_mode.map(|m| m > 0).unwrap_or(false),
                    visible: node.visible.unwrap_or(false),
                });
            }
        }
    }
    for child in node.nodes.iter().chain(node.floating_nodes.iter()) {
        collect_helium_nodes(child, results);
    }
}

pub fn start_event_listener(sender: async_channel::Sender<Event>) {
    std::thread::spawn(move || {
        let subs = [EventType::Window, EventType::Workspace];
        let conn = match Connection::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to connect to sway IPC: {}", e);
                return;
            }
        };
        let events = match conn.subscribe(subs) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to subscribe to sway events: {}", e);
                return;
            }
        };
        for event in events {
            match event {
                Ok(ev) => {
                    if sender.send_blocking(ev).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Sway IPC event error: {}", e);
                    break;
                }
            }
        }
    });
}

pub fn handle_events(
    receiver: async_channel::Receiver<Event>,
    overlay: Rc<Overlay>,
    geo: Rc<RefCell<Option<HeliumGeometry>>>,
) {
    glib::spawn_future_local(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                Event::Window(w) => handle_window_event(&w, &overlay, &geo),
                Event::Workspace(_) => {
                    refresh_from_tree(&overlay, &geo);
                }
                _ => {}
            }
        }
    });
}

fn refresh_from_tree(overlay: &Rc<Overlay>, geo: &Rc<RefCell<Option<HeliumGeometry>>>) {
    let overlay = overlay.clone();
    let geo = geo.clone();
    let (tx, rx) = async_channel::bounded::<Option<HeliumGeometry>>(1);

    std::thread::spawn(move || {
        let result = Connection::new().ok().and_then(|mut c| find_helium(&mut c));
        let _ = tx.send_blocking(result);
    });

    glib::spawn_future_local(async move {
        if let Ok(new_geo) = rx.recv().await {
            apply_geometry(&overlay, &geo, new_geo);
        }
    });
}

fn apply_geometry(
    overlay: &Overlay,
    geo: &RefCell<Option<HeliumGeometry>>,
    new_geo: Option<HeliumGeometry>,
) {
    match new_geo {
        Some(g) if g.visible && !g.fullscreen => {
            overlay.update_position(g.x, g.y, g.width);
            overlay.show();
            *geo.borrow_mut() = Some(g);
        }
        Some(g) => {
            overlay.hide();
            *geo.borrow_mut() = Some(g);
        }
        None => {
            overlay.hide();
            *geo.borrow_mut() = None;
        }
    }
}

fn handle_window_event(
    event: &swayipc::WindowEvent,
    overlay: &Rc<Overlay>,
    geo: &Rc<RefCell<Option<HeliumGeometry>>>,
) {
    let is_helium = event
        .container
        .app_id
        .as_deref()
        .is_some_and(|id| id == TARGET_APP_ID);

    match event.change {
        WindowChange::New | WindowChange::Move | WindowChange::Focus => {
            if is_helium {
                let rect = &event.container.rect;
                let fullscreen = event
                    .container
                    .fullscreen_mode
                    .map(|m| m > 0)
                    .unwrap_or(false);
                let visible = event.container.visible.unwrap_or(true);
                let g = HeliumGeometry {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    fullscreen,
                    visible,
                };
                if visible && !fullscreen {
                    overlay.update_position(g.x, g.y, g.width);
                    overlay.show();
                } else {
                    overlay.hide();
                }
                *geo.borrow_mut() = Some(g);
            } else if event.change == WindowChange::Focus {
                // Another window gained focus — re-check Helium visibility
                refresh_from_tree(overlay, geo);
            }
        }
        WindowChange::Close => {
            if is_helium {
                overlay.hide();
                *geo.borrow_mut() = None;
            }
        }
        WindowChange::FullscreenMode => {
            if is_helium {
                let fullscreen = event
                    .container
                    .fullscreen_mode
                    .map(|m| m > 0)
                    .unwrap_or(false);
                if fullscreen {
                    overlay.hide();
                } else {
                    let rect = &event.container.rect;
                    overlay.update_position(rect.x, rect.y, rect.width);
                    overlay.show();
                }
                if let Some(ref mut g) = *geo.borrow_mut() {
                    g.fullscreen = fullscreen;
                }
            }
        }
        _ => {
            if is_helium {
                let rect = &event.container.rect;
                if let Some(ref mut g) = *geo.borrow_mut() {
                    g.x = rect.x;
                    g.y = rect.y;
                    g.width = rect.width;
                    g.height = rect.height;
                    if !g.fullscreen && g.visible {
                        overlay.update_position(g.x, g.y, g.width);
                    }
                }
            }
        }
    }
}

pub fn start_fallback_poll(overlay: Rc<Overlay>, geo: Rc<RefCell<Option<HeliumGeometry>>>) {
    glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
        let overlay = overlay.clone();
        let geo = geo.clone();

        let (tx, rx) = async_channel::bounded::<Option<HeliumGeometry>>(1);

        std::thread::spawn(move || {
            let result = Connection::new().ok().and_then(|mut c| find_helium(&mut c));
            let _ = tx.send_blocking(result);
        });

        glib::spawn_future_local(async move {
            if let Ok(new_geo) = rx.recv().await {
                apply_geometry(&overlay, &geo, new_geo);
            }
        });

        glib::ControlFlow::Continue
    });
}
