//! Full-width (320 px) menus in the style of A Link to the Past: the item screen on
//! pause, the painted world map, and the title and element-choice screens. Generated art
//! (title_*, map_icons, ui_*) is used when present; everything has a code-drawn fallback.
//! All of these draw in UI mode, where logic x = screen x - 32 (the 256-wide UI is centred).
use super::bag::Slot;
use super::draw::{draw_bomb_icon, draw_elixir_icon};
use super::*;
use crate::keepdef::{Relic, RELICS};

/// Screen-space left edge in UI-mode logic coordinates.
const LEFT: i32 = -32;

fn crop(s: &Sprite, x: i32, y: i32, w: i32, h: i32) -> Sprite {
    let mut out = Sprite::new(w, h);
    for yy in 0..h {
        for xx in 0..w {
            let (sx, sy) = (x + xx, y + yy);
            if sx >= 0 && sy >= 0 && sx < s.w && sy < s.h {
                out.px[(yy * w + xx) as usize] = s.px[(sy * s.w + sx) as usize];
            }
        }
    }
    out
}

impl Game {
    fn ui_sprite(&self, name: &str, anim: &str) -> Option<&Sprite> {
        Some(self.art.sheet(name)?.anim(anim).or_else(|| self.art.sheet(name)?.anim("idle"))?.at(self.frame as u32))
    }

    /// A SNES menu panel: the generated 9-slice frame when present, else a double border.
    pub(super) fn panel(&self, scr: &mut Screen, x: i32, y: i32, w: i32, h: i32) {
        scr.blend(x, y, w, h, rgb(0x080418), 0.9);
        if let Some(f) = self.ui_sprite("ui_frame", "idle").filter(|f| f.w >= 24 && f.h >= 24) {
            let e = f.w / 3;
            let piece = |px: i32, py: i32| crop(f, px * e, py * e, e, e);
            let (corner, edge_h, edge_v) = (e, e, e);
            let _ = (corner, edge_h, edge_v);
            let mut xx = x + e;
            while xx < x + w - e {
                let top = piece(1, 0);
                let bot = piece(1, 2);
                scr.blit_hd(&top, xx as f32, y as f32, false);
                scr.blit_hd(&bot, xx as f32, (y + h - e) as f32, false);
                xx += e;
            }
            let mut yy = y + e;
            while yy < y + h - e {
                scr.blit_hd(&piece(0, 1), x as f32, yy as f32, false);
                scr.blit_hd(&piece(2, 1), (x + w - e) as f32, yy as f32, false);
                yy += e;
            }
            scr.blit_hd(&piece(0, 0), x as f32, y as f32, false);
            scr.blit_hd(&piece(2, 0), (x + w - e) as f32, y as f32, false);
            scr.blit_hd(&piece(0, 2), x as f32, (y + h - e) as f32, false);
            scr.blit_hd(&piece(2, 2), (x + w - e) as f32, (y + h - e) as f32, false);
        } else {
            scr.frame_rect(x, y, w, h, rgb(0xd8d0f8));
            scr.frame_rect(x + 2, y + 2, w - 4, h - 4, rgb(0x5c4880));
        }
    }

