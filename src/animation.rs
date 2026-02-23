use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use image::codecs::png::PngDecoder;
use image::AnimationDecoder;
use std::cell::Cell;
use std::cell::RefCell;
use std::io::Cursor;
use std::time::Duration;

const SPRITE_DATA: &[u8] = include_bytes!("../assets/miku_sprite.png");

struct FrameData {
    textures: Vec<gdk::MemoryTexture>,
    delays: Vec<Duration>,
}

mod imp {
    use super::*;

    pub struct MikuPaintable {
        pub(super) textures: RefCell<Vec<gdk::MemoryTexture>>,
        pub(super) delays: RefCell<Vec<Duration>>,
        pub(super) current_frame: Cell<usize>,
        pub(super) timeout_id: RefCell<Option<glib::SourceId>>,
    }

    impl Default for MikuPaintable {
        fn default() -> Self {
            Self {
                textures: RefCell::new(Vec::new()),
                delays: RefCell::new(Vec::new()),
                current_frame: Cell::new(0),
                timeout_id: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MikuPaintable {
        const NAME: &'static str = "MikuPaintable";
        type Type = super::MikuPaintable;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for MikuPaintable {}

    impl PaintableImpl for MikuPaintable {
        fn intrinsic_width(&self) -> i32 {
            self.textures
                .borrow()
                .first()
                .map(|t| t.width())
                .unwrap_or(240)
        }

        fn intrinsic_height(&self) -> i32 {
            self.textures
                .borrow()
                .first()
                .map(|t| t.height())
                .unwrap_or(60)
        }

        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            let textures = self.textures.borrow();
            let idx = self.current_frame.get();
            if let Some(texture) = textures.get(idx) {
                snapshot.append_texture(
                    texture,
                    &gtk4::graphene::Rect::new(0.0, 0.0, width as f32, height as f32),
                );
            }
        }

        fn flags(&self) -> gdk::PaintableFlags {
            gdk::PaintableFlags::empty()
        }
    }
}

glib::wrapper! {
    pub struct MikuPaintable(ObjectSubclass<imp::MikuPaintable>)
        @implements gdk::Paintable;
}

impl MikuPaintable {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let obj: Self = glib::Object::builder().build();
        let frame_data = decode_apng()?;

        let imp = obj.imp();
        *imp.textures.borrow_mut() = frame_data.textures;
        *imp.delays.borrow_mut() = frame_data.delays;

        Ok(obj)
    }

    pub fn start_animation(&self) {
        let imp = self.imp();
        if imp.timeout_id.borrow().is_some() || imp.textures.borrow().len() <= 1 {
            return;
        }
        self.schedule_next_frame();
    }

    pub fn stop_animation(&self) {
        let imp = self.imp();
        if let Some(id) = imp.timeout_id.borrow_mut().take() {
            id.remove();
        }
    }

    fn schedule_next_frame(&self) {
        let imp = self.imp();
        let delays = imp.delays.borrow();
        let idx = imp.current_frame.get();
        let delay = delays.get(idx).copied().unwrap_or(Duration::from_millis(33));
        drop(delays);

        let paintable = self.clone();
        let id = glib::timeout_add_local_once(delay, move || {
            let imp = paintable.imp();
            let n_frames = imp.textures.borrow().len();
            if n_frames == 0 {
                return;
            }
            let next = (imp.current_frame.get() + 1) % n_frames;
            imp.current_frame.set(next);
            *imp.timeout_id.borrow_mut() = None;
            paintable.invalidate_contents();
            paintable.schedule_next_frame();
        });
        *imp.timeout_id.borrow_mut() = Some(id);
    }

    pub fn width(&self) -> i32 {
        self.imp()
            .textures
            .borrow()
            .first()
            .map(|t| t.width())
            .unwrap_or(240)
    }

    pub fn height(&self) -> i32 {
        self.imp()
            .textures
            .borrow()
            .first()
            .map(|t| t.height())
            .unwrap_or(60)
    }
}

fn decode_apng() -> Result<FrameData, Box<dyn std::error::Error>> {
    let decoder = PngDecoder::new(Cursor::new(SPRITE_DATA))?;
    let apng = decoder.apng()?;

    let mut textures = Vec::new();
    let mut delays = Vec::new();

    for frame in apng.into_frames() {
        let frame: image::Frame = frame?;
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay_ms = if denom == 0 {
            33u32
        } else {
            numer / denom
        };
        let delay = Duration::from_millis(delay_ms.max(10) as u64);

        let rgba_image = frame.into_buffer();
        let width = rgba_image.width();
        let height = rgba_image.height();
        let mut raw = rgba_image.into_raw();

        // Color-key transparency: the Firefox theme background is RGB(32, 39, 47).
        // Pixels close to this color become transparent; semi-close pixels get
        // partial alpha for smooth anti-aliased edges.
        const KEY: [f32; 3] = [32.0, 39.0, 47.0];
        const HARD_THRESHOLD: f32 = 12.0;  // below this: fully transparent
        const SOFT_THRESHOLD: f32 = 35.0;  // between hard and soft: partial alpha

        for pixel in raw.chunks_exact_mut(4) {
            let dist = ((pixel[0] as f32 - KEY[0]).powi(2)
                + (pixel[1] as f32 - KEY[1]).powi(2)
                + (pixel[2] as f32 - KEY[2]).powi(2))
            .sqrt();

            if dist < HARD_THRESHOLD {
                // Fully transparent — zero all channels for clean compositing
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            } else if dist < SOFT_THRESHOLD {
                // Partial alpha for anti-aliased edges
                let t = (dist - HARD_THRESHOLD) / (SOFT_THRESHOLD - HARD_THRESHOLD);
                pixel[3] = (t * 255.0) as u8;
            }
        }

        let bytes = glib::Bytes::from_owned(raw);

        let texture = gdk::MemoryTexture::new(
            width as i32,
            height as i32,
            gdk::MemoryFormat::R8g8b8a8,
            &bytes,
            (width * 4) as usize,
        );
        textures.push(texture);
        delays.push(delay);
    }

    if textures.is_empty() {
        return Err("No frames found in APNG".into());
    }

    eprintln!(
        "Decoded {} frames, delays: {:?}",
        textures.len(),
        &delays[..delays.len().min(5)]
    );

    Ok(FrameData { textures, delays })
}
