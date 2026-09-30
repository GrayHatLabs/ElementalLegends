// Software framebuffer, sprites and the built-in bitmap font.
//
// Two coordinate spaces:
//  * logic space: the game's world units (a tile is 16 units; a classic screen is 256x240).
//  * screen space: the 320x240 framebuffer.
// The Screen maps logic -> screen through a camera + zoom (world mode: 1.5x, so a tile is
// drawn 24 real pixels wide) or a fixed centring offset (UI mode: 1x, 256-wide layouts
// centred on the 320-wide screen). Text and `*_hd` sprites are always drawn pixel-exact in
// screen pixels at their transformed position.

/// Classic logical view size: menu layouts and one standard room.
pub const W: i32 = 256;
pub const H: i32 = 240;
/// Framebuffer size (fills the 640x480 handheld screen at exactly 2x).
pub const SW: i32 = 320;
pub const SH: i32 = 240;
/// World zoom: 16-unit tiles are drawn 24 pixels wide.
pub const ZOOM: f32 = 1.5;
/// Screen height of the HUD band above the play field.
pub const HUD_PX: i32 = 32;
pub const BLACK: u32 = 0xFF00_0000;
pub const WHITE: u32 = 0xFFFC_FCFC;

pub const fn rgb(v: u32) -> u32 {
    0xFF00_0000 | v
}

pub fn mix(dst: u32, src: u32, a: f32) -> u32 {
    let a = a.clamp(0.0, 1.0);
    let ch = |s: u32| {
        let d = ((dst >> s) & 0xFF) as f32;
        let c = ((src >> s) & 0xFF) as f32;
        (d + (c - d) * a) as u32
    };
    0xFF00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// An image; a pixel value of 0 is transparent.
#[derive(Clone)]
pub struct Sprite {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
}

impl Sprite {
    pub fn new(w: i32, h: i32) -> Self {
        Sprite { w, h, px: vec![0; (w * h) as usize] }
    }

    pub fn from_rows(rows: &[&str], pal: impl Fn(char) -> u32) -> Self {
        let h = rows.len() as i32;
        let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
        let mut s = Sprite::new(w, h);
        for (y, r) in rows.iter().enumerate() {
            for (x, ch) in r.chars().enumerate() {
                if ch == '.' || ch == ' ' {
                    continue;
                }
                s.px[y * w as usize + x] = pal(ch);
            }
        }
        s
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.w), (y + h).min(self.h));
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.px[(yy * self.w + xx) as usize] = c;
            }
        }
    }

    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r + r {
                    self.fill(cx + x, cy + y, 1, 1, c);
                }
            }
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32, a: f32) {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.w), (y + h).min(self.h));
        for yy in y0..y1 {
            for xx in x0..x1 {
                let i = (yy * self.w + xx) as usize;
                self.px[i] = mix(self.px[i], c, a);
            }
        }
    }

    pub fn draw(&mut self, src: &Sprite, x: i32, y: i32) {
        for sy in 0..src.h {
            for sx in 0..src.w {
                let c = src.px[(sy * src.w + sx) as usize];
                let (dx, dy) = (x + sx, y + sy);
                if c == 0 || dx < 0 || dy < 0 || dx >= self.w || dy >= self.h {
                    continue;
                }
                self.px[(dy * self.w + dx) as usize] = c;
            }
        }
    }
}

/// Colour treatment when drawing native sprites.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tint {
    None,
    /// Every opaque pixel becomes this colour (hit flash).
    Solid(u32),
    /// Blend toward a colour by an amount (frost, poison).
    Mix(u32, f32),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
}

pub struct Screen {
    pub px: Vec<u32>,
    clip: [i32; 4],
    /// Clip restored by `reset_clip` (the play field while drawing the world).
    base: [i32; 4],
    /// Logic-space offset (screen shake, cinematic pans), applied before the camera.
    pub ox: i32,
    pub oy: i32,
    pub zoom: f32,
    pub cam_x: f32,
    pub cam_y: f32,
    /// Screen-space offset applied after scaling.
    pub sx: i32,
    pub sy: i32,
}

