//! Set pieces and cinematics: the monolith awakening, dungeon buildings,
//! walking into a dungeon, the staircase descent and boss entrances.
use super::bosses::{BKind, BState};
use super::dungeon::{STAIRS_C, STAIRS_R};
use super::*;

/// Colours of the five rune slots on the monolith (one per lair).
const RUNE_COLS: [u32; 5] = [rgb(0x98d858), rgb(0xa4e4fc), rgb(0xbcbcbc), rgb(0xfc9838), rgb(0xf878f8)];
/// The five lair runes (3x5 bit rows), as carved on the monolith and the rune stones.
const GLYPHS: [[u8; 5]; 5] = [
    [0b111, 0b010, 0b010, 0b010, 0b111],
    [0b101, 0b111, 0b010, 0b111, 0b101],
    [0b010, 0b101, 0b111, 0b101, 0b010],
    [0b110, 0b101, 0b110, 0b101, 0b110],
    [0b111, 0b100, 0b111, 0b001, 0b111],
];
const MONO_X: i32 = 128;
const MONO_BASE: i32 = HUD + 6 * TS;

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Game {
    // ------------------------------------------------------------ monolith awakening
    pub(super) fn begin_awaken(&mut self) {
        let s = self.start;
        self.go_play(s, 128.0, SPAWN_Y, false);
        self.mode = Mode::Awaken;
        self.t = 0;
        self.msg = None;
        self.monolith_used = true;
        self.play_song(Some(Song::Title));
    }
    pub(super) fn update_awaken(&mut self) {
        self.t += 1;
        self.update_parts();
        let t = self.t;
        if t == 110 {
            self.sfx(Sfx::Spell);
        }
        if (110..170).contains(&t) && (t - 110) % 12 == 0 {
            self.sfx(Sfx::Select);
        }
        if t == 172 {
            self.sfx(Sfx::Gate);
        }
        if t == 205 {
            self.sfx(Sfx::Heal);
            self.part(128.0, SPAWN_Y, 0.0, 0.0, 20, rgb(0xa4e4fc), 1, PK::Glow(18.0));
            self.boom(128.0, SPAWN_Y, 20, Elem::Ice);
        }
        if (170..235).contains(&t) && t % 2 == 0 {
            let (ox, oy) = (self.rng.range(-6.0, 6.0), self.rng.range(-60.0, 0.0));
            self.part(128.0 + ox, SPAWN_Y + oy, 0.0, 0.6, 18, WHITE, 1, PK::Dot);
        }
        if (self.p(Btn::Start) && t > 20) || t >= 260 {
            self.mode = Mode::Play;
            self.monolith_used = false;
            self.show_msg("YOU AWAKEN BEFORE THE ANCIENT MONOLITH. TOUCH IT TO RESTORE YOUR STRENGTH, THEN SEEK THE VILLAGE.");
            self.play_song(Some(Song::Village));
        }
    }
    pub(super) fn draw_awaken(&self, scr: &mut Screen) {
        let t = self.t;
        // Camera tilts down from the night sky onto the clearing.
        let pan = 1.0 - ease(t as f32 / 110.0);
        let oy = (pan * (RR as i32 * TS) as f32) as i32;
        self.draw_stars(scr);
        scr.world(self.cam.0, self.cam.1);
        scr.clip_screen(0, 0, SW, SH);
        let room = &self.rooms[self.start];
        self.draw_room_img(scr, room, 0, oy);
        let lit = if t < 110 { 0 } else { ((t - 110) / 12).min(5) };
        self.draw_monolith(scr, 0, oy, lit, t >= 170);
        if (170..240).contains(&t) {
            let a = 0.25 + ((t as f32) * 0.3).sin().abs() * 0.25;
            scr.blend(123, HUD, 10, SPAWN_Y as i32 - HUD, rgb(0xd4f4fc), a);
            scr.blend(126, HUD, 4, SPAWN_Y as i32 - HUD, WHITE, a);
        }
        if t >= 205 && (t >= 235 || self.frame % 2 == 0) {
            self.draw_hero(scr, 0.0, 0.0);
        }
        self.draw_parts(scr);
        scr.ui();
        scr.unclip();
        if t >= 222 {
            scr.fill_screen(0, 36, SW, 22, BLACK);
            scr.text("THE MONOLITH AWAKENS", 128, 43, rgb(0xa4e4fc), Align::Center, 8);
        }
        if t > 20 {
            scr.text("START: SKIP", 250 - 88, 228, rgb(0x747474), Align::Left, 8);
        }
    }

    /// The ancient monolith and its ring of standing stones. `lit` = rune slots glowing.
    pub(super) fn draw_monolith(&self, scr: &mut Screen, ox: i32, oy: i32, lit: i32, awake: bool) {
        let f = self.frame as f32;
        let (x, base) = (MONO_X + ox, MONO_BASE + oy);
        let top = base - 60;
        // Rune circle on the ground.
        let glow = if awake { 0.18 + (f * 0.05).sin().abs() * 0.12 } else { 0.08 };
        scr.blend_ellipse(x, base + 2, 30, 9, rgb(0x3cbcfc), glow);
        scr.ellipse(x, base + 2, 30, 9, rgb(0x2c78c8));
        for i in 0..12 {
            let a = i as f32 * PI / 6.0 + f * 0.01;
            let (gx, gy) = (x + (a.cos() * 27.0) as i32, base + 2 + (a.sin() * 7.0) as i32);
            scr.fill(gx, gy, 1 + (i % 2), 1, if awake { rgb(0xa4e4fc) } else { rgb(0x2c78c8) });
        }
        // Standing stones.
        for &(c, r) in &[(4, 3), (11, 3), (4, 9), (11, 9)] {
            let (sx, sy) = (c * TS + 8 + ox, HUD + r * TS + 16 + oy);
            if self.obj_hd(scr, "obj_standing_stone", sx as f32, sy as f32) {
                if awake {
                    scr.blend_disc(sx, sy - 10, 4, rgb(0x3cbcfc), 0.35);
                }
                continue;
            }
            scr.fill(sx - 5, sy - 16, 10, 16, rgb(0x5c5c68));
            scr.fill(sx - 4, sy - 18, 8, 2, rgb(0x5c5c68));
            scr.fill(sx - 5, sy - 16, 2, 16, rgb(0x8c8c98));
            scr.fill(sx + 3, sy - 16, 2, 16, rgb(0x2c2c38));
            scr.fill(sx - 2, sy - 11, 3, 1, if awake { rgb(0x3cbcfc) } else { rgb(0x3c3c48) });
            scr.fill(sx - 1, sy - 9, 1, 3, if awake { rgb(0x3cbcfc) } else { rgb(0x3c3c48) });
            scr.fill(sx - 5, sy - 3, 4, 2, rgb(0x2c6c1c));
        }
        // The monolith: a tapering slab with highlight, shadow and moss.
        let hd = self.obj_hd(scr, "obj_monolith", x as f32, base as f32);
        for row in 0..if hd { 0 } else { 60 } {
            let w = 11 - row.min(8) / 3 - if row < 4 { 4 - row } else { 0 };
            let (lx, ly) = (x - w, top + row);
            scr.fill(lx, ly, w * 2, 1, rgb(0x5c5c68));
            scr.fill(lx, ly, 2, 1, rgb(0x8c8c98));
            scr.fill(x + w - 2, ly, 2, 1, rgb(0x2c2c38));
        }
        if !hd {
            scr.fill(x - 11, base - 6, 7, 5, rgb(0x2c6c1c));
            scr.fill(x + 5, base - 12, 5, 4, rgb(0x2c6c1c));
            scr.fill(x - 3, top + 20, 2, 3, rgb(0x2c6c1c));
        }
        // Crown gem, always glowing.
        let pulse = 0.3 + (f * 0.08).sin().abs() * 0.35;
        scr.blend_disc(x, top + 6, 6, rgb(0x3cbcfc), pulse);
        scr.fill(x - 1, top + 4, 3, 4, rgb(0xa4e4fc));
        scr.fill(x, top + 4, 1, 1, WHITE);
        // Five rune slots: lit in colour for every conquered lair.
        for (i, g) in GLYPHS.iter().enumerate() {
            let gy = top + 14 + i as i32 * 9;
            let cleared = self.s.cleared[i + 1];
            let col = if lit > i as i32 && self.mode == Mode::Awaken {
                rgb(0xa4e4fc)
            } else if cleared {
                RUNE_COLS[i]
            } else {
                rgb(0x3c3c48)
            };
            if cleared || (lit > i as i32 && self.mode == Mode::Awaken) {
                scr.blend_disc(x, gy + 2, 5, col, 0.3);
            }
            for (ry, bits) in g.iter().enumerate() {
                for bx in 0..3 {
                    if bits & (4 >> bx) != 0 {
                        scr.pset(x - 1 + bx, gy + ry as i32, col);
                    }
                }
            }
        }
        // Magical motes drifting up (deterministic, so drawing needs no RNG).
        if awake {
            for i in 0..10 {
                let ph = (self.frame as i32 + i * 37) % 90;
                let mx = x + ((i as f32 * 2.4 + ph as f32 * 0.05).sin() * 16.0) as i32;
                let my = base - ph * 2 / 3;
                let c = if i % 3 == 0 { WHITE } else { rgb(0xa4e4fc) };
                if ph < 80 {
                    scr.pset(mx, my, c);
                }
            }
        }
    }

    // ------------------------------------------------------------ dungeon buildings
    /// Fantasy structure standing over a lair entrance; the doorway is centred at (128, 128).
    pub(super) fn draw_building(&self, scr: &mut Screen, n: usize, ox: i32, oy: i32) {
        let st = self.gate_state(n);
        let f = self.frame as f32;
        let (cx, by) = (128 + ox, HUD + 6 * TS + oy); // door centre x, ground line y
        let pulse = 0.25 + (f * 0.1).sin().abs() * 0.35;
        let door_glow = match n {
            1 => rgb(0x48d848),
            2 => rgb(0x78a0fc),
            3 => rgb(0xfcbc3c),
            4 => rgb(0xfc6000),
            5 => rgb(0xa4e4fc),
            _ => rgb(0xb040fc),
        };
        // Doorway rectangle (x, y, w, h) filled by each style.
        let (dx, dy, dw, dh) = (cx - 8, by - 22, 16, 22);
        let hd = self.obj_hd(scr, &format!("bld_{}", n.min(6)), cx as f32, by as f32);
        match if hd { 0 } else { n } {
            0 => {}
            1 => {
                // Overgrown forest shrine.
                scr.fill(cx - 28, by - 6, 56, 6, rgb(0x5c5c5c));
                scr.fill(cx - 24, by - 10, 48, 4, rgb(0x747474));
                scr.fill(cx - 26, by - 42, 52, 32, rgb(0x5c6c50));
                for row in 0..4 {
                    scr.fill(cx - 26, by - 40 + row * 8, 52, 1, rgb(0x3c4c34));
                }
                for px in [cx - 28, cx + 20] {
                    scr.fill(px, by - 46, 8, 38, rgb(0x7c8c70));
                    scr.fill(px - 1, by - 48, 10, 3, rgb(0x9cac90));
                    scr.fill(px + 6, by - 46, 2, 38, rgb(0x4c5c40));
                }
                for row in 0..20 {
                    let w = 34 - row * 34 / 20;
                    scr.fill(cx - w, by - 48 - row, w * 2, 1, if row < 3 { rgb(0x2c6c1c) } else { rgb(0x4c5c40) });
                }
                for i in 0..6 {
                    let vx = cx - 30 + i * 12;
                    let len = 8 + (i * 7) % 12;
                    let sway = ((f * 0.05 + i as f32).sin() * 1.5) as i32;
                    scr.line(vx, by - 48, vx + sway, by - 48 + len, rgb(0x2c7c1c));
                    scr.fill(vx + sway - 1, by - 48 + len, 3, 2, rgb(0x48a838));
                }
                for &mx in &[cx - 34, cx + 32] {
                    scr.fill(mx - 1, by - 3, 2, 3, rgb(0xfcd8a8));
                    scr.fill(mx - 3, by - 6, 6, 3, rgb(0xd82800));
                    scr.pset(mx - 1, by - 5, WHITE);
                }
                scr.fill(dx, dy, dw, dh, BLACK);
                scr.disc(cx, dy, 8, BLACK);
            }
            2 => {
                // Underground crypt: a mausoleum with steps descending into darkness.
                scr.fill(cx - 30, by - 36, 60, 36, rgb(0x50505c));
                for row in 0..4 {
                    scr.fill(cx - 30, by - 34 + row * 9, 60, 1, rgb(0x34343c));
                }
                for row in 0..18 {
                    let w = 34 - row * 34 / 18;
                    scr.fill(cx - w, by - 38 - row, w * 2, 1, rgb(0x60606c));
                }
                scr.fill(cx - 34, by - 38, 68, 2, rgb(0x7c7c88));
                scr.disc(cx, by - 46, 4, rgb(0xd8d8c8));
                scr.fill(cx - 2, by - 47, 1, 2, BLACK);
                scr.fill(cx + 1, by - 47, 1, 2, BLACK);
                scr.fill(cx - 1, by - 43, 3, 1, BLACK);
                scr.fill(dx - 2, dy, dw + 4, dh, rgb(0x080808));
                for (i, sy) in [dy + 8, dy + 12, dy + 16].iter().enumerate() {
                    scr.fill(dx + i as i32 * 2, *sy, dw - i as i32 * 4, 1, rgb(0x3c3c44));
                }
                for &kx in &[cx - 24, cx + 22] {
                    scr.fill(kx, by - 10, 2, 6, rgb(0xd8d8c8));
                    let c = if (self.frame / 4) % 2 == 0 { rgb(0xfc9838) } else { rgb(0xfce040) };
                    scr.fill(kx, by - 13, 2, 3, c);
                    scr.blend_disc(kx + 1, by - 12, 4, rgb(0xfc9838), 0.25);
                }
                scr.line(cx + 18, by - 36, cx + 24, by - 28, rgb(0x747480));
                scr.line(cx + 24, by - 36, cx + 18, by - 28, rgb(0x747480));
            }
            3 => {
                // Ruined castle gatehouse with one broken tower and a banner.
                let stone = rgb(0x7c6c50);
                scr.fill(cx - 32, by - 58, 18, 58, stone);
                scr.fill(cx + 14, by - 48, 18, 48, stone);
                for i in 0..4 {
                    scr.fill(cx - 32 + i * 5, by - 62, 3, 4, stone);
                }
                scr.fill(cx + 14, by - 51, 4, 3, stone);
                scr.fill(cx + 22, by - 53, 3, 5, stone);
                scr.fill(cx + 28, by - 50, 4, 2, stone);
                scr.fill(cx - 14, by - 42, 28, 42, rgb(0x6c5c44));
                for i in 0..3 {
                    scr.fill(cx - 12 + i * 10, by - 46, 6, 4, rgb(0x6c5c44));
                }
                for row in 0..6 {
                    scr.fill(cx - 32, by - 50 + row * 9, 64, 1, rgb(0x4c4030));
                }
                scr.fill(cx - 25, by - 44, 3, 8, BLACK);
                scr.fill(cx + 21, by - 36, 3, 8, BLACK);
                scr.fill(dx, dy, dw, dh, BLACK);
                scr.disc(cx, dy, 8, BLACK);
                for i in 0..4 {
                    scr.fill(dx + 1 + i * 4, dy - 6, 1, 8, rgb(0x5c5c64));
                }
                scr.fill(dx - 2, dy - 6, dw + 4, 1, rgb(0x5c5c64));
                scr.line(cx - 23, by - 62, cx - 23, by - 78, rgb(0x5c3410));
                let wave = ((f * 0.15).sin() * 2.0) as i32;
                scr.fill(cx - 22, by - 78, 12, 6 + wave.abs() / 2, rgb(0xa81000));
                scr.fill(cx - 18, by - 76, 3, 2, rgb(0xfcbc3c));
                for &(rx, ry) in &[(cx - 36, by - 3), (cx + 34, by - 2), (cx + 30, by - 4)] {
                    scr.fill(rx, ry, 4, 3, rgb(0x6c5c44));
                }
            }
            4 => {
                // Dragon's mountain fortress: volcanic peak with a carved gate.
                for row in 0..74 {
                    let w = 4 + row * 36 / 74;
                    let y = by - 74 + row;
                    scr.fill(cx - w, y, w * 2, 1, rgb(0x3c2c2c));
                    scr.fill(cx - w, y, 2, 1, rgb(0x5c4444));
                }
                for (i, &(lx, ly)) in [(-3, -70), (4, -62), (-8, -52), (10, -44)].iter().enumerate() {
                    let c = if (self.frame / 6 + i as u64) % 3 == 0 { rgb(0xfce040) } else { rgb(0xfc6000) };
                    scr.line(cx + lx, by + ly, cx + lx + 2, by + ly + 8, c);
                }
                for i in 0..6 {
                    let ph = (self.frame as i32 + i * 23) % 80;
                    let sx = cx + ((i as f32 * 1.7 + ph as f32 * 0.04).sin() * 6.0) as i32;
                    scr.blend_disc(sx, by - 76 - ph / 3, 2 + ph / 30, rgb(0x747474), 0.35);
                }
                scr.fill(dx - 6, dy - 6, dw + 12, dh + 6, rgb(0x241818));
                scr.fill(dx, dy, dw, dh, rgb(0x100404));
                scr.blend(dx, dy + 8, dw, dh - 8, rgb(0xfc6000), pulse * 0.6);
                scr.disc(cx, dy - 11, 6, rgb(0xd8d8c8));
                scr.fill(cx - 3, dy - 12, 2, 2, BLACK);
                scr.fill(cx + 2, dy - 12, 2, 2, BLACK);
                scr.line(cx - 5, dy - 15, cx - 12, dy - 22, rgb(0xd8d8c8));
                scr.line(cx + 5, dy - 15, cx + 12, dy - 22, rgb(0xd8d8c8));
            }
            5 => {
                // Forgotten sanctuary: a columned temple with a glowing sigil.
                let stone = rgb(0x5c5c8c);
                scr.fill(cx - 32, by - 5, 64, 5, rgb(0x3c3c6c));
                scr.fill(cx - 28, by - 8, 56, 3, stone);
                for &px in &[cx - 26, cx - 16, cx + 10, cx + 20] {
                    scr.fill(px, by - 42, 6, 34, rgb(0x7878b8));
                    scr.fill(px + 4, by - 42, 2, 34, rgb(0x3c3c6c));
                }
                scr.fill(cx - 32, by - 48, 64, 6, stone);
                for row in 0..14 {
                    let w = 32 - row * 32 / 14;
                    scr.fill(cx - w, by - 48 - row, w * 2, 1, rgb(0x4c4c7c));
                }
                scr.blend_disc(cx, by - 54, 5, rgb(0xa4e4fc), pulse);
                scr.ring(cx, by - 54, 3, rgb(0xa4e4fc));
                scr.fill(dx, dy, dw, dh, rgb(0x0c0c1c));
                for i in 0..3 {
                    let bob = ((f * 0.06 + i as f32 * 2.0).sin() * 3.0) as i32;
                    let (gx, gy) = (cx - 40 + i * 40, by - 30 + bob - (i % 2) * 10);
                    scr.fill(gx, gy - 2, 1, 5, rgb(0xa4e4fc));
                    scr.fill(gx - 1, gy - 1, 3, 3, rgb(0x78c8ec));
                }
            }
            _ => {
                // The Dark Tower.
                scr.fill(cx - 16, by - 84, 32, 84, rgb(0x302040));
                for row in 0..10 {
                    scr.fill(cx - 16, by - 80 + row * 8, 32, 1, rgb(0x201430));
                }
                scr.fill(cx - 16, by - 84, 3, 84, rgb(0x5c4880));
                for row in 0..14 {
                    let w = 20 - row * 20 / 14;
                    scr.fill(cx - w, by - 86 - row, w * 2, 1, rgb(0x201430));
                }
                for &wy in &[by - 70, by - 50] {
                    scr.fill(cx - 2, wy, 4, 7, rgb(0xb040fc));
                    scr.blend_disc(cx, wy + 3, 5, rgb(0xb040fc), pulse);
                }
                for i in 0..5 {
                    let a = f * 0.04 + i as f32 * 1.26;
                    let (sx, sy) = (cx + (a.cos() * 24.0) as i32, by - 50 + (a.sin() * 8.0) as i32);
                    scr.pset(sx, sy, rgb(0xf878f8));
                }
                scr.fill(dx, dy, dw, dh, BLACK);
                scr.disc(cx, dy, 8, BLACK);
            }
        }
        // Entrance state.
        match st {
            0 => {
                scr.blend(dx, dy - 4, dw, dh + 4, rgb(0x9818a8), 0.35);
                scr.line(dx, dy, dx + dw - 1, dy + dh - 1, rgb(0x9c9ca4));
                scr.line(dx + dw - 1, dy, dx, dy + dh - 1, rgb(0x9c9ca4));
                scr.fill(cx - 2, dy + dh / 2 - 2, 5, 5, rgb(0xfcbc3c));
            }
            1 => {
                scr.blend(dx, dy - 4, dw, dh + 4, door_glow, pulse);
                for i in 0..4 {
                    let ph = (self.frame as i32 + i * 19) % 40;
                    scr.pset(cx - 5 + i * 3, by - 2 - ph / 2, door_glow);
                }
            }
            _ => self.draw_rune_stone(scr, n, dx, dy, dw, dh),
        }
    }

    /// A carved stone slab bearing the lair's rune seals a conquered lair's doorway. With
    /// generated building art the slab covers the doorway painted in the art.
    fn draw_rune_stone(&self, scr: &mut Screen, n: usize, dx: i32, dy: i32, dw: i32, dh: i32) {
        let (cx, by) = (dx + dw / 2, dy + dh);
        let (x, y, w, h) = match self.art_doorway(n) {
            Some((x0, y0, x1, y1)) => (cx + x0 - 1, by + y0 - 1, (x1 - x0 + 3).max(10), (y1 - y0 + 2).max(12)),
            None => (dx - 2, dy - 2, dw + 4, dh + 2),
        };
        scr.fill(x, y, w, h, rgb(0x6c6c78));
        scr.fill(x, y, w, 2, rgb(0x9c9ca8));
        scr.fill(x, y, 2, h, rgb(0x8c8c98));
        scr.fill(x + w - 2, y, 2, h, rgb(0x3c3c48));
        scr.fill(x, y + h - 2, w, 2, rgb(0x3c3c48));
        scr.frame_rect(x, y, w, h, rgb(0x202028));
        // A couple of weathering cracks and a tuft of moss.
        scr.line(x + 3, y + 5, x + 6, y + 9, rgb(0x4c4c58));
        scr.line(x + w - 5, y + h - 8, x + w - 8, y + h - 4, rgb(0x4c4c58));
        scr.fill(x + 1, y + h - 4, 4, 2, rgb(0x2c6c1c));
        // The rune, glowing softly in the lair's colour (the Dark Tower's is violet).
        let col = if n <= 5 { RUNE_COLS[n - 1] } else { rgb(0xb040fc) };
        let glyph = GLYPHS[(n - 1).min(4)];
        let pulse = 0.18 + (self.frame as f32 * 0.06).sin().abs() * 0.2;
        let (gx, gy) = (x + w / 2 - 3, y + h / 2 - 5);
        scr.blend_disc(x + w / 2, y + h / 2, 7, col, pulse);
        for (ry, bits) in glyph.iter().enumerate() {
            for bx in 0..3 {
                if bits & (4 >> bx) != 0 {
                    scr.fill(gx + bx * 2, gy + ry as i32 * 2, 2, 2, col);
                }
            }
        }
    }

    /// The doorway in a building's generated art, as logic offsets (x0, y0, x1, y1) from
    /// the door centre on the ground line. Doorways are painted as one flat dark colour:
    /// take the longest dark run down the middle of the sprite and flood-fill that colour.
    fn art_doorway(&self, n: usize) -> Option<(i32, i32, i32, i32)> {
        let sh = self.art.sheet(&format!("bld_{}", n.min(6)))?;
        let img = sh.anim("idle")?.at(0);
        let (w, h) = (img.w, img.h);
        let at = |x: i32, y: i32| img.px[(y * w + x) as usize];
        let rgb3 = |c: u32| [(c >> 16 & 0xff) as i32, (c >> 8 & 0xff) as i32, (c & 0xff) as i32];
        let dark = |c: u32| c != 0 && rgb3(c).iter().all(|&v| v < 70);
        // Longest run of one dark colour down the centre column (lower two thirds).
        let x = w / 2;
        let (mut best, mut run) = ((0, 0, 0u32), (0, 0, 0u32)); // (length, start y, colour)
        for y in h / 3..h {
            let c = at(x, y);
            if dark(c) && run.0 > 0 && c == run.2 {
                run.0 += 1;
            } else if dark(c) {
                run = (1, y, c);
            } else {
                run = (0, 0, 0);
            }
            if run.0 > best.0 {
                best = run;
            }
        }
        if best.0 < 8 {
            return None;
        }
        let seed_col = rgb3(best.2);
        let near = |c: u32| c != 0 && rgb3(c).iter().zip(seed_col).all(|(a, b)| (a - b).abs() <= 10);
        let mut seen = vec![false; (w * h) as usize];
        let seed = (x, best.1 + best.0 / 2);
        let mut stack = vec![seed];
        seen[(seed.1 * w + seed.0) as usize] = true;
        let mut bb = (seed.0, seed.1, seed.0, seed.1);
        while let Some((px, py)) = stack.pop() {
            bb = (bb.0.min(px), bb.1.min(py), bb.2.max(px), bb.3.max(py));
            for (nx, ny) in [(px + 1, py), (px - 1, py), (px, py + 1), (px, py - 1)] {
                if nx >= 0 && ny >= 0 && nx < w && ny < h && !seen[(ny * w + nx) as usize] && near(at(nx, ny)) {
                    seen[(ny * w + nx) as usize] = true;
                    stack.push((nx, ny));
                }
            }
        }
        // Art pixels -> logic units around the anchor (centre bottom, 2 px up).
        let (ax, ay) = (sh.cell.0 / 2, sh.cell.1 - 2);
        let l = |v: i32| (v as f32 / ZOOM).round() as i32;
        Some((l(bb.0 - ax), l(bb.1 - ay), l(bb.2 - ax), l(bb.3 - ay)))
    }

    // ------------------------------------------------------------ entering a dungeon
    pub(super) fn begin_enter_dungeon(&mut self, n: usize) {
        self.mode = Mode::EnterDungeon;
        self.t = 0;
        self.gate_n = n;
        self.gate_room = self.room;
        self.walk_from = (self.pl.x, self.pl.y);
        self.eb.clear();
        self.pb.clear();
        self.pl.dir = b'u';
        self.play_song(None);
        self.sfx(Sfx::Gate);
    }
    pub(super) fn update_enter_dungeon(&mut self) {
        self.t += 1;
        self.update_parts();
        let t = self.t;
        let p = ease(t as f32 / 30.0);
        let (gx, gy) = self.door_pos(self.gate_room);
        self.pl.x = self.walk_from.0 + (gx - self.walk_from.0) * p;
        self.pl.y = self.walk_from.1 + (gy - 12.0 - self.walk_from.1) * p;
        self.pl.walk += 1;
        self.pl.moving = t < 30;
        if t >= 60 {
            let n = self.gate_n;
            self.start_dungeon(n);
        }
    }
    pub(super) fn draw_enter_dungeon(&self, scr: &mut Screen) {
        let t = self.t;
        if t > 30 {
            scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, ((t - 30) as f32 / 26.0).min(1.0));
        }
        let name = self.dname(self.gate_n);
        if t > 12 && !name.is_empty() {
            scr.fill_screen(0, 150, SW, 20, BLACK);
            scr.text(&name, 128, 156, WHITE, Align::Center, 8);
        }
    }

    // ------------------------------------------------------------ staircase descent
    pub(super) fn begin_descend(&mut self) {
        self.mode = Mode::Descend;
        self.t = 0;
        self.walk_from = (self.pl.x, self.pl.y);
        self.gate_n = self.dungeon.as_ref().map_or(1, |d| d.n);
        self.enemies.clear();
        self.eb.clear();
        self.pb.clear();
        self.sink = 0.0;
        self.pan_y = 0.0;
        self.msg = None;
        self.play_song(None);
        self.sfx(Sfx::Gate);
    }
    fn stairs_xy() -> (f32, f32) {
        let (sx, sy) = tile_center(STAIRS_C, STAIRS_R);
        (sx + 8.0, sy + 8.0)
    }
    pub(super) fn update_descend(&mut self) {
        self.t += 1;
        self.update_parts();
        let t = self.t;
        let (sx, sy) = Self::stairs_xy();
        let bottom = sy + 22.0;
        self.pl.dir = b'u';
        self.pl.inv = 0;
        self.pl.moving = (20..130).contains(&t);
        if t < 20 {
            if t % 3 == 0 {
                let ox = self.rng.range(-14.0, 14.0);
                self.part(sx + ox, sy + 10.0, 0.0, -0.8, 18, rgb(0xb040fc), 1, PK::Ember);
            }
        } else if t < 60 {
            let p = ease((t - 20) as f32 / 40.0);
            self.pl.x = self.walk_from.0 + (sx - self.walk_from.0) * p;
            self.pl.y = self.walk_from.1 + (bottom - self.walk_from.1) * p;
            self.pl.walk += 1;
        } else if t < 130 {
            // Walk down the steps: the mage moves onto the stairs and sinks out of sight.
            self.pl.y = bottom - (t - 60) as f32 * 0.3;
            self.sink = ((t - 60) as f32 / 70.0 * 18.0).min(18.0);
            self.pan_y = ((t - 60) as f32 * 0.22).min(14.0);
            self.pl.walk += 1;
            if t % 16 == 0 {
                self.sfx(Sfx::Push);
            }
        }
        if t == 120 {
            self.sfx(Sfx::Rumble);
        }
        if t >= 160 {
            let n = self.gate_n;
            self.sink = 0.0;
            self.pan_y = 0.0;
            self.start_boss(n);
        }
    }
    pub(super) fn draw_descend_overlay(&self, scr: &mut Screen) {
        let t = self.t;
        let dark = ((t - 50) as f32 / 90.0).clamp(0.0, 1.0);
        if dark > 0.0 {
            // Vignette: edges darken first, then everything.
            scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, dark * 0.85);
            for i in 0..6 {
                let a = (dark * 0.25 * (6 - i) as f32 / 6.0).min(1.0);
                scr.blend_screen(0, HUD_PX + i * 8, SW, 8, BLACK, a);
                scr.blend_screen(0, SH - (i + 1) * 8, SW, 8, BLACK, a);
                scr.blend_screen(i * 8, HUD_PX, 8, SH - HUD_PX, BLACK, a);
                scr.blend_screen(SW - (i + 1) * 8, HUD_PX, 8, SH - HUD_PX, BLACK, a);
            }
        }
        if t >= 140 {
            scr.fill_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK);
        }
    }

    // ------------------------------------------------------------ boss entrance
    pub(super) fn update_boss_intro(&mut self) {
        self.t += 1;
        self.update_parts();
        if self.fade > 0 {
            self.fade -= 1;
        }
        let t = self.t;
        let Some(mut b) = self.boss.take() else {
            self.mode = Mode::Play;
            return;
        };
        b.intro = t;
        b.t += 1;
        match b.kind {
            BKind::Treant => {
                if t < 110 && t % 6 == 0 {
                    self.shake = 3;
                    let ox = self.rng.range(-28.0, 28.0);
                    self.part(b.x + ox, b.y + 30.0, ox * 0.03, -1.2, 22, rgb(0x5c3c10), 2, PK::Shard);
                }
            }
            BKind::Guardian => {
                if t < 110 && t % 2 == 0 {
                    let a = self.rng.range(0.0, PI * 2.0);
                    self.part(b.x + a.cos() * 22.0, b.y + a.sin() * 22.0, -a.cos() * 0.4, -a.sin() * 0.4, 22, rgb(0x9818a8), 2, PK::Dot);
                }
                if t == 100 {
                    self.flash = 10;
                }
            }
            BKind::Golem => {
                if t < 110 && t % 12 == 0 {
                    self.sfx(Sfx::Push);
                    self.shake = 4;
                }
                if t == 110 {
                    self.shake = 14;
                    self.flash = 8;
                    self.sfx(Sfx::Quake);
                }
            }
            BKind::Dragon => {
                b.z = (1.0 - (t as f32 / 100.0).min(1.0)) * 150.0;
                if t == 40 {
                    self.sfx(Sfx::Roar);
                }
                if t == 100 {
                    self.shake = 16;
                    self.sfx(Sfx::Boom);
                    for _ in 0..14 {
                        let vx = self.rng.range(-2.0, 2.0);
                        self.part(b.x + vx * 8.0, b.y + 14.0, vx, -0.6, 22, rgb(0x747468), 2, PK::Dot);
                    }
                }
            }
            BKind::Sorcerer | BKind::DarkSorcerer => {
                if t < 100 && t % 8 == 0 {
                    let (lx, ly) = (b.x + self.rng.range(-30.0, 30.0), HUDF + 16.0);
                    self.part(lx, ly, 0.0, 0.0, 8, b.el.light(), 1, PK::Line(b.x, b.y));
                    self.sfx(Sfx::Zap);
                }
                if t == 100 {
                    self.flash = 12;
                    self.part(b.x, b.y, 0.0, 0.0, 16, b.el.light(), 1, PK::Ring);
                }
            }
        }
        if t == 120 {
            self.sfx(Sfx::Roar);
        }
        if t >= 150 {
            b.state = BState::Idle;
            b.z = 0.0;
            b.cool = 40;
            self.mode = Mode::Play;
        }
        self.boss = Some(b);
    }
    pub(super) fn draw_boss_banner(&self, scr: &mut Screen) {
        let Some(b) = &self.boss else { return };
        let t = self.t;
        if !(30..150).contains(&t) {
            return;
        }
        scr.fill_screen(0, 150, SW, 36, BLACK);
        scr.fill_screen(0, 150, SW, 1, rgb(0x5c4880));
        scr.fill_screen(0, 185, SW, 1, rgb(0x5c4880));
        scr.text(b.name, 128, 156, b.el.light(), Align::Center, 8);
        let hint = match b.kind {
            BKind::Sorcerer | BKind::DarkSorcerer => "ITS WEAKNESS SHIFTS WITH ITS ELEMENT!".to_string(),
            BKind::Golem => "EARTH CRACKS ARMOR, STORM CORE".to_string(),
            _ => format!("WEAK TO {}", b.weak.name()),
        };
        scr.text(&hint, 128, 172, WHITE, Align::Center, 8);
    }
}
