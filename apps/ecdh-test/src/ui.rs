use core::fmt::Write;
use log::info;

use gam::{Gam, Point, Rectangle, DrawStyle, PixelColor, TextView, TextBounds, GlyphStyle, Gid};
use xous::String as XousString;

use x25519_dalek::{PublicKey, StaticSecret};

/// Maximum number of messages to keep in history
const MAX_HISTORY: usize = 20;

pub struct EcdhTestUi {
    gam: Gam,
    token: [u32; 4],
    content_canvas: Gid,
    screensize: Point,
    history: heapless::Vec<XousString<512>, MAX_HISTORY>,
}

impl EcdhTestUi {
    pub fn new(xns: &xous_names::XousNames, gam: &Gam, token: [u32; 4]) -> Self {
        let content_canvas = gam.request_content_canvas(token).expect("couldn't get content canvas");
        let screensize = gam.get_canvas_bounds(content_canvas).expect("couldn't get screen size");

        Self {
            gam: Gam::new(xns).expect("couldn't clone GAM connection"),
            token,
            content_canvas,
            screensize,
            history: heapless::Vec::new(),
        }
    }

    pub fn add_message(&mut self, msg: &str) {
        let mut xstr = XousString::<512>::new();
        write!(xstr, "{}", msg).ok();

        // Circular buffer behavior
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(xstr).ok();
    }

    fn format_hex(bytes: &[u8]) -> XousString<256> {
        let mut result = XousString::new();
        for b in bytes {
            write!(result, "{:02x} ", b).ok();
        }
        result
    }

    fn bytes_to_log_string(bytes: &[u8]) -> XousString<256> {
        let mut s = XousString::new();
        for (i, b) in bytes.iter().enumerate() {
            if i > 0 && i % 16 == 0 {
                write!(s, "\n").ok();
            }
            write!(s, "{:02x} ", b).ok();
        }
        s
    }

    pub fn handle_input(&mut self, input: &str) {
        // Echo input
        let mut echo = XousString::<512>::new();
        write!(echo, ">{}", input).ok();
        self.add_message(echo.as_str().unwrap_or(""));

        let trimmed = input.trim();

        match trimmed {
            "run" => {
                self.cmd_run();
            }
            "clear" => {
                self.history.clear();
                self.add_message("Screen cleared");
            }
            _ => {
                self.add_message("Type 'run' to test ECDH");
            }
        }
    }

    fn cmd_run(&mut self) {
        info!("=== STARTING ECDH TEST ===");
        self.add_message("=== ECDH TEST ===");

        // Get TRNG
        let xns = xous_names::XousNames::new().unwrap();
        let mut trng = trng::Trng::new(&xns).expect("couldn't get TRNG");

        // Generate our keypair
        self.add_message("1. Generating our keypair...");
        let mut our_secret_bytes = [0u8; 32];
        trng.fill_bytes_via_next(&mut our_secret_bytes);
        let our_secret = StaticSecret::from(our_secret_bytes);
        let our_public = PublicKey::from(&our_secret);

        info!("Our private key: {}", Self::bytes_to_log_string(&our_secret_bytes).as_str().unwrap());
        info!("Our public key: {}", Self::bytes_to_log_string(our_public.as_bytes()).as_str().unwrap());

        let mut msg = XousString::<512>::new();
        write!(msg, "Our priv: {}", Self::format_hex(&our_secret_bytes).as_str().unwrap()).ok();
        self.add_message(msg.as_str().unwrap_or(""));

        let mut msg = XousString::<512>::new();
        write!(msg, "Our pub:  {}", Self::format_hex(our_public.as_bytes()).as_str().unwrap()).ok();
        self.add_message(msg.as_str().unwrap_or(""));

        // Generate peer's keypair
        self.add_message("2. Generating peer keypair...");
        let mut peer_secret_bytes = [0u8; 32];
        trng.fill_bytes_via_next(&mut peer_secret_bytes);
        let peer_secret = StaticSecret::from(peer_secret_bytes);
        let peer_public = PublicKey::from(&peer_secret);

        info!("Peer private key: {}", Self::bytes_to_log_string(&peer_secret_bytes).as_str().unwrap());
        info!("Peer public key: {}", Self::bytes_to_log_string(peer_public.as_bytes()).as_str().unwrap());

        let mut msg = XousString::<512>::new();
        write!(msg, "Peer pub: {}", Self::format_hex(peer_public.as_bytes()).as_str().unwrap()).ok();
        self.add_message(msg.as_str().unwrap_or(""));

        // Perform ECDH: our_private * peer_public
        self.add_message("3. Computing ECDH...");
        info!("Computing ECDH: our_private.diffie_hellman(peer_public)");
        info!("  Input private: {}", Self::bytes_to_log_string(&our_secret_bytes).as_str().unwrap());
        info!("  Input public:  {}", Self::bytes_to_log_string(peer_public.as_bytes()).as_str().unwrap());

        let shared_secret = our_secret.diffie_hellman(&peer_public);

        info!("  Output shared: {}", Self::bytes_to_log_string(shared_secret.as_bytes()).as_str().unwrap());

        let mut msg = XousString::<512>::new();
        write!(msg, "Shared:   {}", Self::format_hex(shared_secret.as_bytes()).as_str().unwrap()).ok();
        self.add_message(msg.as_str().unwrap_or(""));

        // Check for the bug
        self.add_message("4. Checking results...");

        if shared_secret.as_bytes() == peer_public.as_bytes() {
            self.add_message("BUG: shared == peer_pub!");
            info!("Shared secret equals peer public key!");
        } else if shared_secret.as_bytes() == our_public.as_bytes() {
            self.add_message("BUG: shared == our_pub!");
            info!("Shared secret equals our public key!");
        } else {
            self.add_message("OK: shared != any pubkey");
            info!("ECDH output looks correct");
        }

        info!("=== ECDH TEST COMPLETE ===");
        self.add_message("=== TEST COMPLETE ===");
    }

    pub fn redraw(&mut self) -> Result<(), xous::Error> {
        // Clear canvas
        self.gam
            .draw_rectangle(
                self.content_canvas,
                Rectangle::new_with_style(
                    Point::new(0, 0),
                    self.screensize,
                    DrawStyle {
                        fill_color: Some(PixelColor::Light),
                        stroke_color: None,
                        stroke_width: 0,
                    },
                ),
            )
            .expect("can't clear canvas");

        // Draw messages from bottom to top
        let margin = 4;
        let line_height = 16;
        let mut y = self.screensize.y - margin;

        for msg in self.history.iter().rev() {
            if let Ok(msg_str) = msg.as_str() {
                let mut tv = TextView::new(
                    self.content_canvas,
                    TextBounds::BoundingBox(Rectangle::new(
                        Point::new(margin, y - line_height),
                        Point::new(self.screensize.x - margin, y),
                    )),
                );
                tv.style = GlyphStyle::Small;
                write!(tv.text, "{}", msg_str).ok();
                self.gam.post_textview(&mut tv).ok();

                y -= line_height;
                if y < 0 {
                    break;
                }
            }
        }

        self.gam.redraw()?;
        Ok(())
    }
}