    // ------------------------------------------------------------ item screen
    /// Where each bag slot sits on the item screen (3 columns).
    fn item_cell(i: usize) -> (i32, i32) {
        (LEFT + 26 + (i % 3) as i32 * 34, HUD + 34 + (i / 3) as i32 * 34)
    }
    /// The pause screen's item page: bag grid (pick with the d-pad), relics, status.
    pub(super) fn draw_item_screen(&self, scr: &mut Screen) {
        scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, BLACK, 0.75);
        // Bag.
        self.panel(scr, LEFT + 6, HUD + 6, 112, 96);
        scr.text("- ITEMS -", LEFT + 62, HUD + 12, rgb(0xfcbc3c), Align::Center, 8);
        let slots = self.slots();
        let sel = self.s.bag_sel.min(slots.len() - 1);
        for (i, s) in slots.iter().enumerate() {
            let (cx, cy) = Self::item_cell(i);
            match self.ui_sprite("ui_slot", "idle") {
                Some(img) => scr.blit_hd(img, (cx - img.w / 2) as f32, (cy - img.h / 2) as f32, false),
                None => scr.frame_rect(cx - 11, cy - 11, 22, 22, rgb(0x5c4880)),
            }
            self.draw_slot_icon(scr, *s, cx, cy);
            let n = self.slot_count(*s);
            if !matches!(s, Slot::Whip | Slot::Cloak) {
                scr.text(&n.to_string(), cx + 8, cy + 6, if n > 0 { WHITE } else { rgb(0x747474) }, Align::Left, 8);
            }
            if i == sel {
                match self.ui_sprite("ui_cursor", "idle") {
                    Some(img) => scr.blit_hd(img, (cx - img.w / 2) as f32, (cy - img.h / 2) as f32, false),
                    None => {
                        if (self.frame >> 3) & 1 == 1 {
                            scr.frame_rect(cx - 13, cy - 13, 26, 26, rgb(0xfce040));
                        }
                    }
                }
            }
        }
        scr.text(slots[sel].name(), LEFT + 62, HUD + 92, WHITE, Align::Center, 8);
        // Relics.
        self.panel(scr, LEFT + 124, HUD + 6, 190, 96);
        scr.text("- RELICS -", LEFT + 219, HUD + 12, rgb(0xfcbc3c), Align::Center, 8);
        for (i, r) in RELICS.iter().enumerate() {
            let (cx, cy) = (LEFT + 146 + i as i32 * 36, HUD + 42);
            scr.frame_rect(cx - 11, cy - 11, 22, 22, rgb(0x5c4880));
            if self.has_relic(*r) {
                if !self.relic_icon(scr, r.key(), cx as f32, cy as f32, true) && !self.relic_icon(scr, r.key(), cx as f32, cy as f32, false) {
                    scr.disc(cx, cy, 5, rgb(0x98d858));
                }
            }
        }
        let owned: Vec<Relic> = RELICS.iter().copied().filter(|r| self.has_relic(*r)).collect();
        let hint = match owned.last() {
            None => "RELICS LIE IN THE GREAT CHESTS OF THE LAIRS.".to_string(),
            Some(_) => {
                let mut uses = vec![];
                for r in &owned {
                    uses.push(match r {
                        Relic::Whip => "WHIP: Y",
                        Relic::Lantern => "LANTERN: DARK",
                        Relic::Gloves => "GLOVES: PUSH",
                        Relic::Boots => "BOOTS: LAVA",
                        Relic::Cloak => "CLOAK: Y",
                    });
                }
                uses.join("  ")
            }
        };
        let mut line = String::new();
        let mut y = HUD + 64;
        for w in hint.split(' ') {
            if line.len() + w.len() + 1 > 22 && !line.is_empty() {
                scr.text(&line, LEFT + 219, y, rgb(0x98d858), Align::Center, 8);
                y += 10;
                line.clear();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(w);
        }
        if !line.is_empty() {
            scr.text(&line, LEFT + 219, y, rgb(0x98d858), Align::Center, 8);
        }
        // Status.
        self.panel(scr, LEFT + 6, HUD + 106, 308, 96);
        let el = self.el();
        scr.text(&format!("{} MAGIC   SPELL: {}", el.name(), SPELL_NAMES[el.idx()]), LEFT + 160, HUD + 114, el.light(), Align::Center, 8);
        let runes = (1..=5).filter(|&i| self.s.cleared[i]).count();
        scr.text(
            &format!("LIFE {}/{}   MAGIC {}/{}   GOLD {}", self.s.hp.max(0), self.s.max_hp, self.s.mp as i32, self.s.max_mp, self.s.gold),
            LEFT + 160,
            HUD + 128,
            WHITE,
            Align::Center,
            8,
        );
        // Five rune slots, lit for each conquered lair.
        for i in 0..5 {
            let (rx, ry) = (LEFT + 112 + i as i32 * 24, HUD + 148);
            let lit = self.s.cleared[i + 1];
            scr.frame_rect(rx - 8, ry - 8, 16, 16, rgb(0x5c4880));
            if lit {
                scr.blend_disc(rx, ry, 7, [rgb(0x98d858), rgb(0xa4e4fc), rgb(0xbcbcbc), rgb(0xfc9838), rgb(0xf878f8)][i], 0.6);
                scr.text(&(i + 1).to_string(), rx + 1, ry - 3, WHITE, Align::Center, 8);
            }
        }
        scr.text(&format!("RUNES {}/5", runes), LEFT + 64, HUD + 145, rgb(0xf878f8), Align::Center, 8);
        scr.text(&self.quest_line(), LEFT + 160, HUD + 166, rgb(0xd878fc), Align::Center, 8);
        let map_word = if self.dungeon.is_some() && self.in_lair == 0 && !self.in_shop() { "LAIR MAP" } else { "MAP" };
        scr.text(&format!("D-PAD: PICK   A: {}   START: RESUME", map_word), LEFT + 160, HUD + 184, rgb(0x747474), Align::Center, 8);
    }
    fn draw_slot_icon(&self, scr: &mut Screen, s: Slot, cx: i32, cy: i32) {
        let (x, y) = (cx as f32, cy as f32);
        match s {
            Slot::Ration => match self.item_hd(IK::Meat) {
                Some(img) => scr.spr_hd(img, x, y, false),
                None => {
                    scr.disc(cx, cy, 4, rgb(0xa86030));
                    scr.fill(cx - 1, cy - 2, 2, 2, rgb(0xfcbc3c));
                }
            },
            Slot::Potion => match self.item_named("mana_potion") {
                Some(img) => scr.spr_hd(img, x, y, false),
                None => scr.spr(&self.spr.potion.img, x, y, false),
            },
            Slot::Antidote => match self.item_named("antidote") {
                Some(img) => scr.spr_hd(img, x, y, false),
                None => scr.spr(&self.spr.antidote.img, x, y, false),
            },
            Slot::Bomb => draw_bomb_icon(scr, cx, cy),
            Slot::Elixir => draw_elixir_icon(scr, cx, cy),
            Slot::Whip | Slot::Cloak => {
                let key = if s == Slot::Whip { "vine_whip" } else { "feather_cloak" };
                if !self.relic_icon(scr, key, x, y, false) {
                    scr.disc(cx, cy, 5, if s == Slot::Whip { rgb(0x58d854) } else { rgb(0xd4f4fc) });
                }
            }
        }
    }
    /// Move the item-screen cursor (3 columns).
    pub(super) fn item_cursor(&mut self, dx: i32, dy: i32) {
        let n = self.slots().len() as i32;
        let cur = self.s.bag_sel.min(n as usize - 1) as i32;
        let mut next = cur + dx + dy * 3;
        if dx != 0 {
            next = next.rem_euclid(n);
        } else if !(0..n).contains(&next) {
            next = cur;
        }
        if next != cur {
            self.s.bag_sel = next as usize;
            self.sfx(Sfx::Select);
        }
    }

