//! The relic bag: carried items used with the potion button (Y / L1 / R1, keyboard C).
//! Select (keyboard Tab / V) picks the item; in the pause menu Left / Right also pick it.

use super::*;

/// Bag slots, in cycling order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Slot {
    Potion,
    Antidote,
    Bomb,
    Elixir,
}
pub(super) const SLOTS: [Slot; 4] = [Slot::Potion, Slot::Antidote, Slot::Bomb, Slot::Elixir];
pub(super) const MAX_BOMBS: i32 = 9;
pub(super) const MAX_ELIXIRS: i32 = 3;
/// Frames from dropping a bomb to its blast.
const FUSE: i32 = 90;
/// Blast radius (logic units) and damage.
const BLAST_R: f32 = 30.0;
const BLAST_DMG: f32 = 8.0;

impl Slot {
    pub(super) fn name(self) -> &'static str {
        match self {
            Slot::Potion => "MANA POTION",
            Slot::Antidote => "ANTIDOTE",
            Slot::Bomb => "BOMB",
            Slot::Elixir => "ELIXIR",
        }
    }
}

/// A lit bomb on the ground.
#[derive(Clone, Copy, Debug)]
pub(super) struct Bomb {
    pub x: f32,
    pub y: f32,
    pub t: i32,
}

impl Game {
    pub(super) fn slot(&self) -> Slot {
        SLOTS[self.s.bag_sel.min(SLOTS.len() - 1)]
    }
    pub(super) fn slot_count(&self, s: Slot) -> i32 {
        match s {
            Slot::Potion => self.s.potions,
            Slot::Antidote => self.s.antidotes,
            Slot::Bomb => self.s.bombs,
            Slot::Elixir => self.s.elixirs,
        }
    }
    /// Select the next (d = 1) or previous (d = -1) bag item.
    pub(super) fn cycle_bag(&mut self, d: i32) {
        let n = SLOTS.len() as i32;
        self.s.bag_sel = ((self.s.bag_sel as i32 + d).rem_euclid(n)) as usize;
        let s = self.slot();
        let (x, y) = (self.pl.x, self.pl.y);
        if !self.paused {
            self.float(format!("{} X{}", s.name(), self.slot_count(s)), x - 40.0, y - 22.0, WHITE);
        }
        self.sfx(Sfx::Select);
    }
    /// The potion button: use the selected item.
    pub(super) fn use_bag(&mut self) {
        let (x, y) = (self.pl.x, self.pl.y);
        match self.slot() {
            Slot::Potion => {
                // The same button cures poison first when you carry an antidote.
                if self.poison > 0 && self.s.antidotes > 0 {
                    self.use_antidote();
                } else {
                    self.drink_potion();
                }
            }
            Slot::Antidote => {
                if self.s.antidotes <= 0 {
                    self.float("NO ANTIDOTES", x - 44.0, y - 18.0, rgb(0x747474));
                    self.sfx(Sfx::Deny);
                } else if self.poison <= 0 {
                    self.float("NOT POISONED", x - 44.0, y - 18.0, rgb(0x58d854));
                } else {
                    self.use_antidote();
                }
            }
            Slot::Bomb => {
                if self.s.bombs <= 0 {
                    self.float("NO BOMBS", x - 32.0, y - 18.0, rgb(0x747474));
                    self.sfx(Sfx::Deny);
                } else if self.bombs.len() < 3 {
                    self.s.bombs -= 1;
                    // Dropped just in front of the mage, like the bombs in Zelda.
                    let (dx, dy) = match self.pl.dir {
                        b'u' => (0.0, -20.0),
                        b'l' => (-11.0, 3.0),
                        b'r' => (11.0, 3.0),
                        _ => (0.0, 11.0),
                    };
                    self.bombs.push(Bomb { x: x + dx, y: y + dy, t: 0 });
                    self.sfx(Sfx::Ignite);
                }
            }
            Slot::Elixir => {
                if self.s.elixirs <= 0 {
                    self.float("NO ELIXIRS", x - 40.0, y - 18.0, rgb(0x747474));
                    self.sfx(Sfx::Deny);
                } else if self.s.hp >= self.s.max_hp && self.s.mp >= self.s.max_mp as f32 - 0.5 && self.poison <= 0 {
                    self.float("ALREADY FULL", x - 44.0, y - 18.0, rgb(0xfcbc3c));
                } else {
                    self.s.elixirs -= 1;
                    self.s.hp = self.s.max_hp;
                    self.s.mp = self.s.max_mp as f32;
                    if self.poison > 0 {
                        self.cure_poison();
                    }
                    self.float("ELIXIR! FULLY RESTORED", x - 80.0, y - 18.0, rgb(0xfcbc3c));
                    self.part(x, y, 0.0, 0.0, 20, rgb(0xfce078), 1, PK::Glow(18.0));
                    self.sfx(Sfx::Heal);
                }
            }
        }
    }

