//! Drawing for format 2 lairs and relics: new objects, coloured barriers, lava and hidden
//! bridges, the whip, floating, the relic held overhead, dark rooms and the lair map.
//! Generated art is used when its sheet exists; otherwise everything is code-drawn.
use super::dungeon::{Obj, OK};
use super::*;
use crate::keepdef::{Relic, Shutter};

impl Game {
    /// Like obj_hd, with a chosen animation row.
    pub(super) fn obj_hd_anim(&self, scr: &mut Screen, name: &str, anim: &str, x: f32, y: f32) -> bool {
        let Some(sh) = self.art.sheet(name) else { return false };
        let Some(a) = sh.anim(anim).or_else(|| sh.anim("idle")) else { return false };
        scr.spr_hd_anchor(a.at(self.frame as u32), x, y, sh.cell.0 / 2, sh.cell.1 - 2, false, Tint::None);
        true
    }
    /// A 16 px relic/key icon from the relic_items sheet, centred.
    pub(super) fn relic_icon(&self, scr: &mut Screen, name: &str, x: f32, y: f32, big: bool) -> bool {
        let sheet = if big { "relic_items_big" } else { "relic_items" };
        let Some(img) = self.art.sheet(sheet).and_then(|sh| sh.anim(name)).map(|a| a.at(0)) else { return false };
        scr.spr_hd(img, x, y, false);
        true
    }

    pub(super) fn draw_keep_obj(&self, scr: &mut Screen, o: &Obj) {
        let (fx, fy) = o.pos();
        let (x, y) = (fx as i32, fy as i32);
        let base = fy + 8.0;
        match o.k {
            OK::BigChest => {
                scr.blend_ellipse(x, y + 7, 13, 3, BLACK, 0.3);
                let art = if o.on { "obj_big_chest_open" } else { "obj_big_chest_closed" };
                if !self.obj_hd(scr, art, fx, base) {
                    scr.fill(x - 12, y - 8, 24, 16, rgb(0xa81000));
                    scr.fill(x - 12, y - 8, 24, 3, rgb(0xd82800));
                    scr.frame_rect(x - 12, y - 8, 24, 16, rgb(0x401008));
                    scr.fill(x - 12, y - 1, 24, 2, rgb(0xfcbc3c));
                    scr.fill(x - 2, y - 3, 4, 5, rgb(0xfce040));
                    if o.on {
                        scr.blend(x - 10, y - 12, 20, 6, rgb(0xfce040), 0.5);
                    }
                }
            }
            OK::Pot => {
                if !self.obj_hd(scr, "obj_pot", fx, base) {
                    scr.disc(x, y + 1, 6, rgb(0xa85c2c));
                    scr.fill(x - 3, y - 6, 6, 3, rgb(0x8c4c20));
                    scr.fill(x - 4, y - 2, 2, 4, rgb(0xd08848));
                }
            }
            OK::Crate => {
                if !self.obj_hd(scr, "obj_crate", fx, base) {
                    scr.fill(x - 7, y - 7, 14, 14, rgb(0x8c5c2c));
                    scr.frame_rect(x - 7, y - 7, 14, 14, rgb(0x4c2c10));
                    scr.line(x - 7, y - 7, x + 6, y + 6, rgb(0x5c3c18));
                }
            }
            OK::Boulder => {
                if !self.obj_hd(scr, "obj_boulder", fx, base) {
                    scr.disc(x, y, 7, rgb(0x6c6c78));
                    scr.disc(x - 2, y - 2, 3, rgb(0x9c9ca8));
                    scr.fill(x + 1, y + 2, 4, 2, rgb(0x3c3c48));
                }
            }
            OK::Post => {
                if !self.obj_hd(scr, "obj_whip_post", fx, base) {
                    scr.fill(x - 2, y - 8, 4, 15, rgb(0x6c4420));
                    scr.ring(x, y - 9, 3, rgb(0xfcbc3c));
                }
            }
            OK::IceBlock => {
                if !self.obj_hd(scr, "obj_ice_block", fx, base) {
                    scr.fill(x - 8, y - 8, 16, 16, rgb(0x78c8ec));
                    scr.fill(x - 8, y - 8, 16, 2, rgb(0xd4f4fc));
                    scr.frame_rect(x - 8, y - 8, 16, 16, rgb(0x3c8cbc));
                    scr.fill(x - 4, y - 3, 5, 1, WHITE);
                }
            }
            OK::Crystal(el) => {
                let art = format!("obj_crystal_{}", ["fire", "ice", "storm", "earth"][el.idx().min(3)]);
                if o.on {
                    scr.blend_disc(x, y - 6, 9, el.light(), 0.25 + (self.frame as f32 * 0.1).sin().abs() * 0.15);
                }
                if !self.obj_hd_anim(scr, &art, if o.on { "lit" } else { "idle" }, fx, base) {
                    let c = if o.on { el.light() } else { mix(el.main(), BLACK, 0.5) };
                    scr.fill(x - 3, y - 12, 6, 14, c);
                    scr.fill(x - 5, y - 6, 10, 4, c);
                    scr.fill(x - 4, y + 3, 8, 3, rgb(0x5c5c64));
                }
                if o.tag > 0 && o.tag < 10 {
                    scr.text(&o.tag.to_string(), x + 1, y + 6, WHITE, Align::Center, 8);
                }
            }
            OK::CrystalSwitch => {
                let blue = self.crystals_blue();
                let art = if blue { "obj_crystal_switch_blue" } else { "obj_crystal_switch_orange" };
                if !self.obj_hd(scr, art, fx, base) {
                    scr.fill(x - 4, y + 2, 8, 5, rgb(0x5c5c64));
                    scr.disc(x, y - 3, 5, if blue { rgb(0x3c7cfc) } else { rgb(0xfc9838) });
                    scr.pset(x - 2, y - 5, WHITE);
                }
            }
            OK::FloorSwitch => {
                let art = if o.on { "obj_floor_switch_down" } else { "obj_floor_switch_up" };
                if !self.obj_hd(scr, art, fx, fy + 8.0) {
                    scr.fill(x - 6, y - 6, 12, 12, rgb(0x3c3c44));
                    scr.fill(x - 4, y - 4, 8, 8, if o.on { rgb(0x5c5c64) } else { rgb(0x9c9ca8) });
                }
            }
            OK::Tablet => {
                if !self.obj_hd(scr, "obj_lore_tablet", fx, base) {
                    scr.fill(x - 6, y - 9, 12, 16, rgb(0x6c6c78));
                    scr.frame_rect(x - 6, y - 9, 12, 16, rgb(0x34343c));
                    for k in 0..4 {
                        scr.fill(x - 4, y - 6 + k * 3, 8, 1, rgb(0x3c3c48));
                    }
                }
            }
            OK::Prop => {
                let name = self.dungeon.as_ref().and_then(|d| d.keep.as_ref()).and_then(|k| k.texts.get(o.tag)).cloned().unwrap_or_default();
                if !self.obj_hd(scr, &name, fx, base) && o.hard {
                    scr.fill(x - 5, y - 10, 10, 17, rgb(0x5c5c64));
                    scr.fill(x - 5, y - 10, 10, 2, rgb(0x8c8c98));
                }
            }
            _ => {}
        }
    }

