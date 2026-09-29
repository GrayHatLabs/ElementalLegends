//! Rendering: the play field, entities, effects, HUD, maps and menus.
use super::bosses::{draw_flames, draw_ice_block, BState};
use super::dungeon::{dungeon_name, Obj, OK, R_EAST, R_ENTRY, R_FEAST, R_HUB, R_PANTRY, R_STAIRS, R_WEST, STAIRS_C, STAIRS_R};
use super::*;

impl Game {
    pub fn draw(&mut self, scr: &mut Screen) {
        scr.unclip();
        scr.ox = 0;
        scr.oy = 0;
        scr.clear();
        if self.shake > 0 {
            scr.ox = self.rng.irange(-2, 2);
            scr.oy = self.rng.irange(-2, 2);
        }
        match self.mode {
            Mode::Title => self.draw_title(scr),
            Mode::Choose => self.draw_choose(scr),
            Mode::Intro => self.draw_intro(scr),
            Mode::Awaken => self.draw_awaken(scr),
            Mode::Play | Mode::Dying | Mode::EnterDungeon | Mode::Descend | Mode::BossIntro => {
                self.draw_play(scr);
                match self.mode {
                    Mode::EnterDungeon => self.draw_enter_dungeon(scr),
                    Mode::Descend => self.draw_descend_overlay(scr),
                    Mode::BossIntro => self.draw_boss_banner(scr),
                    _ => {}
                }
                self.draw_hud(scr);
                if self.paused {
                    if self.in_lair > 0 {
                        scr.blend(0, HUD, W, H - HUD, BLACK, 0.6);
                        scr.text("PAUSED", 128, 120, WHITE, Align::Center, 16);
                    } else if self.dungeon.is_some() {
                        self.draw_dungeon_map(scr);
                    } else {
                        self.draw_map(scr);
                    }
                }
            }
            Mode::GameOver => self.draw_gameover(scr),
            Mode::Reward => self.draw_reward(scr),
            Mode::Victory => self.draw_victory(scr),
        }
        scr.ox = 0;
        scr.oy = 0;
        scr.unclip();
        if self.flash > 0 {
            scr.blend(0, 0, W, H, WHITE, self.flash as f32 / 20.0);
        }
    }

    pub(super) fn draw_stars(&self, scr: &mut Screen) {
        for s in &self.stars {
            let c = if s.s > 1.2 { WHITE } else if s.s > 0.7 { rgb(0x747474) } else { rgb(0x3c3c7c) };
            scr.pset(s.x as i32, s.y as i32, c);
        }
    }

