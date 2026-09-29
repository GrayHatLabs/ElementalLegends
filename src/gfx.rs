// Software framebuffer, sprites and the built-in bitmap font.

pub const W: i32 = 256;
pub const H: i32 = 240;
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

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
}

pub struct Screen {
    pub px: Vec<u32>,
    clip: [i32; 4],
    pub ox: i32,
    pub oy: i32,
}

impl Screen {
    pub fn new() -> Self {
        Screen { px: vec![BLACK; (W * H) as usize], clip: [0, 0, W, H], ox: 0, oy: 0 }
    }

    pub fn clear(&mut self) {
        self.px.iter_mut().for_each(|p| *p = BLACK);
    }

    pub fn clip(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.clip = [x0.max(0), y0.max(0), x1.min(W), y1.min(H)];
    }

    pub fn unclip(&mut self) {
        self.clip = [0, 0, W, H];
    }

    #[inline]
    pub fn pset(&mut self, x: i32, y: i32, c: u32) {
        let (x, y) = (x + self.ox, y + self.oy);
        if x < self.clip[0] || y < self.clip[1] || x >= self.clip[2] || y >= self.clip[3] {
            return;
        }
        self.px[(y * W + x) as usize] = c;
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let x0 = (x + self.ox).max(self.clip[0]);
        let y0 = (y + self.oy).max(self.clip[1]);
        let x1 = (x + w + self.ox).min(self.clip[2]);
        let y1 = (y + h + self.oy).min(self.clip[3]);
        for yy in y0..y1 {
            let row = (yy * W) as usize;
            for xx in x0..x1 {
                self.px[row + xx as usize] = c;
            }
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32, a: f32) {
        let x0 = (x + self.ox).max(self.clip[0]);
        let y0 = (y + self.oy).max(self.clip[1]);
        let x1 = (x + w + self.ox).min(self.clip[2]);
        let y1 = (y + h + self.oy).min(self.clip[3]);
        for yy in y0..y1 {
            let row = (yy * W) as usize;
            for xx in x0..x1 {
                let i = row + xx as usize;
                self.px[i] = mix(self.px[i], c, a);
            }
        }
    }

    pub fn frame_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        self.fill(x, y, w, 1, c);
        self.fill(x, y + h - 1, w, 1, c);
        self.fill(x, y, 1, h, c);
        self.fill(x + w - 1, y, 1, h, c);
    }

    pub fn blit(&mut self, s: &Sprite, x: i32, y: i32, flip: bool, white: bool) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                let c = s.px[(sy * s.w + sx) as usize];
                if c == 0 {
                    continue;
                }
                let dx = if flip { x + s.w - 1 - sx } else { x + sx };
                self.pset(dx, y + sy, if white { WHITE } else { c });
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

    /// Darken a band of the screen by a light map: `grid` holds darkness (0..1) at the
    /// corners of `cell`-sized squares, bilinearly interpolated per pixel and mixed
    /// toward `tint`. Used for dungeon lighting.
    pub fn light_map(&mut self, y0: i32, grid: &[f32], gw: usize, gh: usize, cell: i32, tint: u32) {
        let (tr, tg, tb) = (((tint >> 16) & 255) as f32, ((tint >> 8) & 255) as f32, (tint & 255) as f32);
        let rows = ((gh - 1) as i32 * cell).min(H - y0);
        for py in 0..rows {
            let sy = py + y0 + self.oy;
            if sy < self.clip[1] || sy >= self.clip[3] {
                continue;
            }
            let gy = (py / cell) as usize;
            let fy = (py % cell) as f32 / cell as f32;
            let row = (sy * W) as usize;
            for px in 0..W.min((gw - 1) as i32 * cell) {
                let sx = px + self.ox;
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
    pub fn text(&mut self, s: &str, x: i32, y: i32, col: u32, align: Align, size: i32) {
        let sc = (size / 8).max(1);
        let tw = s.chars().count() as i32 * 8 * sc;
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
                        self.fill(cx + (gx as i32 + 1) * sc, y + gy as i32 * sc, sc, sc, col);
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
        '>' => &[" #   ", "  #  ", "   # ", "    #", "   # ", "  #  ", " #   "],
        '<' => &["   # ", "  #  ", " #   ", "#    ", " #   ", "  #  ", "   # "],
        '+' => &["     ", "  #  ", "  #  ", "#####", "  #  ", "  #  ", "     "],
        '\'' => &["  #  ", "  #  ", "     ", "     ", "     ", "     ", "     "],
        _ => return None,
    })
}