    // ------------------------------------------------------------ painted world map
    /// The overworld as a picture: every area's tiles in its terrain colours (2 px per tile),
    /// fog over places not yet visited, icons for landmarks and the mage.
    pub(super) fn draw_world_map(&self, scr: &mut Screen) {
        scr.fill_screen(0, HUD_PX, SW, SH - HUD_PX, rgb(0x0c0a18));
        let px = 2; // map pixels per tile
        let (ox, oy) = (LEFT + 8, HUD);
        let mut img = Sprite::new(256, 208);
        let fog = rgb(0x1c1830);
        for p in img.px.iter_mut() {
            *p = fog;
        }
        for r in &self.rooms {
            let th = &self.themes[r.theme];
            let (floor, wall, water) = self.theme_map_colours(r.theme);
            let (bx, by) = (r.x as i32 * 32, r.y as i32 * 26);
            for (ty, row) in r.tiles.iter().enumerate() {
                for (tx, &t) in row.iter().enumerate() {
                    let c = match t {
                        T_WALL => wall,
                        T_WATER | T_ICE => water,
                        T_DECOR => mix(wall, rgb(0x303030), 0.4),
                        T_LAVA => rgb(0xc83000),
                        T_PIT | T_HIDDEN => rgb(0x050508),
                        T_THORNS => rgb(0x2c5c18),
                        T_ROCK => rgb(0x6c6c78),
                        _ => floor,
                    };
                    let c = if r.visited { c } else { mix(c, fog, 0.8) };
                    for dy in 0..px {
                        for dx in 0..px {
                            let (x, y) = (bx + tx as i32 * px + dx, by + ty as i32 * px + dy);
                            if x < 256 && y < 208 {
                                img.px[(y * 256 + x) as usize] = c;
                            }
                        }
                    }
                }
            }
            let _ = th;
        }
        scr.blit_hd(&img, ox as f32, oy as f32, false);
        // Area borders (thin, darker) so big areas read as one place.
        for r in self.rooms.iter().filter(|r| r.visited) {
            let (x, y, w, h) = (ox + r.x as i32 * 32, oy + r.y as i32 * 26, r.cw as i32 * 32, r.ch as i32 * 26);
            scr.blend(x, y, w, 1, BLACK, 0.35);
            scr.blend(x, y, 1, h, BLACK, 0.35);
        }
        // Landmarks.
        for r in self.rooms.iter().filter(|r| r.visited) {
            let (cx, cy) = (ox + r.x as i32 * 32 + r.cw as i32 * 16, oy + r.y as i32 * 26 + r.ch as i32 * 13);
            if r.gate > 0 {
                let cleared = self.s.cleared[r.gate];
                self.map_icon(scr, if cleared { "lair_cleared" } else { "lair" }, cx, cy, |scr| {
                    scr.fill(cx - 4, cy - 5, 8, 10, if cleared { rgb(0x98d858) } else { rgb(0xd8a040) });
                    scr.text(&if r.gate == 6 { "X".to_string() } else { r.gate.to_string() }, cx + 1, cy - 3, BLACK, Align::Center, 8);
                });
            }
            if r.cave > 0 {
                let (dx, dy) = self.door_pos(r.i);
                let (ix, iy) = (ox + r.x as i32 * 32 + (dx as i32) / 8, oy + r.y as i32 * 26 + (dy as i32 - HUD) / 8);
                let cleared = self.cave_cleared(r.cave);
                self.map_icon(scr, if cleared { "cave_cleared" } else { "cave" }, ix, iy, |scr| {
                    scr.disc(ix, iy, 4, if cleared { rgb(0x747474) } else { rgb(0xd8b878) });
                    scr.fill(ix - 1, iy - 1, 3, 4, BLACK);
                });
            }
            if r.special == SP_MONOLITH {
                self.map_icon(scr, "monolith", cx, cy, |scr| {
                    scr.fill(cx - 2, cy - 6, 4, 12, rgb(0xa4e4fc));
                });
            }
            if r.special == SP_SHOP {
                self.map_icon(scr, "village", cx, cy, |scr| {
                    scr.fill(cx - 4, cy - 2, 8, 6, rgb(0xd8b878));
                    scr.fill(cx - 5, cy - 5, 10, 3, rgb(0xa81000));
                });
            }
            if r.tank && !self.s.tanks.contains(&r.i) {
                self.map_icon(scr, "heart", cx + 8, cy - 6, |scr| scr.fill(cx + 6, cy - 8, 4, 4, rgb(0xfc7460)));
            }
            if r.chest.is_some() && !self.s.opened.contains(&r.i) {
                self.map_icon(scr, "chest", cx - 8, cy + 6, |scr| scr.fill(cx - 10, cy + 4, 4, 4, rgb(0xa85020)));
            }
            if self.mini_marker(r.mini).is_some() && !self.done(r.mini) {
                self.map_icon(scr, "encounter", cx + 8, cy + 6, |scr| scr.fill(cx + 6, cy + 4, 4, 4, rgb(0xfc3c3c)));
            }
        }
        // The mage (in the current area, or the lair/cave entrance while inside one).
        let ri = if self.dungeon.is_some() { self.gate_room } else { self.room };
        if let Some(r) = self.rooms.get(ri) {
            let (lx, ly) = if self.dungeon.is_some() { self.door_pos(ri) } else { (self.pl.x, self.pl.y) };
            let (mx, my) = (ox + r.x as i32 * 32 + (lx as i32) / 8, oy + r.y as i32 * 26 + (ly as i32 - HUD) / 8);
            if (self.frame >> 3) & 1 == 1 {
                self.map_icon(scr, "player", mx, my, |scr| {
                    scr.fill(mx - 2, my - 2, 5, 5, WHITE);
                    scr.fill(mx - 1, my - 1, 3, 3, self.el().main());
                });
            }
        }
        // Region name, legend and relics along the right edge.
        let name = self.themes[self.rooms[ri].theme].name;
        scr.fill_screen(0, HUD_PX, SW, 11, rgb(0x0c0a18));
        scr.text(&format!("- {} -", name), 128, HUD + 2, rgb(0xfcbc3c), Align::Center, 8);
        for (i, r) in RELICS.iter().enumerate() {
            let (rx, ry) = (LEFT + 300, HUD + 30 + i as i32 * 24);
            scr.frame_rect(rx - 9, ry - 9, 18, 18, rgb(0x5c4880));
            if self.has_relic(*r) && !self.relic_icon(scr, r.key(), rx as f32, ry as f32, false) {
                scr.disc(rx, ry, 4, rgb(0x98d858));
            }
        }
        scr.text("A:ITEMS", LEFT + 256, HUD + 2, rgb(0x747474), Align::Left, 8);
        scr.blend_screen(0, SH - 12, SW, 12, BLACK, 0.7);
        scr.text(&self.quest_line(), 128, HUD + 198, rgb(0xd878fc), Align::Center, 8);
    }
    fn map_icon(&self, scr: &mut Screen, name: &str, x: i32, y: i32, fallback: impl FnOnce(&mut Screen)) {
        match self.art.sheet("map_icons").and_then(|sh| sh.anim(name)).map(|a| a.at(0)) {
            Some(img) => scr.spr_hd(img, x as f32, y as f32, false),
            None => fallback(scr),
        }
    }
    /// Floor, wall and water colours for an area's map pixels, from its tileset when present.
    fn theme_map_colours(&self, theme: usize) -> (u32, u32, u32) {
        let th = &self.themes[theme.min(self.themes.len() - 1)];
        let avg = |s: &Sprite| -> u32 {
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            for &p in s.px.iter().filter(|&&p| p != 0) {
                r += (p >> 16) & 255;
                g += (p >> 8) & 255;
                b += p & 255;
                n += 1;
            }
            let n = n.max(1);
            rgb(((r / n) << 16) | ((g / n) << 8) | (b / n))
        };
        match &th.hd_wall {
            Some(w) if w.len() == 16 => {
                let water = th.hd_water.as_ref().filter(|w| w.len() == 16).map_or(rgb(0x2c5cac), |w| avg(&w[15]));
                (avg(&w[0]), mix(avg(&w[15]), BLACK, 0.25), water)
            }
            _ => (avg(&th.floor), avg(&th.walls[0]), rgb(0x2c5cac)),
        }
    }

