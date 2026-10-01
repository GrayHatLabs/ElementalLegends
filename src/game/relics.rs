//! The five relics in play: the Vine Whip (lash: swing to posts over gaps, cut thorns,
//! flip switches, sting monsters), the Feather Cloak (float over pits for a moment),
//! the Spirit Lantern, Titan Gloves and Ember Boots (passive; see keep.rs and dyn_solid).
//! Also falling into pits.
use super::dungeon::OK;
use super::*;
use crate::keepdef::Relic;

const WHIP_REACH: f32 = 80.0;
const WHIP_FRAMES: i32 = 14;
const HOVER_FRAMES: i32 = 80;

impl Game {
    /// Lash the Vine Whip in the facing direction.
    pub(super) fn use_whip(&mut self) {
        if self.whip.is_some() || self.pull.is_some() {
            return;
        }
        let (fx, fy) = (self.pl.fx, self.pl.fy);
        let l = fx.hypot(fy).max(0.01);
        let (ux, uy) = (fx / l, fy / l);
        let (x0, y0) = (self.pl.x, self.pl.y - 2.0);
        let mut reach = WHIP_REACH;
        let mut d = 6.0;
        while d <= WHIP_REACH {
            let (x, y) = (x0 + ux * d, y0 + uy * d);
            let (c, r) = tile_of(x, y);
            // Thorns are cut wherever they grow.
            if self.tile_at(c, r) == T_THORNS {
                self.cut_thorns(c, r);
                reach = d;
                break;
            }
            // Dungeon objects: posts pull the mage over, switches flip, levers pull.
            if let Some((i, k)) = self.dungeon.as_ref().and_then(|dg| {
                dg.objs[dg.cur].iter().enumerate().find(|(_, o)| o.visible && o.c == c && o.r == r).map(|(i, o)| (i, o.k))
            }) {
                match k {
                    OK::Post => {
                        // Land on the near side of the post.
                        let (px, py) = tile_center(c, r);
                        self.pull = Some((px - ux * 16.0, py - uy * 16.0 + 2.0));
                        self.sfx(Sfx::Zap);
                    }
                    OK::CrystalSwitch => self.toggle_crystals(),
                    OK::Lever => {
                        if let Some(dg) = self.dungeon.as_ref() {
                            if !dg.objs[dg.cur][i].on {
                                self.whip_lever(i);
                            }
                        }
                    }
                    _ => {}
                }
                reach = d;
                break;
            }
            // Monsters are stung and stunned.
            let hit = self.enemies.iter().position(|e| !e.dead && e.active() && (e.x - x).abs() < e.w / 2.0 + 3.0 && (e.y - y).abs() < e.h / 2.0 + 3.0);
            if let Some(i) = hit {
                let mut en = std::mem::take(&mut self.enemies);
                en[i].st.stun(40);
                en[i].kbx = ux * 2.0;
                en[i].kby = uy * 2.0;
                self.damage_enemy(&mut en[i], 1.5, Elem::Neutral);
                en.append(&mut self.enemies);
                self.enemies = en;
                reach = d;
                break;
            }
            if self.shot_blocked(x, y) {
                reach = d;
                break;
            }
            d += 4.0;
        }
        self.whip = Some((WHIP_FRAMES, (ux, uy), reach));
        self.sfx(Sfx::Shoot);
    }
    fn whip_lever(&mut self, i: usize) {
        // Same as walking into it.
        let tag = self.dungeon.as_ref().map_or(usize::MAX, |d| d.objs[d.cur][i].tag);
        if let Some(d) = self.dungeon.as_mut() {
            let cur = d.cur;
            d.objs[cur][i].on = true;
        }
        let flag = self.dungeon.as_ref().and_then(|d| d.keep.as_ref().and_then(|k| k.flags.get(tag).cloned()));
        if let Some(f) = flag {
            self.set_flag(&f);
        }
        self.sfx(Sfx::Click);
    }
    fn cut_thorns(&mut self, c: i32, r: i32) {
        // Cut the whole clump (connected thorn tiles).
        let mut stack = vec![(c, r)];
        let mut cut = vec![];
        while let Some((x, y)) = stack.pop() {
            if self.tile_at(x, y) != T_THORNS || cut.contains(&(x, y)) {
                continue;
            }
            cut.push((x, y));
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
        for &(x, y) in &cut {
            self.set_any_tile(x, y, T_FLOOR);
            let (px, py) = tile_center(x, y);
            for _ in 0..4 {
                let (vx, vy) = (self.rng.range(-1.4, 1.4), self.rng.range(-2.0, -0.4));
                self.part(px, py, vx, vy, 22, rgb(0x2c7c1c), 2, PK::Shard);
            }
        }
        self.rerender_current();
        self.sfx(Sfx::Hit);
    }
    /// Change a tile in the current room (overworld area or dungeon room).
    pub(super) fn set_any_tile(&mut self, c: i32, r: i32, t: u8) {
        if let Some(d) = self.dungeon.as_mut() {
            let cur = d.cur;
            d.rooms[cur].tiles[r as usize][c as usize] = t;
            d.dirty = true;
        } else {
            let ri = self.room;
            self.rooms[ri].tiles[r as usize][c as usize] = t;
        }
    }
    pub(super) fn rerender_current(&mut self) {
        if self.dungeon.is_some() {
            self.dungeon_flush();
        } else {
            let ri = self.room;
            let th = self.rooms[ri].theme;
            render(&mut self.rooms[ri], &self.themes[th]);
        }
    }

    /// Float with the Feather Cloak.
    pub(super) fn use_cloak(&mut self) {
        if self.hover > 0 {
            return;
        }
        if self.s.mp < 4.0 {
            let (x, y) = (self.pl.x, self.pl.y);
            self.float("NO MANA", x - 28.0, y - 18.0, rgb(0x747474));
            self.sfx(Sfx::Deny);
            return;
        }
        self.s.mp -= 4.0;
        self.hover = HOVER_FRAMES;
        self.sfx(Sfx::Heal);
    }

    /// Per-frame relic effects: whip and pull animations, floating, pits, held-up relic,
    /// Titan Gloves on overworld boulders.
    pub(super) fn update_relics(&mut self) {
        if let Some((t, dir, reach)) = self.whip {
            self.whip = if t > 1 { Some((t - 1, dir, reach)) } else { None };
        }
        if let Some((tx, ty)) = self.pull {
            let (dx, dy) = (tx - self.pl.x, ty - self.pl.y);
            let l = dx.hypot(dy);
            if l < 4.5 {
                self.pl.x = tx;
                self.pl.y = ty;
                self.pull = None;
            } else {
                self.pl.x += dx / l * 4.0;
                self.pl.y += dy / l * 4.0;
                self.pl.inv = self.pl.inv.max(4);
            }
        }
        if self.hover > 0 {
            self.hover -= 1;
        }
        if let Some((r, t)) = self.held_up {
            self.held_up = if t > 1 { Some((r, t - 1)) } else { None };
        }
        // Falling into a pit (unless floating or being pulled across).
        if self.mode == Mode::Play && self.hover == 0 && self.pull.is_none() && self.scroll.is_none() {
            let (c, r) = tile_of(self.pl.x, self.pl.y + 2.0);
            let t = self.tile_at(c, r);
            let pit = t == T_PIT || (t == T_HIDDEN && !self.has_relic(Relic::Lantern));
            if pit {
                self.fall_in_pit();
            }
        }
    }
    fn fall_in_pit(&mut self) {
        let (x, y) = (self.pl.x, self.pl.y);
        for i in 0..10 {
            let a = i as f32 * 0.63;
            self.part(x, y, a.cos() * 0.6, a.sin() * 0.6, 18, rgb(0x5c5c64), 2, PK::Dot);
        }
        self.float("FELL!", x - 20.0, y - 18.0, rgb(0xfc7460));
        if self.dungeon.is_none() && !self.gate_warned {
            self.gate_warned = true;
            self.show_msg(if self.has_relic(Relic::Lantern) { "A DEEP CHASM. YOU WOULD NEED TO FLOAT ACROSS IT." } else { "THE GROUND GIVES WAY! A LIGHT THAT SHOWS HIDDEN THINGS MIGHT REVEAL A SAFE PATH." });
        }
        let (rx, ry) = self.room_entry_pos;
        self.pl.x = rx;
        self.pl.y = ry;
        self.pl.kbx = 0.0;
        self.pl.kby = 0.0;
        self.pl.inv = 0;
        self.hurt(2);
        self.sfx(Sfx::Hurt);
    }
    /// Titan Gloves on overworld boulder tiles: push into one to heave it aside.
    pub(super) fn heave_rocks(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        let axis = if ix != 0.0 && iy == 0.0 && blocked.0 {
            (ix.signum(), 0.0)
        } else if iy != 0.0 && ix == 0.0 && blocked.1 {
            (0.0, iy.signum())
        } else {
            self.rock_t = 0;
            return;
        };
        let probe = (self.pl.x + axis.0 * (self.pl.w / 2.0 + 3.0), self.pl.y + axis.1 * (self.pl.h / 2.0 + 3.0));
        let (c, r) = tile_of(probe.0, probe.1);
        if self.tile_at(c, r) != T_ROCK {
            self.rock_t = 0;
            return;
        }
        if !self.has_relic(Relic::Gloves) {
            if self.rock_t == 0 {
                self.float("TOO HEAVY", self.pl.x - 36.0, self.pl.y - 20.0, WHITE);
            }
            self.rock_t = 1;
            return;
        }
        self.rock_t += 1;
        if self.rock_t < 16 {
            return;
        }
        self.rock_t = 0;
        // Heave the whole boulder (connected rock tiles).
        let mut stack = vec![(c, r)];
        let mut gone = vec![];
        while let Some((x, y)) = stack.pop() {
            if self.tile_at(x, y) != T_ROCK || gone.contains(&(x, y)) {
                continue;
            }
            gone.push((x, y));
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
        for &(x, y) in &gone {
            self.set_any_tile(x, y, T_FLOOR);
            let (px, py) = tile_center(x, y);
            for _ in 0..5 {
                let (vx, vy) = (self.rng.range(-1.8, 1.8), self.rng.range(-2.6, -0.6));
                self.part(px, py, vx, vy, 26, rgb(0x8c8c98), 3, PK::Shard);
            }
        }
        self.rerender_current();
        self.sfx(Sfx::Rumble);
        self.shake = 8;
    }

    /// Bumping into an overworld obstacle without its relic: say what might help.
    pub(super) fn gate_hint(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        if self.gate_warned {
            return;
        }
        let axis = if ix != 0.0 && iy == 0.0 && blocked.0 {
            (ix.signum(), 0.0)
        } else if iy != 0.0 && ix == 0.0 && blocked.1 {
            (0.0, iy.signum())
        } else {
            return;
        };
        let probe = (self.pl.x + axis.0 * (self.pl.w / 2.0 + 3.0), self.pl.y + axis.1 * (self.pl.h / 2.0 + 3.0));
        let (c, r) = tile_of(probe.0, probe.1);
        let (relic, text) = match self.tile_at(c, r) {
            T_THORNS => (Relic::Whip, "THORNY VINES CHOKE THE PATH. SOMETHING SHARP AND SUPPLE COULD CUT THEM."),
            T_ROCK => (Relic::Gloves, "A HUGE BOULDER BLOCKS THE WAY. ONLY GREAT STRENGTH COULD MOVE IT."),
            T_LAVA => (Relic::Boots, "A RIVER OF LAVA BARS THE WAY. BOOTS THAT DEFY FIRE WOULD BE NEEDED."),
            _ => return,
        };
        if self.has_relic(relic) {
            return;
        }
        self.gate_warned = true;
        self.show_msg(text);
        self.sfx(Sfx::Deny);
    }
}
