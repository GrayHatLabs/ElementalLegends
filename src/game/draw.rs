//! Rendering: the play field, entities, effects, HUD, maps and menus.
use super::bosses::{draw_flames, draw_ice_block, BState};
use super::dungeon::{dungeon_name, Obj, OK, R_EAST, R_ENTRY, R_FEAST, R_HUB, R_PANTRY, R_STAIRS, R_WEST, STAIRS_C, STAIRS_R};
use super::*;

impl Game {
    pub fn draw(&mut self, scr: &mut Screen) {
        scr.ui();
        scr.unclip();
        scr.ox = 0;
        scr.oy = 0;
        scr.clear();
        if self.shake > 0 {
            // Derived from the frame counter, not the game RNG: drawing must never
            // change gameplay (the self-test and real play would diverge otherwise).
            let h = (self.frame.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) as i32;
            scr.ox = (h & 7) % 5 - 2;
            scr.oy = ((h >> 3) & 7) % 5 - 2;
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
                self.draw_status_icons(scr);
                if self.paused {
                    if self.in_lair > 0 {
                        scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, 0.6);
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
        scr.ui();
        scr.unclip();
        if self.flash > 0 {
            scr.blend_screen(0, 0, SW, SH, WHITE, self.flash as f32 / 20.0);
        }
    }

    /// Starfield across the whole screen (stars live in 256-wide logic space).
    pub(super) fn draw_stars(&self, scr: &mut Screen) {
        for s in &self.stars {
            let c = if s.s > 1.2 { WHITE } else if s.s > 0.7 { rgb(0x747474) } else { rgb(0x3c3c7c) };
            let x = (s.x * SW as f32 / W as f32) as i32;
            scr.fill_screen(x, s.y as i32, 1, 1, c);
        }
    }

    // ------------------------------------------------------------ world
    /// Draws a generated object sprite (sheet `name`, anim `idle`) with its base centred on
    /// (x, y) in logic units. Returns false when the art is missing so callers can fall back.
    pub(super) fn obj_hd(&self, scr: &mut Screen, name: &str, x: f32, y: f32) -> bool {
        let Some(sh) = self.art.sheet(name) else { return false };
        let Some(a) = sh.anim("idle") else { return false };
        scr.spr_hd_anchor(a.at(self.frame as u32), x, y, sh.cell.0 / 2, sh.cell.1 - 2, false, Tint::None);
        true
    }
    fn draw_shrine(&self, scr: &mut Screen, x: i32, y: i32, el: Elem) {
        if !self.obj_hd(scr, "obj_shrine_pedestal", x as f32, y as f32 + 12.0) {
            scr.fill(x - 7, y + 4, 14, 6, rgb(0x747474));
            scr.fill(x - 7, y + 4, 14, 1, rgb(0xbcbcbc));
            scr.fill(x - 5, y + 10, 10, 2, rgb(0x404040));
        }
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
        self.draw_house(scr, "obj_cottage", 128 + ox, HUD + 16 + oy, (0xa81000, 0x681008));
    }
    /// A village house whose base (ground line) is at y + 32; `roof` = (colour, dark edge).
    fn draw_house(&self, scr: &mut Screen, art: &str, x: i32, y: i32, roof: (u32, u32)) {
        if self.obj_hd(scr, art, x as f32, y as f32 + 32.0) {
            return;
        }
        scr.fill(x - 28, y + 10, 56, 22, rgb(0x8c6c4c));
        for i in 0..5 {
            scr.fill(x - 28 + i * 14, y + 10, 2, 22, rgb(0x5c3c20));
        }
        scr.fill(x - 28, y + 20, 56, 2, rgb(0x5c3c20));
        for row in 0..14 {
            let w = 34 - row * 34 / 14;
            scr.fill(x - w, y + 10 - row, w * 2, 1, if row == 0 { rgb(roof.1) } else { rgb(roof.0) });
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
    /// The inn with its innkeeper and the notice board (the shop is drawn separately).
    fn draw_village(&self, scr: &mut Screen, ox: i32, oy: i32) {
        use super::village::{BOARD, INNKEEPER};
        let inn_x = ((*INN_COLS.start() + *INN_COLS.end() + 1) as i32 * TS / 2) + ox;
        let inn_base = HUD + (*INN_ROWS.end() as i32 + 1) * TS + oy;
        self.draw_house(scr, "obj_inn", inn_x, inn_base - 32, (0x8c5020, 0x5c3410));
        let (kx, ky) = (INNKEEPER.0 + ox as f32, INNKEEPER.1 + oy as f32);
        scr.blend_ellipse(kx as i32, ky as i32 + 7, 6, 2, BLACK, 0.3);
        let keeper = ["npc_innkeeper", "npc_merchant"]
            .iter()
            .find_map(|n| self.art.sheet(n).and_then(|sh| sh.anim("idle_down").map(|a| (sh.cell, a, *n))));
        match keeper {
            Some((cell, a, n)) => {
                let fx = if n == "npc_merchant" { Tint::Mix(rgb(0xd87838), 0.3) } else { Tint::None };
                scr.spr_hd_anchor(a.at(self.frame as u32), kx, ky + 7.0, cell.0 / 2, cell.1 - 2, false, fx);
            }
            None => scr.spr(&self.spr.mage_d[4][0].img, kx, ky, false),
        }
        let (bx, by) = (BOARD.0 as i32 + ox, BOARD.1 as i32 + 8 + oy);
        if !self.obj_hd(scr, "obj_notice_board", bx as f32, by as f32) {
            scr.fill(bx - 7, by - 16, 2, 16, rgb(0x5c3410));
            scr.fill(bx + 5, by - 16, 2, 16, rgb(0x5c3410));
            scr.fill(bx - 9, by - 18, 18, 11, rgb(0x8c5020));
            scr.fill(bx - 9, by - 18, 18, 1, rgb(0xb87838));
            for &(px, py) in &[(-7, -16), (-1, -15), (3, -16), (-5, -11)] {
                scr.fill(bx + px, by + py, 4, 4, rgb(0xf0e0b8));
            }
        }
    }
    pub(super) fn draw_room_objs(&self, scr: &mut Screen, ri: usize, ox: i32, oy: i32) {
        let r = &self.rooms[ri];
        let (gx, gy) = r.center();
        let (x, y) = (gx as i32 + ox, gy as i32 + oy);
        let bob = if (self.frame >> 4) & 1 == 1 { 1.0 } else { 0.0 };
        if r.gate > 0 {
            self.draw_building(scr, r.gate, ox, oy);
        }
        if r.special == SP_MONOLITH {
            self.draw_monolith(scr, ox, oy, 5, true);
        }
        if r.tank && !self.s.tanks.contains(&ri) {
            match self.item_named("heart_container") {
                Some(img) => scr.spr_hd(img, x as f32, y as f32 - bob, false),
                None => scr.spr(&self.spr.big_heart.img, x as f32, y as f32 - bob, false),
            }
        }
        if r.cache && !self.s.caches.contains(&ri) {
            match self.item_named("gold_pile") {
                Some(img) => scr.spr_hd(img, x as f32, y as f32, false),
                None => scr.spr(&self.spr.hoard.img, x as f32, y as f32, false),
            }
        }
        if let Some((cx, cy, _, _)) = r.chest {
            let open = self.s.opened.contains(&ri);
            match self.item_named(if open { "chest_open" } else { "chest_closed" }) {
                Some(img) => scr.spr_hd(img, cx + ox as f32, cy + oy as f32, false),
                None => scr.spr(&if open { &self.spr.chest_open } else { &self.spr.chest }.img, cx + ox as f32, cy + oy as f32, false),
            }
        }
        if let Some(se) = r.shrine {
            self.draw_shrine(scr, x, y, Elem::from_idx(se));
        }
        if r.special == SP_SHOP {
            self.draw_village(scr, ox, oy);
            self.draw_cottage(scr, ox, oy);
            let (mx, my) = (128.0 + ox as f32, HUDF + 58.0 + oy as f32);
            match self.art.sheet("npc_merchant").and_then(|sh| sh.anim("idle_down").map(|a| (sh.cell, a))) {
                Some((cell, a)) => {
                    scr.blend_ellipse(mx as i32, my as i32 + 7, 6, 2, BLACK, 0.3);
                    scr.spr_hd_anchor(a.at(self.frame as u32), mx, my + 7.0, cell.0 / 2, cell.1 - 2, false, Tint::None);
                }
                None => scr.spr(&self.spr.mage_d[4][0].img, mx, my, false),
            }
            let prices = self.shop_prices();
            for i in 0..SHOP_ITEMS {
                let px = Self::shop_x(i) as i32 + ox;
                let py = GATE_Y as i32 + oy;
                scr.fill(px - 7, py + 4, 14, 5, rgb(0x6c3c10));
                scr.fill(px - 7, py + 4, 14, 1, rgb(0xa86030));
                let (s, name) = match i {
                    0 => (&self.spr.meat, "meat"),
                    1 => (&self.spr.potion, "mana_potion"),
                    2 => (&self.spr.antidote, "antidote"),
                    3 => (&self.spr.potion, "bomb"),
                    4 => (&self.spr.potion, "elixir"),
                    _ => (&self.spr.big_heart, "heart_container"),
                };
                match self.item_named(name) {
                    Some(img) => scr.spr_hd(img, px as f32, py as f32 - 2.0, false),
                    None if i == 3 => draw_bomb_icon(scr, px, py - 2),
                    None if i == 4 => draw_elixir_icon(scr, px, py - 2),
                    None => scr.spr(&s.img, px as f32, py as f32 - 2.0, false),
                }
                scr.text(&prices[i].to_string(), px + 1, py + 12, rgb(0xfcbc3c), Align::Center, 8);
            }
        }
        self.draw_mini_room(scr, ri, ox, oy);
    }
    fn draw_obj(&self, scr: &mut Screen, o: &Obj) {
        let (fx, fy) = o.pos();
        let (x, y) = (fx as i32, fy as i32);
        match o.k {
            OK::Torch => {
                if !self.obj_hd(scr, "obj_brazier", fx, fy + 8.0) {
                    scr.fill(x - 4, y + 1, 8, 7, rgb(0x3c3c44));
                    scr.fill(x - 3, y + 1, 6, 1, rgb(0x5c5c64));
                    scr.fill(x - 6, y - 3, 12, 4, rgb(0x5c5c64));
                    scr.fill(x - 6, y - 3, 12, 1, rgb(0x8c8c98));
                }
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
            OK::Block if self.obj_hd(scr, "obj_push_block", fx, fy + 8.0) => {
                if o.on {
                    scr.blend(x - 8, y - 8, 16, 16, rgb(0x3cbcfc), 0.2);
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
            OK::Lever if self.obj_hd(scr, if o.on { "obj_lever_on" } else { "obj_lever_off" }, fx, fy + 7.0) => {}
            OK::Lever => {
                scr.fill(x - 6, y + 2, 12, 5, rgb(0x3c3c44));
                scr.fill(x - 6, y + 2, 12, 1, rgb(0x747480));
                let (ex, knob) = if o.on { (x + 6, rgb(0x58d854)) } else { (x - 6, rgb(0xd82800)) };
                scr.line(x, y + 2, ex, y - 7, rgb(0xa0a0ac));
                scr.disc(ex, y - 8, 2, knob);
            }
            OK::Chest => {
                match self.item_named(if o.on { "chest_open" } else { "chest_closed" }) {
                    Some(img) => scr.spr_hd(img, fx, fy, false),
                    None => scr.spr(&if o.on { &self.spr.chest_open } else { &self.spr.chest }.img, fx, fy, false),
                }
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
        // Generated wizard art: walk cycles / idle poses per direction, feet on the ground point.
        let sheet_name = ["mage_fire", "mage_ice", "mage_storm", "mage_earth"][e];
        if let Some(sh) = self.art.sheet(sheet_name) {
            let dir = match p.dir {
                b'u' => "up",
                b'd' => "down",
                _ => "side",
            };
            let name = format!("{}_{}", if p.moving { "walk" } else { "idle" }, dir);
            if let Some(anim) = sh.anim(&name) {
                let img = anim.at(p.walk as u32);
                let (x, y) = (p.x + ox, p.y + oy + 6.0);
                scr.blend_ellipse(x as i32, y as i32, 6, 2, BLACK, 0.35);
                let fx = if self.poison > 0 && self.frame % 20 < 14 { Tint::Mix(rgb(0x58d854), 0.45) } else { Tint::None };
                scr.spr_hd_anchor(img, x, y, sh.cell.0 / 2, sh.cell.1 - 2, p.dir == b'l', fx);
                if p.cast > 0 {
                    let sx = if p.dir == b'l' { x - 7.0 } else { x + 7.0 };
                    scr.blend_disc(sx as i32, (y - 17.0) as i32, 4, Elem::from_idx(e).light(), 0.35);
                    scr.disc(sx as i32, (y - 17.0) as i32, 2, Elem::from_idx(e).light());
                }
                return;
            }
        }
        let frame = if p.moving { ((p.walk / 7) % 4) as usize } else { 0 };
        let s = match p.dir {
            b'u' => &self.spr.mage_u[e][frame],
            b'd' => &self.spr.mage_d[e][frame],
            _ => &self.spr.mage_s[e][frame],
        };
        let bob = if frame % 2 == 1 { 1.0 } else { 0.0 };
        let (x, y) = (p.x + ox, p.y + oy - 2.0 - bob);
        scr.blend_ellipse(x as i32, (p.y + oy + 6.0) as i32, 6, 2, BLACK, 0.35);
        let img = if self.poison > 0 && self.frame % 20 < 14 { &s.sick } else { &s.img };
        scr.spr(img, x, y, p.dir == b'l');
        if p.cast > 0 {
            let sx = if p.dir == b'l' { x - 6.0 } else { x + 6.0 };
            scr.blend_disc(sx as i32, (y - 3.0) as i32, 4, Elem::from_idx(e).light(), 0.35);
            scr.disc(sx as i32, (y - 3.0) as i32, 2, Elem::from_idx(e).light());
        }
    }
    /// Generated monster art, if there is a sheet for this kind (and element). Four-direction
    /// sheets walk toward their heading; one-direction sheets flip to face their movement.
    /// Returns false to fall back to the code-drawn sprite.
    fn draw_enemy_hd(&self, scr: &mut Screen, e: &Enemy, bob: f32, flying: bool) -> bool {
        let el = ["fire", "ice", "storm", "earth", "neutral"][e.el.idx()];
        let name = match e.k {
            EK::Slime => format!("enemy_slime_{el}"),
            EK::Bat => format!("enemy_bat_{el}"),
            EK::Skeleton => format!("enemy_skeleton_{el}"),
            EK::Imp => format!("enemy_imp_{el}"),
            EK::Ghost => format!("enemy_ghost_{el}"),
            EK::Golem => format!("enemy_golem_{el}"),
            EK::Zombie => "enemy_zombie".into(),
            EK::Generator => "enemy_generator".into(),
            _ => return false,
        };
        self.draw_creature_hd(scr, e, &name, None, bob, flying, Tint::None)
    }
    /// Encounter creatures (mini-bosses) drawn from their generated sheets.
    fn draw_mini_hd(&self, scr: &mut Screen, e: &Enemy) -> bool {
        match e.k {
            EK::Treant => {
                scr.blend_ellipse(e.x as i32, (e.y + e.h / 2.0) as i32, 12, 3, BLACK, 0.35);
                self.draw_creature_hd(scr, e, "enemy_treant", None, 0.0, false, Tint::None)
            }
            EK::GraveLord => {
                scr.blend_ellipse(e.x as i32, (e.y + e.h / 2.0) as i32, 9, 3, BLACK, 0.35);
                self.draw_creature_hd(scr, e, "enemy_gravelord", None, 0.0, false, Tint::None)
            }
            EK::Dryad => {
                // Tell: she casts no shadow while disguised; revealed, her true colours show.
                let tint = if e.mode == 0 {
                    Tint::None
                } else {
                    scr.blend_ellipse(e.x as i32, (e.y + e.h / 2.0) as i32, 5, 2, BLACK, 0.3);
                    Tint::Mix(rgb(0x1c5c14), 0.35)
                };
                let bob = if e.mode == 0 && (e.t / 16) % 2 == 0 { -1.0 } else { 0.0 };
                self.draw_creature_hd(scr, e, "npc_dryad", None, bob, false, tint)
            }
            EK::HoardDragon => {
                let lift = if e.mode == 2 { e.timer as f32 } else { 0.0 };
                if lift > 0.0 {
                    scr.blend_ellipse(e.x as i32, (e.y + e.h / 2.0) as i32, 14, 4, BLACK, 0.3);
                }
                let anim = if e.mode == 0 { "idle" } else { "awake" };
                let ok = self.draw_creature_hd(scr, e, "enemy_hoarddragon", Some(anim), -lift, false, Tint::None);
                if ok && e.mode == 3 {
                    let f = if e.vx < 0.0 { -1.0 } else { 1.0 };
                    scr.blend_disc((e.x + f * 22.0) as i32, e.y as i32 - 6, 5, rgb(0xfc9838), 0.5);
                }
                ok
            }
            _ => false,
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_creature_hd(&self, scr: &mut Screen, e: &Enemy, name: &str, forced: Option<&str>, bob: f32, flying: bool, tint: Tint) -> bool {
        let Some(sh) = self.art.sheet(name) else { return false };
        let still = e.st.immobile();
        let moving = !still && e.vx.abs() + e.vy.abs() > 0.05;
        let (anim, flip) = if sh.anim("walk_down").is_some() {
            let (dx, dy) = if moving { (e.vx, e.vy) } else { (self.pl.x - e.x, self.pl.y - e.y) };
            let dir = if dx.abs() > dy.abs() {
                "side"
            } else if dy < 0.0 {
                "up"
            } else {
                "down"
            };
            let a = sh.anim(&format!("{}_{}", if moving { "walk" } else { "idle" }, dir));
            (a, dir == "side" && dx < 0.0)
        } else {
            let a = sh.anim(if moving { "move" } else { "idle" }).or_else(|| sh.anim("idle")).or_else(|| sh.anim("move"));
            (a, e.vx < -0.05 || (!moving && self.pl.x < e.x))
        };
        let anim = forced.and_then(|n| sh.anim(n)).or(anim);
        let Some(anim) = anim else { return false };
        let img = anim.at(if still { 0 } else { e.t as u32 });
        let fx = if e.flash > 0 && self.frame % 4 < 2 {
            Tint::Mix(WHITE, 0.75)
        } else if !matches!(tint, Tint::None) {
            tint
        } else if e.st.frozen() {
            Tint::Mix(rgb(0xc4ecfc), 0.6)
        } else if e.st.chilled() {
            Tint::Mix(rgb(0xa4e4fc), 0.35)
        } else {
            Tint::None
        };
        let feet = e.y + e.h / 2.0 - if flying { 6.0 } else { 0.0 } + bob;
        scr.spr_hd_anchor(img, e.x, feet, sh.cell.0 / 2, sh.cell.1 - 2, flip, fx);
        true
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
        if self.draw_mini_hd(scr, e) || self.draw_mini_enemy(scr, e) {
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
        // Ground shadow: flyers hover above theirs.
        let flying = matches!(e.k, EK::Bat | EK::Ghost);
        let (sy, sr, sa) = if flying { (e.y + e.h / 2.0 + 6.0, e.w / 2.0 - 2.0, 0.22) } else { (e.y + e.h / 2.0, e.w / 2.0, 0.32) };
        scr.blend_ellipse(e.x as i32, sy as i32, sr as i32, 2, BLACK, sa);
        if !self.draw_enemy_hd(scr, e, bob, flying) {
            scr.spr(img, e.x, e.y + bob, flip);
        }
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
                Shot::Apple => scr.spr(&self.spr.apple.img, b.x, b.y, b.age % 20 < 10),
                Shot::Thorn => {
                    let l = b.vx.hypot(b.vy).max(0.01);
                    let (ux, uy) = (b.vx / l, b.vy / l);
                    scr.line(x - (ux * 4.0) as i32, y - (uy * 4.0) as i32, x + (ux * 3.0) as i32, y + (uy * 3.0) as i32, rgb(0x5c3410));
                    scr.pset(x + (ux * 3.0) as i32, y + (uy * 3.0) as i32, rgb(0xb8f818));
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
    /// Generated item art from the `items` sheet (one row per item), if present.
    pub(super) fn item_hd(&self, k: IK) -> Option<&Sprite> {
        let names: &[&str] = match k {
            IK::Coin => &["coin"],
            IK::Gem => &["gem"],
            IK::Apple => &["apple"],
            IK::GoldenApple => &["golden_apple", "goldenapple"],
            IK::Bread => &["bread"],
            IK::Meat => &["roast", "roast_meat", "meat"],
            IK::Potion => &["potion", "mana_potion", "blue_potion"],
            IK::Antidote => &["antidote", "green_antidote"],
            IK::Heart => &["heart", "small_heart"],
            IK::Orb => &[],
        };
        names.iter().find_map(|n| self.item_named(n))
    }
    /// One row of the generated "items" sheet by name.
    pub(super) fn item_named(&self, name: &str) -> Option<&Sprite> {
        Some(self.art.sheet("items")?.anim(name)?.at(self.frame as u32))
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
                IK::GoldenApple => {
                    scr.blend_disc(it.x as i32, (it.y + bob) as i32, 8, rgb(0xfce040), 0.3);
                    &self.spr.golden_apple
                }
                IK::Antidote => &self.spr.antidote,
                IK::Orb => {
                    let (x, y) = (it.x as i32, (it.y + bob) as i32);
                    scr.blend_disc(x, y, 6, it.el.light(), 0.25);
                    scr.disc(x, y, 4, it.el.main());
                    scr.disc(x - 1, y - 1, 2, it.el.light());
                    scr.pset(x - 2, y - 2, WHITE);
                    continue;
                }
            };
            scr.blend_ellipse(it.x as i32, it.y as i32 + 5, 4, 1, BLACK, 0.3);
            let y = it.y + if it.kind == IK::Coin { 0.0 } else { bob };
            match self.item_hd(it.kind) {
                Some(img) => scr.spr_hd(img, it.x, y, false),
                None => scr.spr(&s.img, it.x, y, false),
            }
        }
    }
    pub(super) fn draw_parts(&self, scr: &mut Screen) {
        self.draw_parts_pass(scr, false);
        self.draw_parts_pass(scr, true);
    }
    fn draw_parts_pass(&self, scr: &mut Screen, text: bool) {
        for p in self.parts.iter().filter(|p| matches!(p.kind, PK::Text(_)) == text) {
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
        // Full-width SNES dialogue box with a double border.
        for w in s.split(' ') {
            if line.len() + w.len() + 1 > 35 && !line.is_empty() {
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
        let (sx0, sy0) = (scr.sx, scr.sy);
        scr.sx = 0;
        scr.sy = 0;
        let h = lines.len() as i32 * 11 + 10;
        // Like A Link to the Past: the box moves to the top when the mage is low on screen.
        let mage_y = HUD_PX as f32 + (self.pl.y - self.cam.1) * ZOOM;
        let low = self.mode == Mode::Play && mage_y > (SH - h - 20) as f32;
        let y = if low { HUD_PX + 6 } else { SH - h - 6 };
        scr.blend(8, y, SW - 16, h, rgb(0x080418), 0.88);
        scr.frame_rect(8, y, SW - 16, h, rgb(0xd8d0f8));
        scr.frame_rect(10, y + 2, SW - 20, h - 4, rgb(0x5c4880));
        for (i, l) in lines.iter().enumerate() {
            scr.text(l, SW / 2, y + 6 + i as i32 * 11, WHITE, Align::Center, 8);
        }
        scr.sx = sx0;
        scr.sy = sy0;
    }
    fn scroll_rooms(&self, sc: &Scroll) -> (&Room, &Room) {
        match (&self.dungeon, sc.dun) {
            (Some(d), true) => (&d.rooms[sc.from], &d.rooms[sc.to]),
            _ => (&self.rooms[sc.from], &self.rooms[sc.to]),
        }
    }
    /// The room's ground: native 24 px tiles when the theme has them, else the scaled
    /// code-drawn image. (ox, oy) offsets the room in logic units (area slides).
    pub(super) fn draw_room_img(&self, scr: &mut Screen, room: &Room, ox: i32, oy: i32) {
        match &room.img_hd {
            Some(hd) => scr.blit_hd(hd, ox as f32, (HUD + oy) as f32, false),
            None => scr.blit(&room.img, ox, HUD + oy, false, false),
        }
    }
    fn draw_play(&self, scr: &mut Screen) {
        scr.world(self.cam.0, self.cam.1);
        scr.clip_screen(0, HUD_PX, SW, SH);
        scr.set_base_clip();
        if let Some(sc) = &self.scroll {
            // SNES slide: both areas sit side by side in the old area's frame and the
            // camera glides from the old view to the new one.
            let t = sc.t;
            let t = if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 };
            let (ex, ey) = (sc.cam1.0 + sc.nx, sc.cam1.1 + sc.ny);
            scr.world(sc.cam0.0 + (ex - sc.cam0.0) * t, sc.cam0.1 + (ey - sc.cam0.1) * t);
            let (nx, ny) = (sc.nx as i32, sc.ny as i32);
            let (a, b) = self.scroll_rooms(sc);
            self.draw_room_img(scr, a, 0, 0);
            if !sc.dun {
                self.draw_room_objs(scr, sc.from, 0, 0);
            }
            self.draw_room_img(scr, b, nx, ny);
            if !sc.dun {
                self.draw_room_objs(scr, sc.to, nx, ny);
            }
            self.draw_hero(scr, sc.nx, sc.ny);
            if sc.dun {
                let light = vec![(self.pl.x + sc.nx, self.pl.y + sc.ny - 2.0, 88.0, 1.0)];
                self.apply_light(scr, &light, 0.5);
            }
        } else {
            // The descent camera follows the mage by shifting the whole view.
            let base_oy = scr.oy;
            scr.oy += self.pan_y as i32;
            self.draw_room_img(scr, self.cur_room(), 0, 0);
            if self.overworld() {
                self.draw_room_objs(scr, self.room, 0, 0);
            } else if self.in_lair == 0 {
                self.draw_dungeon_overlays(scr);
            }
            self.draw_items(scr);
            self.draw_bombs(scr, false);
            self.draw_hazards(scr);
            for e in &self.enemies {
                self.draw_enemy(scr, e);
            }
            self.draw_boss(scr);
            if self.mode != Mode::Dying {
                if self.mode == Mode::Descend && self.sink > 0.0 {
                    let feet = scr.ty(self.pl.y + 6.0 - self.sink);
                    scr.clip_screen(0, HUD_PX, SW, feet.max(HUD_PX));
                    self.draw_hero(scr, 0.0, 0.0);
                    scr.reset_clip();
                } else {
                    self.draw_hero(scr, 0.0, 0.0);
                }
            }
            self.draw_bombs(scr, true);
            self.draw_bullets(scr);
            self.draw_parts_pass(scr, false);
            self.draw_ambience(scr);
            self.draw_lighting(scr);
            self.draw_parts_pass(scr, true);
            scr.oy = base_oy;
        }
        scr.ui();
        scr.unclip();
        if self.in_lair > 0 {
            if let Some(b) = &self.boss {
                if b.alive && b.state != BState::Intro {
                    scr.fill_screen(0, SH - 13, SW, 13, BLACK);
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
            scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, (self.fade as f32 / 30.0).min(1.0));
        }
    }

    // ------------------------------------------------------------ ambience
    /// Region atmosphere drawn in screen space (so it doesn't depend on the area size):
    /// falling leaves in Greenwood, mist in the Old Crypt, fireflies in Mirefen and rising
    /// embers in Emberpeak. Purely a function of the frame counter and camera: no RNG,
    /// no game state.
    fn draw_ambience(&self, scr: &mut Screen) {
        if !self.overworld() || self.rooms[self.room].special != SP_NONE {
            return;
        }
        let f = self.frame as f32;
        let (cx, cy) = ((self.cam.0 * ZOOM) as i32, (self.cam.1 * ZOOM) as i32);
        let hash = |i: u32| -> u32 {
            let mut h = i.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
            h ^= h >> 15;
            h = h.wrapping_mul(0x2c1b_3c6d);
            h ^ (h >> 12)
        };
        let (pw, ph) = (SW, SH - HUD_PX);
        // Parallax: particles drift with the camera at 60% speed so they feel in front.
        let wrap = |v: i32, m: i32| ((v % m) + m) % m;
        match self.rooms[self.room].theme {
            0 => {
                for i in 0..14u32 {
                    let h = hash(i);
                    let speed = 0.25 + (h % 100) as f32 / 300.0;
                    let x = wrap((h >> 8) as i32 % pw + ((f * 0.02 + (i as f32) * 7.0).sin() * 14.0) as i32 - cx * 3 / 5, pw);
                    let y = wrap((h >> 16) as i32 % ph + (f * speed) as i32 - cy * 3 / 5, ph) + HUD_PX;
                    let c = [rgb(0x78c030), rgb(0x9cd848), rgb(0xd8a030)][(h % 3) as usize];
                    let flip = ((f * 0.1 + i as f32).sin() > 0.0) as i32;
                    scr.fill_screen(x, y, 2 + flip, 2 - flip, c);
                }
            }
            1 => {
                for i in 0..5u32 {
                    let h = hash(i + 40);
                    let x = wrap((h >> 4) as i32 % (pw + 120) + (f * 0.15) as i32 - cx / 2, pw + 120) - 60;
                    let y = HUD_PX + 20 + (h >> 12) as i32 % (ph - 40);
                    let a = 0.05 + 0.04 * (f * 0.02 + i as f32).sin().abs();
                    scr.blend_screen(x - 40, y - 5, 80, 10, rgb(0xc8d0e0), a);
                    scr.blend_screen(x - 24, y - 9, 48, 18, rgb(0xc8d0e0), a * 0.7);
                }
            }
            2 => {
                for i in 0..16u32 {
                    let h = hash(i + 80);
                    let bx = (h >> 6) as i32 % pw;
                    let by = (h >> 14) as i32 % ph;
                    let x = wrap(bx + ((f * 0.013 + i as f32).sin() * 20.0) as i32 - cx * 3 / 5, pw);
                    let y = wrap(by + ((f * 0.017 + i as f32 * 2.0).cos() * 12.0) as i32 - cy * 3 / 5, ph) + HUD_PX;
                    let glow = ((f * 0.05 + i as f32 * 1.7).sin() * 0.5 + 0.5).powi(2);
                    if glow > 0.15 {
                        scr.blend_screen(x - 2, y - 2, 5, 5, rgb(0xd8f878), glow * 0.35);
                        scr.fill_screen(x, y, 1, 1, mix(rgb(0x98c830), rgb(0xf8fcb0), glow));
                    }
                }
            }
            3 => {
                for i in 0..18u32 {
                    let h = hash(i + 120);
                    let speed = 0.3 + (h % 100) as f32 / 180.0;
                    let x = wrap((h >> 8) as i32 % pw + ((f * 0.03 + i as f32).sin() * 6.0) as i32 - cx * 3 / 5, pw);
                    let y = wrap((h >> 16) as i32 % ph - (f * speed) as i32 - cy * 3 / 5, ph) + HUD_PX;
                    let c = [rgb(0xfce040), rgb(0xfc9838), rgb(0xd82800)][((f as u32 / 8 + i) % 3) as usize];
                    scr.fill_screen(x, y, 1, 1 + (h % 2) as i32, c);
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ lighting
    /// Dungeons and boss arenas are dark; light comes from the mage, torches, bolts,
    /// burning things, shrines, the stairs and the boss itself.
    fn draw_lighting(&self, scr: &mut Screen) {
        if self.overworld() {
            return;
        }
        let ambient = if self.in_lair > 0 { 0.38 } else { 0.5 };
        let f = self.frame as f32;
        let flick = |k: f32| 1.0 + (f * 0.31 + k).sin() * 0.03 + (f * 0.77 + k * 2.0).sin() * 0.02;
        let mut lights: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(48);
        lights.push((self.pl.x, self.pl.y - 2.0, 88.0 * flick(0.0), 1.0));
        if let (Some(d), 0) = (&self.dungeon, self.in_lair) {
            for o in d.objs[d.cur].iter().filter(|o| o.visible) {
                let (x, y) = o.pos();
                match o.k {
                    OK::Torch if o.on => lights.push((x, y - 6.0, 62.0 * flick(x), 1.0)),
                    OK::Shrine(_) => lights.push((x, y - 4.0, 42.0, 0.8)),
                    OK::Chest if !o.on => lights.push((x, y, 22.0, 0.5)),
                    _ => {}
                }
            }
            let room = &d.rooms[d.cur];
            if d.cur == R_STAIRS {
                let (sx, sy) = tile_center(STAIRS_C, STAIRS_R);
                let lit = room.tiles[STAIRS_R as usize][STAIRS_C as usize] == T_STAIRS;
                lights.push((sx + 8.0, sy + 8.0, if lit { 52.0 } else { 44.0 }, 0.8));
            }
            if d.cur == R_ENTRY {
                lights.push((128.0, HF - 6.0, 64.0, 0.75)); // daylight through the way out
            }
            if d.cur == R_FEAST {
                for row in [4, 8] {
                    let y = (HUD + row * TS - 5) as f32;
                    for x in [4 * TS + 5, 12 * TS - 4] {
                        lights.push((x as f32, y, 40.0 * flick(x as f32), 0.9));
                    }
                }
            }
        }
        for b in &self.pb {
            lights.push((b.x, b.y, if b.el == Elem::Fire { 36.0 } else { 24.0 }, 0.9));
        }
        for b in &self.eb {
            if matches!(b.style, Shot::Flame | Shot::Fireball | Shot::Dark) || b.el != Elem::Neutral {
                lights.push((b.x, b.y, 18.0, 0.6));
            }
        }
        for e in self.enemies.iter().filter(|e| e.spawn <= 0) {
            if e.st.burning() {
                lights.push((e.x, e.y, 30.0 * flick(e.x), 0.85));
            } else if e.k == EK::Ghost || e.st.frozen() {
                lights.push((e.x, e.y, 20.0, 0.4));
            }
        }
        if let Some(b) = self.boss.as_ref().filter(|b| !b.gone) {
            lights.push((b.x, b.y - b.z, 64.0, 0.7));
        }
        for p in self.parts.iter().take(160) {
            let life = p.life as f32 / p.max.max(1) as f32;
            match p.kind {
                PK::Burst(_, r) => lights.push((p.x, p.y, r * 3.0, life)),
                PK::Glow(r) => lights.push((p.x, p.y, r * 2.5, life * 0.8)),
                PK::Crystal => lights.push((p.x, p.y, 22.0, life * 0.6)),
                _ => {}
            }
        }
        self.apply_light(scr, &lights, ambient);
    }
    /// Light grid over the visible play field (screen cells), lit by logic-space lights.
    fn apply_light(&self, scr: &mut Screen, lights: &[(f32, f32, f32, f32)], ambient: f32) {
        const CELL: i32 = 8;
        let (gw, gh) = ((SW / CELL + 1) as usize, ((SH - HUD_PX) / CELL + 1) as usize);
        let mut grid = vec![ambient; gw * gh];
        for gy in 0..gh {
            for gx in 0..gw {
                let (x, y) = scr.to_world(gx as i32 * CELL, HUD_PX + gy as i32 * CELL);
                let mut bright = 0.0f32;
                for &(lx, ly, r, s) in lights {
                    // The zoomed-in view shows less of the world, so pools of light are kept
                    // tighter than in world units to keep the edges of the screen dark.
                    let r = r * 0.75;
                    let (dx, dy) = (x - lx, y - ly);
                    let d2 = dx * dx + dy * dy;
                    if d2 < r * r {
                        let q = 1.0 - d2.sqrt() / r;
                        bright = bright.max(q * q * (3.0 - 2.0 * q) * s);
                    }
                }
                grid[gy * gw + gx] = ambient * (1.0 - bright.min(1.0));
            }
        }
        scr.light_map(HUD_PX, &grid, gw, gh, CELL, rgb(0x06040e));
    }
    // ------------------------------------------------------------ HUD & maps
    fn draw_hud(&self, scr: &mut Screen) {
        // Gradient panel with a bevelled lower edge, across the full screen width.
        for y in 0..HUD_PX {
            scr.fill_screen(0, y, SW, 1, mix(rgb(0x100c20), rgb(0x2a2044), y as f32 / HUD_PX as f32));
        }
        scr.fill_screen(0, HUD_PX - 3, SW, 1, rgb(0x7c68b0));
        scr.fill_screen(0, HUD_PX - 2, SW, 1, rgb(0x5c4880));
        scr.fill_screen(0, HUD_PX - 1, SW, 1, rgb(0x08040c));
        // SNES-style layout across the full 320-pixel width (no centring).
        scr.ui();
        scr.sx = 0;
        let el = self.el();
        // Element orb with the magic meter beside it (vertical, like A Link to the Past).
        scr.disc(12, 14, 8, rgb(0x08040c));
        scr.disc(12, 14, 7, el.main());
        scr.disc(10, 12, 3, el.light());
        scr.pset(9, 11, WHITE);
        let mp = (self.s.mp / self.s.max_mp as f32).clamp(0.0, 1.0);
        scr.frame_rect(23, 3, 8, 25, rgb(0x08040c));
        scr.fill(24, 4, 6, 23, rgb(0x001030));
        let h = (23.0 * mp).round() as i32;
        scr.fill(24, 27 - h, 6, h, rgb(0x3cbcfc));
        scr.fill(24, 27 - h, 2, h, rgb(0xa4e4fc));
        scr.text(el.name(), 35, 3, el.light(), Align::Left, 8);
        // Food bar.
        scr.spr(&self.spr.apple.img, 38.0, 20.0, false);
        let hungry = self.s.food <= 25.0 && (self.frame >> 3) & 1 == 1;
        let fc = if hungry { WHITE } else if self.s.food <= 25.0 { rgb(0xd82800) } else { rgb(0xfc9838) };
        scr.frame_rect(44, 16, 50, 8, rgb(0x08040c));
        scr.fill(45, 17, 48, 6, rgb(0x301800));
        let w = (48.0 * (self.s.food / 100.0).clamp(0.0, 1.0)) as i32;
        scr.fill(45, 17, w, 6, fc);
        scr.fill(45, 17, w, 1, mix(fc, WHITE, 0.5));
        // Counters: the selected bag item in its box, runes (or keys in a dungeon), gold.
        use super::bag::Slot;
        let slot = self.slot();
        let n = self.slot_count(slot);
        scr.frame_rect(93, 11, 14, 18, rgb(0x5c4880));
        match slot {
            Slot::Potion => scr.spr(&self.spr.potion.img, 100.0, 20.0, false),
            Slot::Antidote => scr.spr(&self.spr.antidote.img, 100.0, 20.0, false),
            Slot::Bomb => draw_bomb_icon(scr, 100, 20),
            Slot::Elixir => draw_elixir_icon(scr, 100, 20),
        }
        let pc = if n > 0 { rgb(0x3cbcfc) } else { rgb(0x747474) };
        scr.text(&n.to_string(), 109, 16, pc, Align::Left, 8);
        if self.dungeon.is_some() && self.in_lair == 0 {
            scr.spr(&self.spr.key.img, 126.0, 20.0, false);
            scr.text(&format!("X{}", self.dungeon_keys()), 132, 16, rgb(0xfcbc3c), Align::Left, 8);
        } else {
            let runes = (1..=5).filter(|&i| self.s.cleared[i]).count();
            scr.spr(&self.spr.gem.img, 126.0, 20.0, false);
            scr.text(&format!("{}/5", runes), 132, 16, rgb(0xf878f8), Align::Left, 8);
        }
        scr.spr(&self.spr.coin.img, 124.0, 7.0, false);
        scr.text(&format!("{:04}", self.s.gold), 130, 3, rgb(0xfcbc3c), Align::Left, 8);
        // Life: rows of hearts under a -LIFE- label, two HP per heart (half hearts show).
        let per = ((self.s.max_hp + 29) / 30).max(2);
        let hearts = (self.s.max_hp + per - 1) / per;
        let low = self.s.hp <= 6 && (self.frame >> 3) & 1 == 1;
        let (hx, hy) = (178, 11);
        scr.text("- LIFE -", hx + 66, 1, rgb(0xfc7460), Align::Center, 8);
        for i in 0..hearts {
            let (x, y) = (hx + (i % 15) * 9, hy + (i / 15) * 9);
            let v = self.s.hp - i * per;
            let full = if low { WHITE } else { rgb(0xd82800) };
            draw_heart(scr, x, y, rgb(0x401010), 7);
            if v >= per {
                draw_heart(scr, x, y, full, 7);
            } else if v > 0 {
                draw_heart(scr, x, y, full, 4);
            }
        }
        if self.muted {
            scr.text("M", 312, 22, rgb(0x747474), Align::Left, 8);
        }
        scr.ui();
    }
    fn draw_map(&self, scr: &mut Screen) {
        scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, 0.88);
        scr.text(&format!("- {} -", self.themes[self.rooms[self.room].theme].name), 128, HUD + 5, rgb(0xfcbc3c), Align::Center, 8);
        let (cw, ch) = (24, 15);
        let (ox, oy) = ((W - WW as i32 * cw) / 2, HUD + 16);
        for r in &self.rooms {
            if !r.visited {
                continue;
            }
            let (x, y) = (ox + r.x as i32 * cw, oy + r.y as i32 * ch);
            let (aw, ah) = (r.cw as i32 * cw, r.ch as i32 * ch);
            let c = self.themes[r.theme].map_col;
            scr.fill(x + 3, y + 3, aw - 6, ah - 6, c);
            for l in &r.links {
                let s = l.seg as i32;
                match l.d {
                    0 => scr.fill(x + s * cw + cw / 2 - 1, y - 3, 3, 6, c),
                    1 => scr.fill(x + s * cw + cw / 2 - 1, y + ah - 3, 3, 6, c),
                    2 => scr.fill(x + aw - 3, y + s * ch + ch / 2 - 1, 6, 3, c),
                    _ => scr.fill(x - 3, y + s * ch + ch / 2 - 1, 6, 3, c),
                }
            }
            // Features sit at the area's centre (single screens: the middle of the square).
            let (x, y) = (x + aw / 2 - cw / 2, y + ah / 2 - ch / 2);
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
                scr.text("V", x + cw / 2 + 1, y + 5, rgb(0xfcbc3c), Align::Center, 8);
            }
            // Discovered encounters: a red diamond until finished, then grey.
            if let Some(c) = self.mini_marker(r.mini) {
                let (mx, my) = (x + cw - 6, y + 4);
                scr.fill(mx, my - 2, 1, 5, c);
                scr.fill(mx - 2, my, 5, 1, c);
                scr.fill(mx - 1, my - 1, 3, 3, c);
            }
            if r.i == self.room && (self.frame >> 4) & 1 == 1 {
                scr.frame_rect(x + 1, y + 1, cw - 2, ch - 2, WHITE);
            }
        }
        let y = oy + WH as i32 * ch + 6;
        let el = self.el();
        scr.text(&format!("{} MAGIC: {}", el.name(), SPELL_NAMES[el.idx()]), 128, y, el.light(), Align::Center, 8);
        let slot = self.slot();
        scr.text(&format!("LV{} RUNES {}/5  BAG: < {} X{} >", self.s.spell_lv, (1..=5).filter(|&i| self.s.cleared[i]).count(), slot.name(), self.slot_count(slot)), 128, y + 12, rgb(0xf878f8), Align::Center, 8);
        scr.text("M MONOLITH V VILLAGE RED ENCOUNTER", 128, y + 26, rgb(0x747474), Align::Center, 8);
    }
    fn draw_dungeon_map(&self, scr: &mut Screen) {
        let Some(d) = &self.dungeon else { return };
        scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, 0.88);
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
    /// The generated wizard facing the viewer (walking or standing), for menus.
    fn mage_hd(&self, el: usize, walking: bool) -> Option<&Sprite> {
        let sh = self.art.sheet(["mage_fire", "mage_ice", "mage_storm", "mage_earth"][el.min(3)])?;
        let a = sh.anim(if walking { "walk_down" } else { "idle_down" })?;
        Some(a.at(self.frame as u32))
    }
    fn draw_title(&self, scr: &mut Screen) {
        self.draw_stars(scr);
        scr.text("ELEMENTAL", 128, 30, rgb(0x3cbcfc), Align::Center, 16);
        scr.text("LEGENDS", 130, 52, rgb(0x881400), Align::Center, 32);
        scr.text("LEGENDS", 128, 50, rgb(0xfcbc3c), Align::Center, 32);
        let e = ((self.frame / 60) % 4) as usize;
        let bob = ((self.frame as f32 * 0.06).sin() * 2.0) as i32;
        match self.mage_hd(e, true) {
            Some(img) => scr.blit_hd(img, (128 - img.w / 2) as f32, (90 + bob) as f32, false),
            None => scr.blit_scaled(&self.spr.mage_d[e][((self.frame / 8) % 4) as usize].img, 128 - 16, 90 + bob, 2),
        }
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
                scr.frame_rect(cx - 24, 48, 48, 68, EL_LIGHT[i]);
            }
            match self.mage_hd(i, sel) {
                Some(img) => scr.blit_hd(img, (cx - img.w / 2) as f32, (60 + bob) as f32, false),
                None => scr.blit_scaled(&self.spr.mage_d[i][if sel { ((self.frame / 8) % 4) as usize } else { 0 }].img, cx - 18, 56 + bob, 2),
            }
            scr.text(Elem::from_idx(i).name(), cx, 104, if sel { EL_LIGHT[i] } else { rgb(0x747474) }, Align::Center, 8);
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

/// A small heart for the life meter; `cols` < 7 draws only the left part (half heart).
fn draw_heart(scr: &mut Screen, x: i32, y: i32, c: u32, cols: i32) {
    const ROWS: [&str; 6] = [".XX.XX.", "XXXXXXX", "XXXXXXX", ".XXXXX.", "..XXX..", "...X..."];
    for (j, row) in ROWS.iter().enumerate() {
        for (i, ch) in row.bytes().enumerate() {
            if ch == b'X' && (i as i32) < cols {
                let shade = if j == 1 && i == 1 { mix(c, WHITE, 0.6) } else { c };
                scr.fill(x + i as i32, y + j as i32, 1, 1, shade);
            }
        }
    }
}
/// Code-drawn bag icons (until the items sheet has "bomb" / "elixir"), centred on (x, y).
pub(super) fn draw_bomb_icon(scr: &mut Screen, x: i32, y: i32) {
    scr.disc(x, y + 1, 5, rgb(0x2c2c3c));
    scr.disc(x - 2, y - 1, 1, rgb(0x8c8ca8));
    scr.fill(x - 1, y - 5, 3, 2, rgb(0x5c5c64));
    scr.line(x + 1, y - 5, x + 3, y - 8, rgb(0xa86030));
    scr.fill(x + 3, y - 9, 2, 2, rgb(0xfce040));
}
pub(super) fn draw_elixir_icon(scr: &mut Screen, x: i32, y: i32) {
    scr.fill(x - 1, y - 7, 3, 2, rgb(0xa86030));
    scr.fill(x - 2, y - 5, 5, 2, rgb(0xd8d8e8));
    scr.disc(x, y + 1, 5, rgb(0xd8d8e8));
    scr.disc(x, y + 1, 4, rgb(0xfcbc3c));
    scr.fill(x - 2, y - 1, 2, 2, rgb(0xfce078));
}