    /// Per-frame tile effects in the current room: raised barriers, lava shimmer, hidden
    /// bridges under the lantern's light.
    pub(super) fn draw_keep_tiles(&self, scr: &mut Screen) {
        let room = self.cur_room();
        let blue = self.crystals_blue();
        let lantern = self.has_relic(Relic::Lantern);
        let f = self.frame as i32;
        for (r, row) in room.tiles.iter().enumerate() {
            for (c, &t) in row.iter().enumerate() {
                let (px, py) = (c as i32 * TS, HUD + r as i32 * TS);
                let (cx, cy) = (px as f32 + 8.0, py as f32 + 8.0);
                match t {
                    T_ORANGE | T_BLUE => {
                        let up = (t == T_ORANGE) != blue;
                        let col = if t == T_ORANGE { "orange" } else { "blue" };
                        let art = format!("obj_block_{}_{}", col, if up { "up" } else { "down" });
                        if !self.obj_hd(scr, &art, cx, cy + 8.0) {
                            let (hi, lo) = if t == T_ORANGE { (rgb(0xfcbc3c), rgb(0xa86000)) } else { (rgb(0x78a0fc), rgb(0x2c3cbc)) };
                            if up {
                                scr.fill(px + 1, py - 2, 14, 16, lo);
                                scr.fill(px + 1, py - 2, 14, 4, hi);
                            } else {
                                scr.frame_rect(px + 3, py + 3, 10, 10, lo);
                            }
                        }
                    }
                    T_LAVA => {
                        if !self.obj_hd(scr, "tile_lava", cx, cy + 8.0) {
                            let o = (f / 8 + c as i32 * 3 + r as i32 * 5) % 14;
                            scr.fill(px + o, py + 5, 3, 1, rgb(0xfce040));
                            scr.blend(px, py, TS, TS, rgb(0xfc6000), 0.12 + ((f + c as i32 * 9) as f32 * 0.07).sin().abs() * 0.1);
                        }
                    }
                    T_PIT => {
                        self.obj_hd(scr, "tile_pit", cx, cy + 8.0);
                    }
                    T_HIDDEN => {
                        if lantern {
                            scr.blend(px + 1, py + 1, 14, 14, rgb(0xa4e4fc), 0.35 + (f as f32 * 0.08).sin().abs() * 0.15);
                            scr.frame_rect(px + 1, py + 1, 14, 14, rgb(0xd4f4fc));
                        } else {
                            self.obj_hd(scr, "tile_pit", cx, cy + 8.0);
                        }
                    }
                    T_THORNS => {
                        if !self.obj_hd(scr, "tile_thorns", cx, cy + 8.0) {
                            for k in 0..4 {
                                let (ax, ay) = (px + (k * 5) % 14, py + (k * 7) % 12);
                                scr.fill(ax, ay + 2, 8, 2, rgb(0x2c5c18));
                                scr.fill(ax + 2, ay, 2, 6, rgb(0x2c5c18));
                            }
                        }
                    }
                    T_ROCK => {
                        if !self.obj_hd(scr, "obj_boulder", cx, cy + 8.0) {
                            scr.disc(px + 8, py + 8, 7, rgb(0x6c6c78));
                            scr.disc(px + 6, py + 6, 3, rgb(0x9c9ca8));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// The whip's lash, the cloak's float shadow and a relic held overhead.
    pub(super) fn draw_relic_fx(&self, scr: &mut Screen) {
        let (x, y) = (self.pl.x, self.pl.y);
        if let Some((t, (ux, uy), reach)) = self.whip {
            let k = (1.0 - (t as f32 / 14.0 - 0.5).abs() * 2.0).max(0.15);
            let len = reach * k;
            let (x0, y0) = (x + ux * 5.0, y - 2.0 + uy * 5.0);
            let (x1, y1) = (x0 + ux * len, y0 + uy * len);
            scr.line(x0 as i32, y0 as i32, x1 as i32, y1 as i32, rgb(0x2c7c1c));
            scr.line(x0 as i32, y0 as i32 + 1, x1 as i32, y1 as i32 + 1, rgb(0x58d854));
            scr.disc(x1 as i32, y1 as i32, 2, rgb(0x98d858));
        }
        if self.hover > 0 {
            scr.blend_ellipse(x as i32, y as i32 + 9, 6, 2, BLACK, 0.4);
            let a = if self.hover < 20 && self.frame % 4 < 2 { 0.5 } else { 0.25 };
            scr.blend_disc(x as i32, y as i32 - 4, 9, rgb(0xd4f4fc), a);
        }
        if let Some((r, _)) = self.held_up {
            let (ix, iy) = (x, y - 26.0);
            scr.blend_disc(ix as i32, iy as i32, 10, rgb(0xfce040), 0.3);
            if !self.relic_icon(scr, r.key(), ix, iy, true) && !self.relic_icon(scr, r.key(), ix, iy, false) {
                scr.disc(ix as i32, iy as i32, 5, rgb(0xfce040));
            }
        }
    }

    /// How dark the current keep room is (None = not a dark room).
    pub(super) fn keep_darkness(&self) -> Option<f32> {
        let d = self.dungeon.as_ref()?;
        let k = d.keep.as_ref()?;
        k.def.rooms[d.cur].dark.then_some(if self.has_relic(Relic::Lantern) { 0.62 } else { 0.9 })
    }

    /// The lair map on the pause screen: visited rooms (all rooms with the map), the
    /// current room, chests and the boss stairs with the finder, keys and items found.
    pub(super) fn draw_keep_map(&self, scr: &mut Screen) {
        let Some(d) = &self.dungeon else { return };
        let Some(k) = &d.keep else { return };
        scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, 0.88);
        scr.text(&format!("- {} -", self.dname(d.n)), 128, HUD + 4, rgb(0xfcbc3c), Align::Center, 8);
        let has_map = self.ktok("map");
        let finder = self.ktok("finder");
        let (minx, miny) = (k.def.rooms.iter().map(|r| r.x).min().unwrap_or(0), k.def.rooms.iter().map(|r| r.y).min().unwrap_or(0));
        let (maxx, maxy) = (k.def.rooms.iter().map(|r| r.x + r.w).max().unwrap_or(1), k.def.rooms.iter().map(|r| r.y + r.h).max().unwrap_or(1));
        let (gw, gh) = ((maxx - minx) as i32, (maxy - miny) as i32);
        let cw = (232 / gw.max(1)).min(40);
        let ch = (150 / gh.max(1)).min(26);
        let (ox, oy) = (128 - cw * gw / 2, HUD + 18);
        let col = self.themes[d.theme].map_col;
        for (i, kr) in k.def.rooms.iter().enumerate() {
            let (x, y) = (ox + (kr.x - minx) as i32 * cw, oy + (kr.y - miny) as i32 * ch);
            let (w, h) = (kr.w as i32 * cw, kr.h as i32 * ch);
            let visited = d.rooms[i].visited;
            if !visited && !has_map {
                continue;
            }
            if visited {
                scr.fill(x + 2, y + 2, w - 4, h - 4, col);
            } else {
                scr.frame_rect(x + 2, y + 2, w - 4, h - 4, mix(col, BLACK, 0.4));
            }
            if finder && self.room_has_treasure(i) {
                scr.fill(x + w / 2 - 2, y + h / 2 - 2, 4, 4, rgb(0xfcbc3c));
            }
            if finder && kr.boss_stairs {
                scr.text("B", x + w / 2 + 1, y + h / 2 - 3, rgb(0xd82800), Align::Center, 8);
            }
            if i == d.cur && (self.frame >> 4) & 1 == 1 {
                scr.frame_rect(x + 1, y + 1, w - 2, h - 2, WHITE);
            }
            // Doorways between rooms.
            for door in &kr.doors {
                let (sx, sy) = match door.side {
                    0 => (x + door.seg as i32 * cw + cw / 2 - 2, y),
                    1 => (x + door.seg as i32 * cw + cw / 2 - 2, y + h - 3),
                    2 => (x + w - 3, y + door.seg as i32 * ch + ch / 2 - 2),
                    _ => (x, y + door.seg as i32 * ch + ch / 2 - 2),
                };
                if visited || has_map {
                    scr.fill(sx, sy, 4, 4, if door.to.is_none() { WHITE } else { rgb(0x202020) });
                }
            }
        }
        let room_name = &k.def.rooms[d.cur].name;
        let yb = SH - 46 - HUD_PX + HUD;
        if !room_name.is_empty() {
            scr.text(room_name, 128, yb, WHITE, Align::Center, 8);
        }
        let mut items = format!("KEYS {}", self.kkeys());
        if has_map {
            items += "  MAP";
        }
        if finder {
            items += "  FINDER";
        }
        if self.ktok("bigkey") {
            items += "  BIG KEY";
        }
        scr.text(&items, 128, yb + 12, rgb(0xfcbc3c), Align::Center, 8);
        let relics: Vec<&str> = crate::keepdef::RELICS.iter().filter(|r| self.has_relic(**r)).map(|r| r.name()).collect();
        if !relics.is_empty() {
            scr.text(&relics.join(" "), 128, yb + 24, rgb(0x98d858), Align::Center, 8);
        }
        let _ = Shutter::None;
    }

    /// Per-lair atmosphere in hand-designed lairs: spores (shrine), dust and drips (crypt),
    /// falling grit (castle), embers (fortress), sparkles (sanctuary), wisps (tower).
    /// Deterministic (frame counter), so drawing never touches gameplay randomness.
    pub(super) fn draw_keep_ambience(&self, scr: &mut Screen) {
        let Some(d) = &self.dungeon else { return };
        if d.keep.is_none() {
            return;
        }
        let f = self.frame as f32;
        let hash = |i: u32| -> u32 {
            let mut h = i.wrapping_mul(0x9e37_79b9) ^ 0x7f4a_7c15;
            h ^= h >> 15;
            h = h.wrapping_mul(0x2c1b_3c6d);
            h ^ (h >> 12)
        };
        let (pw, ph) = (SW, SH - HUD_PX);
        let wrap = |v: i32, m: i32| ((v % m) + m) % m;
        match d.theme {
            4 => {
                for i in 0..12u32 {
                    let h = hash(i);
                    let x = wrap((h >> 8) as i32 % pw + ((f * 0.03 + i as f32).sin() * 10.0) as i32, pw);
                    let y = wrap((h >> 16) as i32 % ph - (f * (0.15 + (h % 50) as f32 / 400.0)) as i32, ph) + HUD_PX;
                    let a = 0.35 + 0.3 * (f * 0.07 + i as f32).sin().abs();
                    scr.blend_screen(x, y, 2, 2, rgb(0xb8f818), a);
                }
            }
            5 => {
                for i in 0..10u32 {
                    let h = hash(i + 20);
                    let x = wrap((h >> 8) as i32 % pw + ((f * 0.01 + i as f32).sin() * 16.0) as i32, pw);
                    let y = wrap((h >> 16) as i32 % ph + ((f * 0.013 + i as f32).cos() * 10.0) as i32, ph) + HUD_PX;
                    scr.blend_screen(x, y, 1, 1, rgb(0xc8c8d8), 0.5);
                }
                // A drip falls every so often.
                for i in 0..3u32 {
                    let h = hash(i + 60);
                    let period = 150 + (h % 90) as i32;
                    let t = (self.frame as i32 + (h >> 8) as i32) % period;
                    if t < 40 {
                        let x = (h >> 10) as i32 % pw;
                        let y = HUD_PX + 8 + t * 4;
                        scr.fill_screen(x, y, 1, 3, rgb(0x78a8dc));
                    }
                }
            }
            6 => {
                for i in 0..10u32 {
                    let h = hash(i + 100);
                    let x = (h >> 8) as i32 % pw + ((f * 0.02 + i as f32).sin() * 6.0) as i32;
                    let y = wrap((h >> 16) as i32 % ph + (f * (0.3 + (h % 40) as f32 / 100.0)) as i32, ph) + HUD_PX;
                    scr.blend_screen(x, y, 1, 2, rgb(0xd8c8a0), 0.55);
                }
            }
            7 => {
                for i in 0..16u32 {
                    let h = hash(i + 140);
                    let x = wrap((h >> 8) as i32 % pw + ((f * 0.04 + i as f32).sin() * 8.0) as i32, pw);
                    let y = wrap((h >> 16) as i32 % ph - (f * (0.4 + (h % 60) as f32 / 120.0)) as i32, ph) + HUD_PX;
                    let c = if i % 3 == 0 { rgb(0xfce040) } else { rgb(0xfc7800) };
                    let a = 0.4 + 0.4 * (f * 0.11 + i as f32).sin().abs();
                    scr.blend_screen(x, y, 2, 2, c, a);
                }
            }
            8 => {
                for i in 0..14u32 {
                    let h = hash(i + 200);
                    let x = (h >> 8) as i32 % pw;
                    let y = (h >> 16) as i32 % ph + HUD_PX;
                    let tw = ((f * 0.06 + i as f32 * 1.3).sin() * 0.5 + 0.5).powi(3);
                    if tw > 0.2 {
                        scr.blend_screen(x, y, 1, 1, WHITE, tw);
                        if tw > 0.7 {
                            scr.blend_screen(x - 2, y, 5, 1, rgb(0xd4f4fc), tw * 0.5);
                            scr.blend_screen(x, y - 2, 1, 5, rgb(0xd4f4fc), tw * 0.5);
                        }
                    }
                }
            }
            9 => {
                for i in 0..6u32 {
                    let h = hash(i + 260);
                    let x = wrap((h >> 6) as i32 % (pw + 80) + (f * 0.2) as i32, pw + 80) - 40;
                    let y = HUD_PX + 16 + (h >> 14) as i32 % (ph - 32) + ((f * 0.02 + i as f32).sin() * 8.0) as i32;
                    let a = 0.06 + 0.05 * (f * 0.03 + i as f32).sin().abs();
                    scr.blend_screen(x - 20, y - 3, 40, 6, rgb(0xb040fc), a);
                    scr.blend_screen(x - 10, y - 5, 20, 10, rgb(0xd878fc), a * 0.8);
                }
            }
            _ => {}
        }
    }
}