    // ------------------------------------------------------------ title and element choice
    /// The painted title: parallax night scene panning slowly, the logo, the menu panel.
    /// The painted, slowly panning title scene (parallax layers, or the single scene image).
    fn draw_title_backdrop(&self, scr: &mut Screen) {
        let t = self.frame as f32;
        let layers = [("title_sky", 0.15), ("title_far", 0.3), ("title_near", 0.55)];
        let mut painted = false;
        for (name, speed) in layers {
            if let Some(img) = self.ui_sprite(name, "idle") {
                let span = (img.w - SW).max(0) as f32;
                let off = if span > 0.0 { ((t * speed * 0.25) % (span * 2.0) - span).abs() } else { 0.0 };
                scr.blit_hd(img, LEFT as f32 - off, (HUD_PX - 32 + 16) as f32, false);
                painted = true;
            }
        }
        if !painted {
            if let Some(img) = self.ui_sprite("title_scene", "idle") {
                let span = (img.w - SW).max(0) as f32;
                let off = if span > 0.0 { ((t * 0.05) % (span * 2.0) - span).abs() } else { 0.0 };
                scr.blit_hd(img, LEFT as f32 - off, 16.0, false);
            } else {
                self.draw_stars(scr);
            }
        }
    }
    pub(super) fn draw_title_screen(&self, scr: &mut Screen) {
        self.draw_title_backdrop(scr);
        match self.ui_sprite("title_logo", "idle") {
            Some(img) => scr.blit_hd(img, (128 - img.w / 2) as f32, 18.0, false),
            None => {
                scr.text("ELEMENTAL", 128, 30, rgb(0x3cbcfc), Align::Center, 16);
                scr.text("LEGENDS", 130, 52, rgb(0x881400), Align::Center, 32);
                scr.text("LEGENDS", 128, 50, rgb(0xfcbc3c), Align::Center, 32);
            }
        }
        let opts = self.menu_opts();
        let h = 12 + opts.len() as i32 * 12;
        self.panel(scr, 76, 132, 104, h);
        for (i, o) in opts.iter().enumerate() {
            let sel = i == self.menu;
            let prefix = if sel && (self.frame >> 4) & 1 == 1 { "> " } else { "  " };
            scr.text(&format!("{}{}", prefix, o), 92, 138 + i as i32 * 12, if sel { WHITE } else { rgb(0xa0a0b0) }, Align::Left, 8);
        }
        scr.blend_screen(0, 196, SW, 44, BLACK, 0.55);
        scr.text("MOVE: D-PAD  CAST: A  SPELL: B  ITEM: Y", 128, 200, rgb(0xbcbcbc), Align::Center, 8);
        scr.text("BAG: SELECT  BLINK: R1  PAUSE: START", 128, 212, rgb(0xbcbcbc), Align::Center, 8);
        let c = if (self.frame >> 5) & 1 == 1 { rgb(0xfcbc3c) } else { rgb(0xfc7460) };
        scr.text("PRESS START", 128, 226, c, Align::Center, 8);
    }
    /// Element choice across the full width: four mages on pedestals, details below.
    pub(super) fn draw_choose_screen(&self, scr: &mut Screen) {
        // The same moonlit scene as the title, only lightly dimmed so it stays readable.
        self.draw_title_backdrop(scr);
        scr.blend_screen(0, 0, SW, SH, BLACK, 0.3);
        scr.text("CHOOSE YOUR ELEMENT", 128, 14, WHITE, Align::Center, 8);
        for i in 0..4 {
            let cx = LEFT + 40 + i as i32 * 80;
            let sel = i == self.menu;
            let bob = if sel { ((self.frame as f32 * 0.15).sin() * 3.0) as i32 } else { 0 };
            self.panel(scr, cx - 34, 30, 68, 86);
            if sel {
                scr.blend_disc(cx, 80, 26, EL_LIGHT[i], 0.18 + (self.frame as f32 * 0.08).sin().abs() * 0.1);
                scr.frame_rect(cx - 32, 32, 64, 82, EL_LIGHT[i]);
            }
            match self.mage_hd(i, sel) {
                Some(img) => scr.blit_hd(img, (cx - img.w / 2) as f32, (52 + bob) as f32, false),
                None => scr.blit_scaled(&self.spr.mage_d[i][if sel { ((self.frame / 8) % 4) as usize } else { 0 }].img, cx - 18, 56 + bob, 2),
            }
            scr.text(Elem::from_idx(i).name(), cx, 100, if sel { EL_LIGHT[i] } else { rgb(0x747474) }, Align::Center, 8);
        }
        let desc: [[&str; 3]; 4] = [
            ["FIREBOLTS SET FOES ABLAZE", "SPELL: FLAME RING", "STRONG VS ICE FOES"],
            ["SHARDS CHILL, THEN FREEZE", "SPELL: FROST NOVA", "STRONG VS FIRE FOES"],
            ["FAST PIERCING BOLTS", "SPELL: CHAIN BOLT", "STRONG VS EARTH FOES"],
            ["HEAVY ROCKS SMASH ICE", "SPELL: QUAKE", "STRONG VS STORM FOES"],
        ];
        self.panel(scr, LEFT + 30, 124, 260, 62);
        for (j, l) in desc[self.menu].iter().enumerate() {
            scr.text(l, 128, 132 + j as i32 * 14, if j == 2 { rgb(0xfce040) } else { WHITE }, Align::Center, 8);
        }
        scr.text("ORB SHRINES CHANGE YOUR ELEMENT LATER", 128, 196, rgb(0xa0a0b0), Align::Center, 8);
        scr.text("< >  CHOOSE    START  OK", 128, 220, rgb(0xfcbc3c), Align::Center, 8);
    }
}