impl Screen {
    pub fn new() -> Self {
        let mut s = Screen {
            px: vec![BLACK; (SW * SH) as usize],
            clip: [0, 0, SW, SH],
            base: [0, 0, SW, SH],
            ox: 0,
            oy: 0,
            zoom: 1.0,
            cam_x: 0.0,
            cam_y: 0.0,
            sx: 0,
            sy: 0,
        };
        s.ui();
        s
    }

    pub fn clear(&mut self) {
        self.px.iter_mut().for_each(|p| *p = BLACK);
    }

    /// UI mode: 1:1 logic pixels, the 256-wide layout centred on the screen.
    pub fn ui(&mut self) {
        self.zoom = 1.0;
        self.cam_x = 0.0;
        self.cam_y = 0.0;
        self.sx = (SW - W) / 2;
        self.sy = 0;
    }

    /// World mode: logic point (cam_x, cam_y) appears at the top-left of the play field.
    pub fn world(&mut self, cam_x: f32, cam_y: f32) {
        self.zoom = ZOOM;
        self.cam_x = cam_x;
        self.cam_y = cam_y;
        self.sx = 0;
        self.sy = HUD_PX;
    }

    #[inline]
    pub fn tx(&self, x: f32) -> i32 {
        ((x + self.ox as f32 - self.cam_x) * self.zoom).floor() as i32 + self.sx
    }
    #[inline]
    pub fn ty(&self, y: f32) -> i32 {
        ((y + self.oy as f32 - self.cam_y) * self.zoom).floor() as i32 + self.sy
    }
    /// Screen pixel -> logic position (exact inverse of tx/ty).
    pub fn to_world(&self, px: i32, py: i32) -> (f32, f32) {
        (
            (px - self.sx) as f32 / self.zoom + self.cam_x - self.ox as f32,
            (py - self.sy) as f32 / self.zoom + self.cam_y - self.oy as f32,
        )
    }

    /// Clip to a logic-space rectangle (transformed like everything else).
    pub fn clip(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (a, b, c, d) = (self.tx(x0 as f32), self.ty(y0 as f32), self.tx(x1 as f32), self.ty(y1 as f32));
        self.clip_screen(a, b, c, d);
    }
    /// Clip to a screen-space rectangle.
    pub fn clip_screen(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.clip = [x0.max(0), y0.max(0), x1.min(SW), y1.min(SH)];
    }
    /// Remember the current clip as the one `reset_clip` goes back to.
    pub fn set_base_clip(&mut self) {
        self.base = self.clip;
    }
    pub fn reset_clip(&mut self) {
        self.clip = self.base;
    }

    pub fn unclip(&mut self) {
        self.clip = [0, 0, SW, SH];
        self.base = self.clip;
    }

    #[inline]
    pub fn pset(&mut self, x: i32, y: i32, c: u32) {
        self.fill(x, y, 1, 1, c);
    }

    /// Screen rectangle covered by a logic rectangle, clipped.
    #[inline]
    fn span(&self, x: i32, y: i32, w: i32, h: i32) -> (i32, i32, i32, i32) {
        let (x0, y0) = (self.tx(x as f32).max(self.clip[0]), self.ty(y as f32).max(self.clip[1]));
        let (x1, y1) = (self.tx((x + w) as f32).min(self.clip[2]), self.ty((y + h) as f32).min(self.clip[3]));
        (x0, y0, x1, y1)
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let (x0, y0, x1, y1) = self.span(x, y, w, h);
        self.fill_px(x0, y0, x1, y1, c);
    }

    /// Fill a screen-space rectangle (x1/y1 exclusive), respecting the clip.
    fn fill_px(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        let (x0, y0) = (x0.max(self.clip[0]), y0.max(self.clip[1]));
        let (x1, y1) = (x1.min(self.clip[2]), y1.min(self.clip[3]));
        for yy in y0..y1 {
            let row = (yy * SW) as usize;
            for xx in x0..x1 {
                self.px[row + xx as usize] = c;
            }
        }
    }