    pub(super) fn update_bombs(&mut self) {
        let mut blasts = vec![];
        for b in self.bombs.iter_mut() {
            b.t += 1;
            if b.t >= FUSE {
                blasts.push((b.x, b.y));
            }
        }
        self.bombs.retain(|b| b.t < FUSE);
        for (x, y) in blasts {
            self.bomb_blast(x, y);
        }
    }
    fn bomb_blast(&mut self, x: f32, y: f32) {
        let el = Elem::Neutral;
        let mut en = std::mem::take(&mut self.enemies);
        for e in en.iter_mut() {
            if !e.dead && e.active() && dist(x, y, e.x, e.y) < BLAST_R + e.w * 0.5 {
                if e.st.break_freeze() {
                    let (ex, ey, w, h) = (e.x, e.y, e.w, e.h);
                    self.shatter(ex, ey, w, h);
                }
                e.st.stun(30);
                self.damage_enemy(e, BLAST_DMG, el);
            }
        }
        en.append(&mut self.enemies);
        self.enemies = en;
        if let Some((bx, by, bw, _)) = self.boss_box() {
            if dist(x, y, bx, by) < BLAST_R + bw * 0.5 {
                self.boss_hit(BLAST_DMG, el, x, y, 0.0, 0.0);
            }
        }
        self.bomb_cracks(x, y);
        self.boom(x, y, 30, el);
        for i in 0..16 {
            let a = i as f32 * PI / 8.0;
            let col = if i % 2 == 0 { rgb(0xfc9838) } else { rgb(0xfce040) };
            self.part(x, y, a.cos() * 2.2, a.sin() * 2.2, 18, col, 2, PK::Dot);
        }
        self.part(x, y, 0.0, 0.0, 14, WHITE, 1, PK::Glow(BLAST_R));
        self.shake = 12;
        self.sfx(Sfx::BigBoom);
    }
    /// Bombs break cracked walls within the blast (like Quake, but only nearby).
    fn bomb_cracks(&mut self, x: f32, y: f32) {
        if self.dungeon.is_none() || self.in_lair > 0 {
            return;
        }
        for r in 0..RR as i32 {
            for c in 0..RC as i32 {
                let (tx, ty) = tile_center(c, r);
                if self.tile_at(c, r) == T_CRACK && dist(x, y, tx, ty) < BLAST_R + 12.0 {
                    self.break_crack(c, r);
                    return;
                }
            }
        }
    }

    /// Bombs behind the mage are drawn before her, bombs in front after (`front`).
    pub(super) fn draw_bombs(&self, scr: &mut Screen, front: bool) {
        for b in self.bombs.iter().filter(|b| (b.y > self.pl.y) == front) {
            let (x, y) = (b.x as i32, b.y as i32);
            let left = FUSE - b.t;
            let blink = left < 30 && (b.t / 3) % 2 == 0;
            scr.blend_ellipse(x, y + 5, 6, 2, BLACK, 0.35);
            let body = if blink { rgb(0xd82800) } else { rgb(0x2c2c3c) };
            scr.disc(x, y, 5, body);
            scr.disc(x - 2, y - 2, 1, rgb(0x8c8ca8));
            scr.fill(x - 1, y - 7, 3, 2, rgb(0x5c5c64));
            scr.line(x + 1, y - 7, x + 3, y - 10, rgb(0xa86030));
            let c = if self.frame % 4 < 2 { rgb(0xfce040) } else { rgb(0xfc9838) };
            scr.fill(x + 3, y - 11, 2, 2, c);
        }
    }
}