    // ------------------------------------------------------------ world
    fn draw_shrine(&self, scr: &mut Screen, x: i32, y: i32, el: Elem) {
        scr.fill(x - 7, y + 4, 14, 6, rgb(0x747474));
        scr.fill(x - 7, y + 4, 14, 1, rgb(0xbcbcbc));
        scr.fill(x - 5, y + 10, 10, 2, rgb(0x404040));
        let oy2 = y - 4 + ((self.frame as f32 * 0.08).sin() * 2.0) as i32;
        scr.blend_disc(x, oy2, 8, el.light(), 0.2);
        scr.disc(x, oy2, 5, el.main());
        scr.disc(x - 1, oy2 - 1, 3, el.light());
        scr.pset(x - 2, oy2 - 2, WHITE);
        if self.frame % 20 < 10 {
            scr.pset(x + 6, oy2 - 5, WHITE);
        }
    }
    fn draw_cottage(&self, scr: &mut Screen, ox: i32, oy: i32) {
        let (x, y) = (128 + ox, HUD + 16 + oy);
        scr.fill(x - 28, y + 10, 56, 22, rgb(0x8c6c4c));
        for i in 0..5 {
            scr.fill(x - 28 + i * 14, y + 10, 2, 22, rgb(0x5c3c20));
        }
        scr.fill(x - 28, y + 20, 56, 2, rgb(0x5c3c20));
        for row in 0..14 {
            let w = 34 - row * 34 / 14;
            scr.fill(x - w, y + 10 - row, w * 2, 1, if row == 0 { rgb(0x681008) } else { rgb(0xa81000) });
        }
        scr.fill(x - 5, y + 18, 10, 14, rgb(0x5c3410));
        scr.fill(x + 2, y + 25, 1, 1, rgb(0xfcbc3c));
        for &wx in &[x - 20, x + 13] {
            scr.fill(wx, y + 14, 7, 5, rgb(0xfce040));
            scr.fill(wx + 3, y + 14, 1, 5, rgb(0x5c3c20));
        }
        if (self.frame / 30) % 2 == 0 {
            scr.blend_disc(x + 18, y - 6, 2, rgb(0x747474), 0.4);
        }
    }
    pub(super) fn draw_room_objs(&self, scr: &mut Screen, ri: usize, ox: i32, oy: i32) {
        let r = &self.rooms[ri];
        let (x, y) = (GATE_X as i32 + ox, GATE_Y as i32 + oy);
        let bob = if (self.frame >> 4) & 1 == 1 { 1.0 } else { 0.0 };
        if r.gate > 0 {
            self.draw_building(scr, r.gate, ox, oy);
        }
        if r.special == SP_MONOLITH {
            self.draw_monolith(scr, ox, oy, 5, true);
        }
        if r.tank && !self.s.tanks.contains(&ri) {
            scr.spr(&self.spr.big_heart.img, x as f32, y as f32 - bob, false);
        }
        if r.cache && !self.s.caches.contains(&ri) {
            scr.spr(&self.spr.hoard.img, x as f32, y as f32, false);
        }
        if let Some((cx, cy, _, _)) = r.chest {
            let s = if self.s.opened.contains(&ri) { &self.spr.chest_open } else { &self.spr.chest };
            scr.spr(&s.img, cx + ox as f32, cy + oy as f32, false);
        }
        if let Some(se) = r.shrine {
            self.draw_shrine(scr, x, y, Elem::from_idx(se));
        }
        if r.special == SP_SHOP {
            self.draw_cottage(scr, ox, oy);
            let sage = &self.spr.mage_d[4];
            scr.spr(&sage.img, 128.0 + ox as f32, HUDF + 58.0 + oy as f32, false);
            let prices = [15, 25, self.s.heart_price];
            for i in 0..3 {
                let px = Self::shop_x(i) as i32 + ox;
                let py = GATE_Y as i32 + oy;
                scr.fill(px - 8, py + 4, 16, 5, rgb(0x6c3c10));
                scr.fill(px - 8, py + 4, 16, 1, rgb(0xa86030));
                let s = match i {
                    0 => &self.spr.meat,
                    1 => &self.spr.potion,
                    _ => &self.spr.big_heart,
                };
                scr.spr(&s.img, px as f32, py as f32 - 2.0, false);
                scr.text(&prices[i].to_string(), px + 1, py + 12, rgb(0xfcbc3c), Align::Center, 8);
            }
        }
    }
    fn draw_obj(&self, scr: &mut Screen, o: &Obj) {
        let (fx, fy) = o.pos();
        let (x, y) = (fx as i32, fy as i32);
        match o.k {
            OK::Torch => {
                scr.fill(x - 4, y + 1, 8, 7, rgb(0x3c3c44));
                scr.fill(x - 3, y + 1, 6, 1, rgb(0x5c5c64));
                scr.fill(x - 6, y - 3, 12, 4, rgb(0x5c5c64));
                scr.fill(x - 6, y - 3, 12, 1, rgb(0x8c8c98));
                if o.on {
                    scr.blend_disc(x, y - 6, 11, rgb(0xfc9838), 0.18 + (self.frame as f32 * 0.2).sin().abs() * 0.08);
                    draw_flames(scr, x as f32, y as f32 - 8.0, 8.0, 6.0, 1.0, self.frame, (o.c * 7 + o.r) as u32);
                } else {
                    scr.fill(x - 4, y - 4, 8, 1, rgb(0x202020));
                    if (self.frame / 20 + o.c as u64) % 4 == 0 {
                        scr.pset(x, y - 7 - ((self.frame % 20) / 5) as i32, rgb(0x5c5c64));
                    }
                }
            }
            OK::Block => {
                scr.fill(x - 8, y - 8, 16, 16, rgb(0x5c5c64));
                scr.fill(x - 8, y - 8, 16, 1, rgb(0x8c8c98));
                scr.fill(x - 8, y - 8, 1, 16, rgb(0x8c8c98));
                scr.fill(x - 8, y + 7, 16, 1, rgb(0x2c2c34));
                scr.fill(x + 7, y - 8, 1, 16, rgb(0x2c2c34));
                let rc = if o.on { rgb(0x3cbcfc) } else { rgb(0x3c3c48) };
                scr.fill(x - 3, y - 1, 7, 1, rc);
                scr.fill(x, y - 4, 1, 7, rc);
                if o.on {
                    scr.blend(x - 8, y - 8, 16, 16, rgb(0x3cbcfc), 0.12);
                }
            }
            OK::Lever => {
                scr.fill(x - 6, y + 2, 12, 5, rgb(0x3c3c44));
                scr.fill(x - 6, y + 2, 12, 1, rgb(0x747480));
                let (ex, knob) = if o.on { (x + 6, rgb(0x58d854)) } else { (x - 6, rgb(0xd82800)) };
                scr.line(x, y + 2, ex, y - 7, rgb(0xa0a0ac));
                scr.disc(ex, y - 8, 2, knob);
            }
            OK::Chest => {
                let s = if o.on { &self.spr.chest_open } else { &self.spr.chest };
                scr.spr(&s.img, fx, fy, false);
                if !o.on && self.frame % 30 < 4 {
                    scr.pset(x + 6, y - 6, WHITE);
                }
            }
            OK::Shrine(el) => self.draw_shrine(scr, x, y, el),
        }
    }
    fn draw_dungeon_overlays(&self, scr: &mut Screen) {
        let Some(d) = &self.dungeon else { return };
        let room = &d.rooms[d.cur];
        let f = self.frame as i32;
        for r in 0..RR {
            for c in 0..RC {
                let (px, py) = (c as i32 * TS, HUD + r as i32 * TS);
                match room.tiles[r][c] {
                    T_WATER => {
                        let o = (f / 6 + c as i32 * 3 + r as i32 * 5) % 16;
                        scr.fill(px + o, py + 4 + (c as i32 % 3) * 4, 3, 1, rgb(0x5c8cdc));
                        scr.fill(px + (o + 8) % 16, py + 10, 2, 1, rgb(0x3c6cbc));
                    }
                    T_PLATE => {
                        let pressed = d.objs[d.cur].iter().any(|o| o.k == OK::Block && o.c == c as i32 && o.r == r as i32);
                        if pressed {
                            scr.blend(px + 2, py + 2, 12, 12, rgb(0x3cbcfc), 0.3);
                        }
                    }
                    _ => {}
                }
            }
        }
        if d.cur == R_STAIRS {
            let (sx, sy) = (STAIRS_C * TS, HUD + STAIRS_R * TS);
            if room.tiles[STAIRS_R as usize][STAIRS_C as usize] == T_BARRIER {
                let a = 0.3 + (self.frame as f32 * 0.1).sin().abs() * 0.25;
                scr.blend(sx - 2, sy - 2, 36, 36, rgb(0x9818a8), a);
                scr.frame_rect(sx - 2, sy - 2, 36, 36, rgb(0xf878f8));
                scr.ring(sx + 16, sy + 16, 11, rgb(0xf878f8));
                scr.line(sx + 6, sy + 16, sx + 26, sy + 16, rgb(0xf878f8));
                scr.line(sx + 16, sy + 6, sx + 16, sy + 26, rgb(0xf878f8));
            } else {
                let a = 0.1 + (self.frame as f32 * 0.08).sin().abs() * 0.1;
                scr.blend(sx, sy, 32, 32, rgb(0xb040fc), a);
            }
        }
        if d.cur == R_FEAST {
            self.draw_feast_tables(scr);
        }
        if d.cur == R_PANTRY {
            self.draw_pantry(scr, room);
        }
        for o in d.objs[d.cur].iter().filter(|o| o.visible) {
            self.draw_obj(scr, o);
        }
    }
    /// Two long banquet tables with plates, goblets and candles (drawn over their decor tiles).
    fn draw_feast_tables(&self, scr: &mut Screen) {
        for (ti, row) in [4i32, 8].iter().enumerate() {
            let ti = ti as i32;
            let (x0, y0) = (4 * TS, HUD + row * TS);
            let w = 8 * TS;
            scr.blend_ellipse(x0 + w / 2, y0 + 15, w / 2, 3, BLACK, 0.35);
            scr.fill(x0 + 2, y0 + 10, 2, 6, rgb(0x3c2410));
            scr.fill(x0 + w - 4, y0 + 10, 2, 6, rgb(0x3c2410));
            scr.fill(x0, y0 + 2, w, 9, rgb(0x8c5020));
            scr.fill(x0, y0 + 2, w, 1, rgb(0xb87838));
            scr.fill(x0, y0 + 10, w, 1, rgb(0x5c3410));
            scr.fill(x0 + 1, y0 + 4, w - 2, 1, rgb(0xa86030));
            for i in 0..8 {
                let px = x0 + 8 + i * TS;
                scr.disc(px, y0 + 6, 3, rgb(0xd8d8c8));
                scr.disc(px, y0 + 6, 2, WHITE);
                let dish = match (i + ti * 3) % 4 {
                    0 => &self.spr.meat.img,
                    1 => &self.spr.bread.img,
                    2 => &self.spr.apple.img,
                    _ => &self.spr.potion.img,
                };
                if (i + ti) % 2 == 0 {
                    scr.spr(dish, px as f32, (y0 + 4) as f32, false);
                } else {
                    scr.fill(px - 1, y0 + 2, 3, 5, rgb(0xfcbc3c));
                    scr.fill(px - 2, y0 + 1, 5, 1, rgb(0xfce040));
                }
            }
            for &cx in &[x0 + 4, x0 + w - 5] {
                scr.fill(cx, y0 - 3, 2, 6, rgb(0xf8f0d8));
                let c = if (self.frame / 5 + cx as u64) % 2 == 0 { rgb(0xfce040) } else { rgb(0xfc9838) };
                scr.fill(cx, y0 - 6, 2, 3, c);
                scr.blend_disc(cx + 1, y0 - 5, 6, rgb(0xfc9838), 0.18);
            }
        }
    }
    /// Barrels and sacks on the pantry's decor tiles.
    fn draw_pantry(&self, scr: &mut Screen, room: &Room) {
        for r in 0..RR {
            for c in 0..RC {
                if room.tiles[r][c] != T_DECOR {
                    continue;
                }
                let (x, y) = (c as i32 * TS + 8, HUD + r as i32 * TS + 8);
                if (c + r) % 3 == 0 {
                    scr.disc(x, y + 2, 6, rgb(0xc8b080));
                    scr.disc(x - 2, y, 3, rgb(0xe0d0a0));
                    scr.fill(x - 2, y - 6, 4, 3, rgb(0xa08858));
                } else {
                    scr.fill(x - 6, y - 7, 12, 15, rgb(0x8c5020));
                    scr.fill(x - 6, y - 7, 12, 2, rgb(0xb87838));
                    scr.fill(x - 6, y - 3, 12, 1, rgb(0x404040));
                    scr.fill(x - 6, y + 4, 12, 1, rgb(0x404040));
                    scr.fill(x + 3, y - 5, 2, 12, rgb(0x5c3410));
                }
            }
        }
    }
    pub(super) fn draw_hero(&self, scr: &mut Screen, ox: f32, oy: f32) {
        let p = &self.pl;
        if p.inv > 0 && self.frame & 2 != 0 && self.mode == Mode::Play {
            return;
        }
        if self.mode == Mode::EnterDungeon && self.t > 24 && (self.t > 34 || self.frame % 2 == 0) {
            return;
        }
        let e = self.s.el.min(3);
        let s = match p.dir {
            b'u' => &self.spr.mage_u[e],
            b'd' => &self.spr.mage_d[e],
            _ => &self.spr.mage_s[e],
        };
        let bob = ((p.walk >> 3) & 1) as f32;
        let (x, y) = (p.x + ox, p.y + oy - 2.0 - bob);
        scr.spr(&s.img, x, y, p.dir == b'l');
        if p.cast > 0 {
            let sx = if p.dir == b'l' { x - 6.0 } else { x + 6.0 };
            scr.blend_disc(sx as i32, (y - 3.0) as i32, 4, Elem::from_idx(e).light(), 0.35);
            scr.disc(sx as i32, (y - 3.0) as i32, 2, Elem::from_idx(e).light());
        }
    }
    fn draw_enemy(&self, scr: &mut Screen, e: &Enemy) {
        if e.spawn > 0 {
            if e.spawn & 2 != 0 {
                let r = e.spawn / 4;
                scr.fill(e.x as i32 - r, e.y as i32, r * 2 + 1, 1, WHITE);
                scr.fill(e.x as i32, e.y as i32 - r, 1, r * 2 + 1, WHITE);
            }
            return;
        }
        let still = e.st.immobile();
        if e.k == EK::Ghost && self.frame % 4 == 0 && e.flash == 0 && !still {
            return;
        }
        let set = if e.k == EK::Generator {
            &self.spr.generator
        } else {
            &self.spr.enemies[e.k as usize][e.el.idx()]
        };
        let mut flip = false;
        let mut bob = 0.0;
        if !still {
            match e.k {
                EK::Slime => bob = ((e.t >> 4) & 1) as f32,
                EK::Bat | EK::Skeleton => flip = (e.t >> 3) & 1 == 1,
                EK::Ghost => bob = (e.t as f32 * 0.1).sin().round(),
                EK::Generator => bob = if e.t % 30 < 4 { -1.0 } else { 0.0 },
                _ => {}
            }
        }
        let img = if e.flash > 0 {
            &set.white
        } else if e.st.frozen() || e.st.chilled() {
            &set.frost
        } else {
            &set.img
        };
        scr.spr(img, e.x, e.y + bob, flip);
        if e.st.frozen() {
            draw_ice_block(scr, e.x, e.y, e.w, e.h, e.st.encase(), self.frame);
        } else if e.st.chilled() && (self.frame + e.id as u64) % 12 < 2 {
            scr.pset(e.x as i32 + 5, e.y as i32 - 6, WHITE);
            scr.pset(e.x as i32 - 4, e.y as i32 - 2, rgb(0xa4e4fc));
        }
        if e.st.burning() {
            draw_flames(scr, e.x, e.y, e.w, e.h, e.st.flame_scale(), self.frame, e.id);
        }
        if e.st.stun > 0 && !e.st.frozen() {
            for i in 0..2 {
                let a = self.frame as f32 * 0.2 + i as f32 * PI;
                scr.pset(e.x as i32 + (a.cos() * 6.0) as i32, e.y as i32 - e.h as i32 / 2 - 3, rgb(0xfce040));
            }
        }
    }
    fn draw_bullets(&self, scr: &mut Screen) {
        for b in &self.pb {
            let (x, y) = (b.x.round() as i32, b.y.round() as i32);
            match b.el {
                Elem::Fire => {
                    // Firebolt: glowing halo, pulsing core, bright heart leading the flight.
                    let l = b.vx.hypot(b.vy).max(0.01);
                    let (ux, uy) = (b.vx / l, b.vy / l);
                    scr.blend_disc(x, y, 7, rgb(0xfc9838), 0.28);
                    let r = if b.age % 4 < 2 { 4 } else { 3 };
                    scr.disc(x, y, r, rgb(0xd82800));
                    scr.disc(x + (ux * 1.0) as i32, y + (uy * 1.0) as i32, r - 1, rgb(0xfc9838));
                    scr.disc(x + (ux * 2.0) as i32, y + (uy * 2.0) as i32, 1, rgb(0xfce040));
                    scr.pset(x + (ux * 2.0) as i32, y + (uy * 2.0) as i32, WHITE);
                }
                Elem::Ice => {
                    // Spinning ice shard.
                    let a = b.vy.atan2(b.vx) + (b.age as f32 * 0.35).sin() * 0.5;
                    let (ca, sa) = (a.cos(), a.sin());
                    let pts = [(5.0, 0.0), (0.0, 2.5), (-4.0, 0.0), (0.0, -2.5)];
                    let pp: Vec<(i32, i32)> =
                        pts.iter().map(|&(px, py)| (x + (px * ca - py * sa) as i32, y + (px * sa + py * ca) as i32)).collect();
                    scr.blend_disc(x, y, 5, rgb(0xa4e4fc), 0.3);
                    for i in 0..4 {
                        let (p0, p1) = (pp[i], pp[(i + 1) % 4]);
                        scr.line(p0.0, p0.1, p1.0, p1.1, rgb(0x78c8ec));
                    }
                    scr.line(pp[0].0, pp[0].1, pp[2].0, pp[2].1, WHITE);
                    scr.fill(x, y, 1, 1, WHITE);
                }
                Elem::Storm => {
                    let l = b.vx.hypot(b.vy).max(0.01);
                    let (ux, uy) = (b.vx / l, b.vy / l);
                    let (nx, ny) = (-uy, ux);
                    let mut prev = (x, y);
                    for k in 1..5 {
                        let j = if (k + b.age) % 2 == 0 { 2.0 } else { -2.0 };
                        let pt = ((b.x - ux * k as f32 * 3.0 + nx * j) as i32, (b.y - uy * k as f32 * 3.0 + ny * j) as i32);
                        scr.line(prev.0, prev.1, pt.0, pt.1, if k < 3 { WHITE } else { rgb(0xfce040) });
                        prev = pt;
                    }
                    scr.blend_disc(x, y, 3, rgb(0xfce040), 0.35);
                }
                _ => {
                    scr.disc(x, y, 4, rgb(0x3c2410));
                    scr.disc(x, y, 3, rgb(0x8c5020));
                    let a = b.age as f32 * 0.4;
                    scr.fill(x + (a.cos() * 1.5) as i32 - 1, y + (a.sin() * 1.5) as i32 - 1, 2, 1, rgb(0x98d858));
                }
            }
        }
        let blink = (self.frame >> 2) & 1 == 1;
        for b in &self.eb {
            let (x, y) = (b.x.round() as i32, b.y.round() as i32);
            match b.style {
                Shot::Flame => {
                    let r = 2 + (b.age / 12).min(2);
                    scr.blend_disc(x, y, r + 2, rgb(0xfc9838), 0.3);
                    scr.disc(x, y, r, if b.age % 4 < 2 { rgb(0xfc9838) } else { rgb(0xd82800) });
                    scr.pset(x, y, rgb(0xfce040));
                }
                Shot::Fireball => {
                    scr.blend_disc(x, y, 7, rgb(0xfc6000), 0.3);
                    scr.disc(x, y, 4, rgb(0xd82800));
                    scr.disc(x, y, 3, rgb(0xfc9838));
                    scr.disc(x, y, 1, rgb(0xfce040));
                }
                Shot::Boulder => {
                    scr.blend_ellipse(x, y + 6, 5, 2, BLACK, 0.3);
                    scr.disc(x, y, 5, rgb(0x5c5c58));
                    scr.disc(x - 1, y - 2, 2, rgb(0x9c9c90));
                    scr.pset(x + 2, y + 2, rgb(0x2c2c28));
                }
                Shot::Dark => {
                    scr.blend_disc(x, y, 5, rgb(0x9818a8), 0.35);
                    scr.disc(x, y, 3, rgb(0x9818a8));
                    scr.disc(x, y, 1, BLACK);
                }
                Shot::Wave => {
                    scr.fill(x - 2, y - 1, 5, 3, rgb(0x5c3c10));
                    scr.fill(x - 1, y - 1, 3, 1, rgb(0x98d858));
                }
                Shot::Orb => {
                    let c = if b.el == Elem::Neutral {
                        if blink { rgb(0xfcfcfc) } else { rgb(0xbcbcbc) }
                    } else if blink {
                        b.el.light()
                    } else {
                        b.el.main()
                    };
                    scr.fill(x - 2, y - 1, 4, 2, c);
                    scr.fill(x - 1, y - 2, 2, 4, c);
                    scr.fill(x - 1, y - 1, 2, 2, if b.el == Elem::Neutral { rgb(0x747474) } else { WHITE });
                }
            }
        }
    }
    fn draw_items(&self, scr: &mut Screen) {
        let bob = ((self.frame as f32 * 0.12).sin() * 1.5).round();
        for it in &self.items {
            if it.life < 120 && self.frame & 4 != 0 {
                continue;
            }
            let s = match it.kind {
                IK::Coin => &self.spr.coin,
                IK::Gem => &self.spr.gem,
                IK::Apple => &self.spr.apple,
                IK::Bread => &self.spr.bread,
                IK::Meat => &self.spr.meat,
                IK::Potion => &self.spr.potion,
                IK::Heart => &self.spr.heart,
                IK::Orb => {
                    let (x, y) = (it.x as i32, (it.y + bob) as i32);
                    scr.blend_disc(x, y, 6, it.el.light(), 0.25);
                    scr.disc(x, y, 4, it.el.main());
                    scr.disc(x - 1, y - 1, 2, it.el.light());
                    scr.pset(x - 2, y - 2, WHITE);
                    continue;
                }
            };
            scr.spr(&s.img, it.x, it.y + if it.kind == IK::Coin { 0.0 } else { bob }, false);
        }
    }
    pub(super) fn draw_parts(&self, scr: &mut Screen) {
        for p in &self.parts {
            let q = 1.0 - p.life as f32 / p.max.max(1) as f32; // 0 -> 1 over the lifetime
            let (x, y) = (p.x.round() as i32, p.y.round() as i32);
            match &p.kind {
                PK::Text(s) => scr.text(s, x, y, p.c, Align::Left, 8),
                PK::Ring => {
                    let r = (p.max - p.life) * 2 + 2;
                    scr.ring(x, y, r, if p.c != WHITE { p.c } else if p.life > 6 { WHITE } else { rgb(0xfcbc3c) });
                }
                PK::Line(x2, y2) => {
                    let (mx, my) = ((p.x + x2) / 2.0 + self.rng_jitter(p.life), (p.y + y2) / 2.0 - self.rng_jitter(p.life + 3));
                    let c = if p.life % 2 == 0 { WHITE } else { p.c };
                    scr.line(x, y, mx as i32, my as i32, c);
                    scr.line(mx as i32, my as i32, *x2 as i32, *y2 as i32, c);
                }
                PK::Dot | PK::Shard => scr.fill(x, y, p.sz, p.sz, p.c),
                PK::Ember => {
                    let c = if q < 0.35 {
                        rgb(0xfce040)
                    } else if q < 0.7 {
                        rgb(0xfc9838)
                    } else {
                        rgb(0xa81000)
                    };
                    scr.fill(x, y, p.sz, p.sz, c);
                }
                PK::Burst(el, r) => {
                    let rad = (*r * q.sqrt()) as i32 + 1;
                    let (inner, outer) = match el {
                        Elem::Fire => (if q < 0.4 { rgb(0xfce040) } else { rgb(0xfc9838) }, rgb(0xd82800)),
                        Elem::Storm => (WHITE, rgb(0xfce040)),
                        Elem::Ice => (WHITE, rgb(0xa4e4fc)),
                        _ => (rgb(0x98d858), rgb(0x5c3c10)),
                    };
                    scr.blend_disc(x, y, rad, inner, (1.0 - q) * 0.75);
                    scr.ring(x, y, rad, outer);
                }
                PK::Crystal => {
                    let len = 3.0 + q * 9.0;
                    for i in 0..6 {
                        let a = i as f32 * PI / 3.0 + 0.3;
                        let (ex, ey) = (x + (a.cos() * len) as i32, y + (a.sin() * len) as i32);
                        scr.line(x + (a.cos() * len * 0.3) as i32, y + (a.sin() * len * 0.3) as i32, ex, ey, if q < 0.5 { WHITE } else { p.c });
                    }
                }
                PK::Glow(r) => scr.blend_disc(x, y, *r as i32, p.c, (1.0 - q) * 0.45),
            }
        }
    }
    /// Deterministic wobble for lightning (draw() can't consume the RNG through &self).
    fn rng_jitter(&self, k: i32) -> f32 {
        (((self.frame as i32 * 7 + k * 13) % 11) - 5) as f32
    }
    fn draw_msg(&self, scr: &mut Screen) {
        let Some((s, _)) = &self.msg else { return };
        let mut lines: Vec<String> = vec![];
        let mut line = String::new();
        for w in s.split(' ') {
            if line.len() + w.len() + 1 > 28 && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(w);
        }
        if !line.is_empty() {
            lines.push(line);
        }
        let h = lines.len() as i32 * 11 + 8;
        let y = H - h - 8;
        scr.fill(12, y, W - 24, h, BLACK);
        scr.frame_rect(12, y, W - 24, h, WHITE);
        for (i, l) in lines.iter().enumerate() {
            scr.text(l, 128, y + 5 + i as i32 * 11, WHITE, Align::Center, 8);
        }
    }
    fn scroll_imgs(&self, sc: &Scroll) -> (&Sprite, &Sprite) {
        match (&self.dungeon, sc.dun) {
            (Some(d), true) => (&d.rooms[sc.from].img, &d.rooms[sc.to].img),
            _ => (&self.rooms[sc.from].img, &self.rooms[sc.to].img),
        }
    }
    fn draw_play(&self, scr: &mut Screen) {
        scr.clip(0, HUD, W, H);
        if let Some(sc) = &self.scroll {
            let t = sc.t;
            let t = if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 };
            let (dx, dy) = (DIRS[sc.d].0, DIRS[sc.d].1);
            let (mut ox, mut oy, mut nx, mut ny) = (0, 0, 0, 0);
            if dx != 0 {
                ox = (-(dx as f32) * t * WF).round() as i32;
                nx = (dx as f32 * (1.0 - t) * WF).round() as i32;
            } else {
                let rh = (RR as i32 * TS) as f32;
                oy = (-(dy as f32) * t * rh).round() as i32;
                ny = (dy as f32 * (1.0 - t) * rh).round() as i32;
            }
            let (a, b) = self.scroll_imgs(sc);
            scr.blit(a, ox, HUD + oy, false, false);
            if !sc.dun {
                self.draw_room_objs(scr, sc.from, ox, oy);
            }
            scr.blit(b, nx, HUD + ny, false, false);
            if !sc.dun {
                self.draw_room_objs(scr, sc.to, nx, ny);
            }
            self.draw_hero(scr, nx as f32, ny as f32);
        } else {
            // The descent camera follows the mage by shifting the whole view.
            let base_oy = scr.oy;
            scr.oy += self.cam_y as i32;
            scr.blit(&self.cur_room().img, 0, HUD, false, false);
            if self.overworld() {
                self.draw_room_objs(scr, self.room, 0, 0);
            } else if self.in_lair == 0 {
                self.draw_dungeon_overlays(scr);
            }
            self.draw_items(scr);
            self.draw_hazards(scr);
            for e in &self.enemies {
                self.draw_enemy(scr, e);
            }
            self.draw_boss(scr);
            if self.mode != Mode::Dying {
                if self.mode == Mode::Descend && self.sink > 0.0 {
                    let feet = (self.pl.y + 6.0 - self.sink) as i32 + scr.oy;
                    scr.clip(0, HUD, W, feet.max(HUD));
                    self.draw_hero(scr, 0.0, 0.0);
                    scr.clip(0, HUD, W, H);
                } else {
                    self.draw_hero(scr, 0.0, 0.0);
                }
            }
            self.draw_bullets(scr);
            self.draw_parts(scr);
            scr.oy = base_oy;
        }
        scr.unclip();
        if self.in_lair > 0 {
            if let Some(b) = &self.boss {
                if b.alive && b.state != BState::Intro {
                    scr.fill(0, H - 13, W, 13, BLACK);
                    scr.text(b.name, 4, H - 11, b.el.light(), Align::Left, 8);
                    let bx = 132;
                    let bw = 256 - bx - 6;
                    scr.fill(bx, H - 10, bw, 6, rgb(0x300000));
                    scr.fill(bx, H - 10, (bw as f32 * b.hp / b.max).ceil() as i32, 6, rgb(0xf83800));
                    if b.armor > 0.0 {
                        scr.fill(bx, H - 4, (bw as f32 * b.armor / b.armor_max).ceil() as i32, 2, rgb(0xbcbcbc));
                    }
                    if b.phase > 1 {
                        scr.fill(bx - 5, H - 10, 3, 6, rgb(0xfc3c3c));
                    }
                }
            }
            if self.boss_dead && self.clear_t > 130 {
                scr.text("VICTORY!", 128, 110, rgb(0xb8f818), Align::Center, 16);
            }
        }
        self.draw_msg(scr);
        if self.fade > 0 {
            scr.blend(0, HUD, W, H - HUD, BLACK, (self.fade as f32 / 30.0).min(1.0));
        }
    }

    // ------------------------------------------------------------ HUD & maps
    fn draw_hud(&self, scr: &mut Screen) {
        scr.fill(0, 0, W, HUD, BLACK);
        scr.fill(0, HUD - 2, W, 1, rgb(0x5c4880));
        scr.text("HP", 4, 5, rgb(0xfc7460), Align::Left, 8);
        let per = ((self.s.max_hp + 41) / 42).max(2);
        let segs = (self.s.max_hp + per - 1) / per;
        let low = self.s.hp <= 6 && (self.frame >> 3) & 1 == 1;
        for i in 0..segs {
            let v = self.s.hp - i * per;
            let c = if v >= per {
                if low { WHITE } else { rgb(0xf83800) }
            } else if v > 0 {
                rgb(0xa81000)
            } else {
                rgb(0x301000)
            };
            scr.fill(24 + i * 4, 5, 3, 8, c);
        }
        scr.spr(&self.spr.coin.img, 197.0, 9.0, false);
        scr.text(&format!("{:04}", self.s.gold), 204, 5, rgb(0xfcbc3c), Align::Left, 8);
        scr.text("MP", 4, 19, rgb(0x3cbcfc), Align::Left, 8);
        scr.fill(24, 20, 44, 6, rgb(0x001030));
        scr.fill(24, 20, (44.0 * self.s.mp / self.s.max_mp as f32) as i32, 6, rgb(0x3cbcfc));
        scr.spr(&self.spr.apple.img, 78.0, 22.0, false);
        let hungry = self.s.food <= 25.0 && (self.frame >> 3) & 1 == 1;
        scr.fill(84, 20, 44, 6, rgb(0x301800));
        let fc = if hungry { WHITE } else if self.s.food <= 25.0 { rgb(0xd82800) } else { rgb(0xfc9838) };
        scr.fill(84, 20, (44.0 * self.s.food / 100.0) as i32, 6, fc);
        let el = self.el();
        scr.disc(139, 22, 3, el.main());
        scr.pset(138, 21, el.light());
        scr.text(el.name(), 145, 19, el.light(), Align::Left, 8);
        // Carried mana potions.
        scr.spr(&self.spr.potion.img, 191.0, 22.0, false);
        let pc = if self.s.potions > 0 { rgb(0x3cbcfc) } else { rgb(0x747474) };
        scr.text(&self.s.potions.to_string(), 197, 19, pc, Align::Left, 8);
        if self.dungeon.is_some() && self.in_lair == 0 {
            scr.spr(&self.spr.key.img, 213.0, 22.0, false);
            scr.text(&format!("X{}", self.dungeon_keys()), 219, 19, rgb(0xfcbc3c), Align::Left, 8);
        } else {
            let runes = (1..=5).filter(|&i| self.s.cleared[i]).count();
            scr.spr(&self.spr.gem.img, 213.0, 22.0, false);
            scr.text(&format!("{}/5", runes), 219, 19, rgb(0xf878f8), Align::Left, 8);
        }
        if self.muted {
            scr.text("M", 248, 5, rgb(0x747474), Align::Left, 8);
        }
    }
    fn draw_map(&self, scr: &mut Screen) {
        scr.blend(0, HUD, W, H - HUD, BLACK, 0.88);
        scr.text(&format!("- {} -", self.themes[self.rooms[self.room].theme].name), 128, HUD + 5, rgb(0xfcbc3c), Align::Center, 8);
        let (cw, ch) = (24, 17);
        let (ox, oy) = ((W - WW as i32 * cw) / 2, HUD + 18);
        for r in &self.rooms {
            if !r.visited {
                continue;
            }
            let (x, y) = (ox + r.x as i32 * cw, oy + r.y as i32 * ch);
            let c = self.themes[r.theme].map_col;
            scr.fill(x + 3, y + 3, cw - 6, ch - 6, c);
            if r.doors[2] {
                scr.fill(x + cw - 3, y + ch / 2 - 1, 6, 3, c);
            }
            if r.doors[3] {
                scr.fill(x - 3, y + ch / 2 - 1, 6, 3, c);
            }
            if r.doors[1] {
                scr.fill(x + cw / 2 - 1, y + ch - 3, 3, 6, c);
            }
            if r.doors[0] {
                scr.fill(x + cw / 2 - 1, y - 3, 3, 6, c);
            }
            if r.gate > 0 {
                let st = self.gate_state(r.gate);
                let col = [rgb(0x747474), WHITE, rgb(0xb8f818)][st as usize];
                let label = if r.gate == 6 { "X".to_string() } else { r.gate.to_string() };
                scr.text(&label, x + cw / 2 + 1, y + 5, col, Align::Center, 8);
            }
            let dot = |scr: &mut Screen, dx: i32, c: u32| scr.fill(x + cw / 2 - 2 + dx, y + ch / 2 - 2, 4, 4, c);
            if r.tank && !self.s.tanks.contains(&r.i) {
                dot(scr, 0, rgb(0xfc7460));
            }
            if r.cache && !self.s.caches.contains(&r.i) {
                dot(scr, 0, rgb(0xfcbc3c));
            }
            if let Some(se) = r.shrine {
                dot(scr, -6, EL_LIGHT[se]);
            }
            if r.chest.is_some() && !self.s.opened.contains(&r.i) {
                dot(scr, 6, rgb(0xa85020));
            }
            if r.special == SP_MONOLITH {
                scr.text("M", x + cw / 2 + 1, y + 5, rgb(0xa4e4fc), Align::Center, 8);
            }
            if r.special == SP_SHOP {
                scr.text("S", x + cw / 2 + 1, y + 5, rgb(0xfcbc3c), Align::Center, 8);
            }
            if r.i == self.room && (self.frame >> 4) & 1 == 1 {
                scr.frame_rect(x + 1, y + 1, cw - 2, ch - 2, WHITE);
            }
        }
        let y = oy + WH as i32 * ch + 6;
        let el = self.el();
        scr.text(&format!("{} MAGIC: {}", el.name(), SPELL_NAMES[el.idx()]), 128, y, el.light(), Align::Center, 8);
        scr.text(&format!("MAGIC LV{}   RUNES {}/5", self.s.spell_lv, (1..=5).filter(|&i| self.s.cleared[i]).count()), 128, y + 12, rgb(0xf878f8), Align::Center, 8);
        scr.text("M MONOLITH  S SHOP  1-5 LAIRS", 128, y + 26, rgb(0x747474), Align::Center, 8);
    }
    fn draw_dungeon_map(&self, scr: &mut Screen) {
        let Some(d) = &self.dungeon else { return };
        scr.blend(0, HUD, W, H - HUD, BLACK, 0.88);
        scr.text(&format!("- {} -", dungeon_name(d.n)), 128, HUD + 6, rgb(0xfcbc3c), Align::Center, 8);
        let prog = self.s.dprog[d.n];
        let (cw, ch) = (48, 30);
        let (ox, oy) = (128 - cw * 3 / 2, HUD + 26);
        let col = self.themes[dungeon::dungeon_theme(d.n)].map_col;
        let labels = ["ENTRY", "HALL", "WEST", "EAST", "STAIRS", "FEAST", "FOOD"];
        for (i, r) in d.rooms.iter().enumerate() {
            let (x, y) = (ox + r.x as i32 * cw, oy + r.y as i32 * ch);
            // The pantry stays off the map until it's found.
            if !d.has[i] || (i == R_PANTRY && !r.visited) {
                continue;
            }
            if !r.visited {
                scr.frame_rect(x + 4, y + 4, cw - 8, ch - 8, rgb(0x303030));
                continue;
            }
            scr.fill(x + 4, y + 4, cw - 8, ch - 8, col);
            scr.text(labels[i], x + cw / 2 + 1, y + 11, BLACK, Align::Center, 8);
            let done = match i {
                R_WEST => prog & dungeon::D_WEST != 0,
                R_EAST => prog & dungeon::D_EAST != 0,
                _ => false,
            };
            if done {
                scr.fill(x + cw - 12, y + 6, 4, 4, rgb(0x58d854));
            }
            if i == d.cur && (self.frame >> 4) & 1 == 1 {
                scr.frame_rect(x + 2, y + 2, cw - 4, ch - 4, WHITE);
            }
        }
        // Connections; the hall's north door shows its lock.
        let hub = (ox + cw + cw / 2, oy + ch);
        let door_col = if prog & dungeon::D_DOOR != 0 { col } else { rgb(0xfcbc3c) };
        scr.fill(hub.0 - 2, hub.1 - 4, 4, 8, door_col);
        scr.fill(ox + cw - 4, oy + ch + ch / 2 - 1, 8, 3, col);
        scr.fill(ox + cw * 2 - 4, oy + ch + ch / 2 - 1, 8, 3, col);
        scr.fill(hub.0 - 1, oy + ch * 2 - 4, 3, 8, col);
        let _ = (R_ENTRY, R_HUB);
        if d.has[R_FEAST] {
            scr.fill(ox + cw - 4, oy + ch * 2 + ch / 2 - 1, 8, 3, col);
        }
        if d.has[R_PANTRY] && d.rooms[R_PANTRY].visited {
            scr.fill(ox + cw * 2 - 4, oy + ch * 2 + ch / 2 - 1, 8, 3, col);
        }
        let y = oy + ch * 3 + 4;
        scr.text(&format!("KEYS: {}", self.dungeon_keys()), 128, y, rgb(0xfcbc3c), Align::Center, 8);
        let goal = if prog & dungeon::D_EAST == 0 {
            "BREAK THE SEAL IN THE EAST"
        } else if prog & dungeon::D_DOOR == 0 {
            "FIND THE KEY IN THE WEST"
        } else {
            "DESCEND THE STAIRS"
        };
        scr.text(goal, 128, y + 14, WHITE, Align::Center, 8);
    }

    // ------------------------------------------------------------ menus
    fn draw_title(&self, scr: &mut Screen) {
        self.draw_stars(scr);
        scr.text("ELEMENTAL", 128, 30, rgb(0x3cbcfc), Align::Center, 16);
        scr.text("LEGENDS", 130, 52, rgb(0x881400), Align::Center, 32);
        scr.text("LEGENDS", 128, 50, rgb(0xfcbc3c), Align::Center, 32);
        let e = ((self.frame / 60) % 4) as usize;
        let s = &self.spr.mage_d[e];
        let bob = ((self.frame as f32 * 0.06).sin() * 2.0) as i32;
        scr.blit_scaled(&s.img, 128 - 16, 90 + bob, 2);
        let opts = self.menu_opts();
        for (i, o) in opts.iter().enumerate() {
            let sel = i == self.menu;
            let prefix = if sel && (self.frame >> 4) & 1 == 1 { "> " } else { "  " };
            scr.text(&format!("{}{}", prefix, o), 92, 136 + i as i32 * 12, if sel { WHITE } else { rgb(0x747474) }, Align::Left, 8);
        }
        scr.text("MOVE: D-PAD   CAST: A / Z", 128, 184, rgb(0x747474), Align::Center, 8);
        scr.text("SPELL: B / X  MAP: START", 128, 196, rgb(0x747474), Align::Center, 8);
        scr.text("POTION: Y / C  STRAFE: HOLD A", 128, 208, rgb(0x747474), Align::Center, 8);
        let c = if (self.frame >> 5) & 1 == 1 { rgb(0xfcbc3c) } else { rgb(0xfc7460) };
        scr.text("PRESS START", 128, 224, c, Align::Center, 8);
    }
    fn draw_choose(&self, scr: &mut Screen) {
        self.draw_stars(scr);
        scr.text("CHOOSE YOUR ELEMENT", 128, 24, WHITE, Align::Center, 8);
        for i in 0..4 {
            let cx = 32 + i as i32 * 64;
            let sel = i == self.menu;
            let bob = if sel { ((self.frame as f32 * 0.15).sin() * 3.0) as i32 } else { 0 };
            if sel {
                scr.frame_rect(cx - 24, 48, 48, 60, EL_LIGHT[i]);
            }
            scr.blit_scaled(&self.spr.mage_d[i].img, cx - 16, 58 + bob, 2);
            scr.text(Elem::from_idx(i).name(), cx, 96, if sel { EL_LIGHT[i] } else { rgb(0x747474) }, Align::Center, 8);
        }
        let desc: [[&str; 3]; 4] = [
            ["FIREBOLTS SET FOES ABLAZE", "SPELL: FLAME RING", "STRONG VS ICE FOES"],
            ["SHARDS CHILL, THEN FREEZE", "SPELL: FROST NOVA", "STRONG VS FIRE FOES"],
            ["FAST PIERCING BOLTS", "SPELL: CHAIN BOLT", "STRONG VS EARTH FOES"],
            ["HEAVY ROCKS SMASH ICE", "SPELL: QUAKE", "STRONG VS STORM FOES"],
        ];
        for (j, l) in desc[self.menu].iter().enumerate() {
            scr.text(l, 128, 126 + j as i32 * 14, if j == 2 { rgb(0xfce040) } else { WHITE }, Align::Center, 8);
        }
        scr.text("FIND ORB SHRINES TO", 128, 180, rgb(0x747474), Align::Center, 8);
        scr.text("CHANGE ELEMENT LATER", 128, 192, rgb(0x747474), Align::Center, 8);
        scr.text("< >  CHOOSE    START  OK", 128, 220, rgb(0xfcbc3c), Align::Center, 8);
    }
    fn draw_intro(&self, scr: &mut Screen) {
        self.draw_stars(scr);
        let mut chars = self.t / 2;
        for (i, l) in INTRO.iter().enumerate() {
            if chars <= 0 {
                break;
            }
            let n = (chars as usize).min(l.len());
            chars -= l.len() as i32;
            let c = if i >= 7 { rgb(0xfcbc3c) } else { WHITE };
            scr.text(&l[..n], 128, 36 + i as i32 * 14, c, Align::Center, 8);
        }
        scr.text("START: SKIP", 128, 224, rgb(0x747474), Align::Center, 8);
    }
    fn draw_gameover(&self, scr: &mut Screen) {
        scr.text("GAME OVER", 128, 80, rgb(0xd82800), Align::Center, 16);
        for (i, o) in ["CONTINUE", "TITLE"].iter().enumerate() {
            let sel = i == self.menu;
            scr.text(&format!("{}{}", if sel { "> " } else { "  " }, o), 96, 130 + i as i32 * 14, if sel { WHITE } else { rgb(0x747474) }, Align::Left, 8);
        }
        scr.text("CONTINUING COSTS HALF", 128, 190, rgb(0x747474), Align::Center, 8);
        scr.text("YOUR GOLD", 128, 202, rgb(0x747474), Align::Center, 8);
    }
    fn draw_reward(&self, scr: &mut Screen) {
        self.draw_stars(scr);
        let n = self.gate_n.clamp(1, 6);
        let b = bosses::make_boss(n);
        scr.text(b.name, 128, 50, b.el.light(), Align::Center, 8);
        scr.text("DEFEATED!", 128, 66, rgb(0xb8f818), Align::Center, 16);
        if self.t > 30 {
            scr.text(&format!("YOU CLAIM RUNE {} OF 5", n), 128, 106, rgb(0xf878f8), Align::Center, 8);
        }
        if self.t > 40 {
            scr.text("THE MONOLITH WILL SHINE", 128, 118, rgb(0xa4e4fc), Align::Center, 8);
        }
        if self.t > 45 {
            scr.text(REWARDS[n], 128, 136, rgb(0xfcbc3c), Align::Center, 8);
        }
        if self.t > 55 {
            scr.text("MAXIMUM LIFE UP", 128, 150, rgb(0xfc7460), Align::Center, 8);
        }
        if self.t > 65 {
            let s = if n < 5 { format!("LAIR {} IS NOW OPEN", n + 1) } else { "THE DARK TOWER IS OPEN".to_string() };
            scr.text(&s, 128, 168, rgb(0x3cbcfc), Align::Center, 8);
        }
        if self.t > 60 && (self.frame >> 4) & 1 == 1 {
            scr.text("PRESS START", 128, 206, WHITE, Align::Center, 8);
        }
    }
    fn draw_victory(&self, scr: &mut Screen) {
        self.draw_stars(scr);
        let secs = self.s.time / 60;
        let hms = format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60);
        let lines: [(String, u32); 11] = [
            ("THE DARK SORCERER FALLS.".into(), WHITE),
            ("LIGHT RETURNS TO".into(), WHITE),
            ("THE REALM.".into(), WHITE),
            ("".into(), WHITE),
            ("HAIL, ELEMENTAL LEGEND!".into(), rgb(0xfcbc3c)),
            ("".into(), WHITE),
            (format!("TIME  {}", hms), rgb(0x3cbcfc)),
            (format!("GOLD  {}", self.s.gold), rgb(0x3cbcfc)),
            (format!("HEARTS {}", self.s.tanks.len()), rgb(0x3cbcfc)),
            ("".into(), WHITE),
            ("THANK YOU FOR PLAYING".into(), rgb(0xb8f818)),
        ];
        for (i, (s, c)) in lines.iter().enumerate() {
            if self.t > i as i32 * 20 {
                scr.text(s, 128, 36 + i as i32 * 14, *c, Align::Center, 8);
            }
        }
        if self.t > 240 && (self.frame >> 4) & 1 == 1 {
            scr.text("PRESS START", 128, 214, WHITE, Align::Center, 8);
        }
    }
}