    /// Fill a screen-space rectangle ignoring the camera (full-screen fades, bars).
    pub fn fill_screen(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        self.fill_px(x, y, x + w, y + h, c);
    }
    pub fn blend_screen(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32, a: f32) {
        let (x0, y0) = (x.max(self.clip[0]), y.max(self.clip[1]));
        let (x1, y1) = ((x + w).min(self.clip[2]), (y + h).min(self.clip[3]));
        self.blend_px(x0, y0, x1, y1, c, a);
    }

    fn blend_px(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32, a: f32) {
        for yy in y0..y1 {
            let row = (yy * SW) as usize;
            for xx in x0..x1 {
                let i = row + xx as usize;
                self.px[i] = mix(self.px[i], c, a);
            }
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32, a: f32) {
        let (x0, y0, x1, y1) = self.span(x, y, w, h);
        self.blend_px(x0, y0, x1, y1, c, a);
    }

    pub fn frame_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        self.fill(x, y, w, 1, c);
        self.fill(x, y + h - 1, w, 1, c);
        self.fill(x, y, 1, h, c);
        self.fill(x + w - 1, y, 1, h, c);
    }

    /// Draw a logic-resolution sprite (each of its pixels is one logic unit, so it is
    /// scaled with the world). Uses inverse mapping, so it is exact and fast at any zoom.
    pub fn blit(&mut self, s: &Sprite, x: i32, y: i32, flip: bool, white: bool) {
        let (x0, y0, x1, y1) = self.span(x, y, s.w, s.h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let inv = 1.0 / self.zoom;
        let bx = self.cam_x - self.ox as f32 - x as f32;
        let by = self.cam_y - self.oy as f32 - y as f32;
        // Source column for every destination column, computed once.
        let cols: Vec<i32> = (x0..x1)
            .map(|dx| {
                let u = (((dx - self.sx) as f32 + 0.5) * inv + bx).floor() as i32;
                let u = u.clamp(0, s.w - 1);
                if flip {
                    s.w - 1 - u
                } else {
                    u
                }
            })
            .collect();
        for dy in y0..y1 {
            let v = ((((dy - self.sy) as f32 + 0.5) * inv + by).floor() as i32).clamp(0, s.h - 1);
            let srow = (v * s.w) as usize;
            let drow = (dy * SW) as usize;
            for (i, dx) in (x0..x1).enumerate() {
                let c = s.px[srow + cols[i] as usize];
                if c != 0 {
                    self.px[drow + dx as usize] = if white { WHITE } else { c };
                }
            }
        }
    }

    /// Draw a screen-resolution sprite (native art) unscaled, top-left at a logic position.
    pub fn blit_hd(&mut self, s: &Sprite, x: f32, y: f32, flip: bool) {
        let (dx0, dy0) = (self.tx(x), self.ty(y));
        self.blit_px(s, dx0, dy0, flip, None);
    }

    /// Native sprite centred on a logic position.
    pub fn spr_hd(&mut self, s: &Sprite, cx: f32, cy: f32, flip: bool) {
        let (dx0, dy0) = (self.tx(cx) - s.w / 2, self.ty(cy) - s.h / 2);
        self.blit_px(s, dx0, dy0, flip, None);
    }

    /// Native sprite anchored by a point inside it: pixel (ax, ay) of the sprite lands on
    /// logic position (x, y). Used for characters whose feet sit at their position.
    pub fn spr_hd_anchor(&mut self, s: &Sprite, x: f32, y: f32, ax: i32, ay: i32, flip: bool, fx: Tint) {
        let ax = if flip { s.w - 1 - ax } else { ax };
        let (dx0, dy0) = (self.tx(x) - ax, self.ty(y) - ay);
        self.blit_fx(s, dx0, dy0, flip, fx);
    }

    fn blit_px(&mut self, s: &Sprite, dx0: i32, dy0: i32, flip: bool, tint: Option<u32>) {
        self.blit_fx(s, dx0, dy0, flip, tint.map_or(Tint::None, Tint::Solid));
    }

    fn blit_fx(&mut self, s: &Sprite, dx0: i32, dy0: i32, flip: bool, fx: Tint) {
        let (x0, y0) = (dx0.max(self.clip[0]), dy0.max(self.clip[1]));
        let (x1, y1) = ((dx0 + s.w).min(self.clip[2]), (dy0 + s.h).min(self.clip[3]));
        for dy in y0..y1 {
            let srow = ((dy - dy0) * s.w) as usize;
            let drow = (dy * SW) as usize;
            for dx in x0..x1 {
                let u = dx - dx0;
                let u = if flip { s.w - 1 - u } else { u };
                let c = s.px[srow + u as usize];
                if c != 0 {
                    self.px[drow + dx as usize] = match fx {
                        Tint::None => c,
                        Tint::Solid(t) => t,
                        Tint::Mix(t, a) => mix(c, t, a),
                    };
                }
            }
        }
    }

    /// Draw a sprite centred on (cx, cy).
    pub fn spr(&mut self, s: &Sprite, cx: f32, cy: f32, flip: bool) {
        let x = (cx - s.w as f32 / 2.0).round() as i32;
        let y = (cy - s.h as f32 / 2.0).round() as i32;
        self.blit(s, x, y, flip, false);
    }

    pub fn blit_scaled(&mut self, s: &Sprite, x: i32, y: i32, sc: i32) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                let c = s.px[(sy * s.w + sx) as usize];
                if c != 0 {
                    self.fill(x + sx * sc, y + sy * sc, sc, sc, c);
                }
            }
        }
    }

    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r + r {
                    self.pset(cx + x, cy + y, c);
                }
            }
        }
    }

    /// Darken a screen-space band by a light map: `grid` holds darkness (0..1) at the
    /// corners of `cell`-sized squares starting at screen row `y0`, bilinearly
    /// interpolated per pixel and mixed toward `tint`. Used for dungeon lighting.
    pub fn light_map(&mut self, y0: i32, grid: &[f32], gw: usize, gh: usize, cell: i32, tint: u32) {
        let (tr, tg, tb) = (((tint >> 16) & 255) as f32, ((tint >> 8) & 255) as f32, (tint & 255) as f32);
        let rows = ((gh - 1) as i32 * cell).min(SH - y0);
        for py in 0..rows {
            let sy = py + y0;
            if sy < self.clip[1] || sy >= self.clip[3] {
                continue;
            }
            let gy = (py / cell) as usize;
            let fy = (py % cell) as f32 / cell as f32;
            let row = (sy * SW) as usize;
            for px in 0..SW.min((gw - 1) as i32 * cell) {
                let sx = px;
                if sx < self.clip[0] || sx >= self.clip[2] {
                    continue;
                }
                let gx = (px / cell) as usize;
                let fx = (px % cell) as f32 / cell as f32;
                let i = gy * gw + gx;
                let top = grid[i] + (grid[i + 1] - grid[i]) * fx;
                let bot = grid[i + gw] + (grid[i + gw + 1] - grid[i + gw]) * fx;
                let a = top + (bot - top) * fy;
                if a <= 0.01 {
                    continue;
                }
                let p = &mut self.px[row + sx as usize];
                let c = *p;
                let (r, g, b) = (((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32);
                let (r, g, b) = (r + (tr - r) * a, g + (tg - g) * a, b + (tb - b) * a);
                *p = 0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
            }
        }
    }

    /// Translucent filled circle (glows, halos, overlays).
    pub fn blend_disc(&mut self, cx: i32, cy: i32, r: i32, c: u32, a: f32) {
        for y in -r..=r {
            let half = ((r * r + r - y * y).max(0) as f32).sqrt() as i32;
            self.blend(cx - half, cy + y, half * 2 + 1, 1, c, a);
        }
    }

    /// Translucent ellipse, e.g. shadows and ground markers.
    pub fn blend_ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32, a: f32) {
        let ry = ry.max(1);
        for y in -ry..=ry {
            let f = 1.0 - (y * y) as f32 / (ry * ry) as f32;
            let half = (rx as f32 * f.max(0.0).sqrt()) as i32;
            self.blend(cx - half, cy + y, half * 2 + 1, 1, c, a);
        }
    }

    pub fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
        let steps = ((rx.max(ry)) * 6).max(12);
        for i in 0..steps {
            let a = i as f32 / steps as f32 * std::f32::consts::TAU;
            self.pset(cx + (a.cos() * rx as f32).round() as i32, cy + (a.sin() * ry as f32).round() as i32, c);
        }
    }

    pub fn ring(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        let (mut x, mut y, mut d) = (r, 0, 1 - r);
        while x >= y {
            for (px, py) in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)] {
                self.pset(cx + px, cy + py, c);
            }
            y += 1;
            if d < 0 {
                d += 2 * y + 1;
            } else {
                x -= 1;
                d += 2 * (y - x) + 1;
            }
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.pset(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Text with a drop shadow. `size` is 8, 16 or 32 (pixels per character cell).
    /// The anchor is transformed like any logic point; the glyphs are drawn pixel-exact.
    pub fn text(&mut self, s: &str, x: i32, y: i32, col: u32, align: Align, size: i32) {
        let sc = (size / 8).max(1);
        let tw = s.chars().count() as i32 * 8 * sc;
        let (x, y) = (self.tx(x as f32), self.ty(y as f32));
        let x = if align == Align::Center { x - tw / 2 } else { x };
        self.text_raw(s, x + 1, y + 1, BLACK, sc);
        self.text_raw(s, x, y, col, sc);
    }

    fn text_raw(&mut self, s: &str, x: i32, y: i32, col: u32, sc: i32) {
        for (i, ch) in s.chars().enumerate() {
            let Some(g) = glyph(ch) else { continue };
            let cx = x + i as i32 * 8 * sc;
            for (gy, row) in g.iter().enumerate() {
                for (gx, b) in row.bytes().enumerate() {
                    if b == b'#' {
                        let (px, py) = (cx + (gx as i32 + 1) * sc, y + gy as i32 * sc);
                        self.fill_px(px, py, px + sc, py + sc, col);
                    }
                }
            }
        }
    }
}

fn glyph(c: char) -> Option<&'static [&'static str; 7]> {
    Some(match c.to_ascii_uppercase() {
        'A' => &[" ### ", "#   #", "#   #", "#####", "#   #", "#   #", "#   #"],
        'B' => &["#### ", "#   #", "#   #", "#### ", "#   #", "#   #", "#### "],
        'C' => &[" ### ", "#   #", "#    ", "#    ", "#    ", "#   #", " ### "],
        'D' => &["#### ", "#   #", "#   #", "#   #", "#   #", "#   #", "#### "],
        'E' => &["#####", "#    ", "#    ", "#### ", "#    ", "#    ", "#####"],
        'F' => &["#####", "#    ", "#    ", "#### ", "#    ", "#    ", "#    "],
        'G' => &[" ### ", "#   #", "#    ", "# ###", "#   #", "#   #", " ####"],
        'H' => &["#   #", "#   #", "#   #", "#####", "#   #", "#   #", "#   #"],
        'I' => &[" ### ", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", " ### "],
        'J' => &["  ###", "   # ", "   # ", "   # ", "   # ", "#  # ", " ##  "],
        'K' => &["#   #", "#  # ", "# #  ", "##   ", "# #  ", "#  # ", "#   #"],
        'L' => &["#    ", "#    ", "#    ", "#    ", "#    ", "#    ", "#####"],
        'M' => &["#   #", "## ##", "# # #", "# # #", "#   #", "#   #", "#   #"],
        'N' => &["#   #", "##  #", "# # #", "#  ##", "#   #", "#   #", "#   #"],
        'O' => &[" ### ", "#   #", "#   #", "#   #", "#   #", "#   #", " ### "],
        'P' => &["#### ", "#   #", "#   #", "#### ", "#    ", "#    ", "#    "],
        'Q' => &[" ### ", "#   #", "#   #", "#   #", "# # #", "#  # ", " ## #"],
        'R' => &["#### ", "#   #", "#   #", "#### ", "# #  ", "#  # ", "#   #"],
        'S' => &[" ####", "#    ", "#    ", " ### ", "    #", "    #", "#### "],
        'T' => &["#####", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "  #  "],
        'U' => &["#   #", "#   #", "#   #", "#   #", "#   #", "#   #", " ### "],
        'V' => &["#   #", "#   #", "#   #", "#   #", "#   #", " # # ", "  #  "],
        'W' => &["#   #", "#   #", "#   #", "# # #", "# # #", "## ##", "#   #"],
        'X' => &["#   #", "#   #", " # # ", "  #  ", " # # ", "#   #", "#   #"],
        'Y' => &["#   #", "#   #", " # # ", "  #  ", "  #  ", "  #  ", "  #  "],
        'Z' => &["#####", "    #", "   # ", "  #  ", " #   ", "#    ", "#####"],
        '0' => &[" ### ", "#   #", "#  ##", "# # #", "##  #", "#   #", " ### "],
        '1' => &["  #  ", " ##  ", "  #  ", "  #  ", "  #  ", "  #  ", " ### "],
        '2' => &[" ### ", "#   #", "    #", "   # ", "  #  ", " #   ", "#####"],
        '3' => &["#####", "   # ", "  #  ", "   # ", "    #", "#   #", " ### "],
        '4' => &["   # ", "  ## ", " # # ", "#  # ", "#####", "   # ", "   # "],
        '5' => &["#####", "#    ", "#### ", "    #", "    #", "#   #", " ### "],
        '6' => &["  ## ", " #   ", "#    ", "#### ", "#   #", "#   #", " ### "],
        '7' => &["#####", "    #", "   # ", "  #  ", " #   ", " #   ", " #   "],
        '8' => &[" ### ", "#   #", "#   #", " ### ", "#   #", "#   #", " ### "],
        '9' => &[" ### ", "#   #", "#   #", " ####", "    #", "   # ", " ##  "],
        ':' => &["     ", "  #  ", "  #  ", "     ", "  #  ", "  #  ", "     "],
        '-' => &["     ", "     ", "     ", " ### ", "     ", "     ", "     "],
        '.' => &["     ", "     ", "     ", "     ", "     ", "  ## ", "  ## "],
        ',' => &["     ", "     ", "     ", "     ", "  ## ", "   # ", "  #  "],
        '!' => &["  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "     ", "  #  "],
        '?' => &[" ### ", "#   #", "    #", "   # ", "  #  ", "     ", "  #  "],
        '/' => &["    #", "    #", "   # ", "  #  ", " #   ", "#    ", "#    "],
        '(' => &["   # ", "  #  ", " #   ", " #   ", " #   ", "  #  ", "   # "],
        ')' => &[" #   ", "  #  ", "   # ", "   # ", "   # ", "  #  ", " #   "],
        '>' => &[" #   ", "  #  ", "   # ", "    #", "   # ", "  #  ", " #   "],
        '<' => &["   # ", "  #  ", " #   ", "#    ", " #   ", "  #  ", "   # "],
        '+' => &["     ", "  #  ", "  #  ", "#####", "  #  ", "  #  ", "     "],
        '\'' => &["  #  ", "  #  ", "     ", "     ", "     ", "     ", "     "],
        _ => return None,
    })
}
