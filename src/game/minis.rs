//! One-time overworld encounters (each lives on a fixed screen, has its own save flag,
//! shows on the map once discovered and never blocks the way to the lairs):
//!
//! * HOARD DRAGON - sleeps on a gold pile in a dead end. It can't be killed; pestering it
//!   and taking its gold makes it angrier. When the pile is gone it flies off for good.
//! * DECEIVING DRYAD - a friendly "villager" who leads you to a poison pool on the next
//!   screen, then reveals herself. Tells: no shadow, flowers wilt where she walks.
//! * FOOD TREES + ANGRY TREANT - shake or shoot fruit trees for apples (they regrow);
//!   one tree is a sleeping treant.
//! * GRAVEYARD - lightning raises zombies from the graves; enough of them wakes the Grave Lord.
use super::bosses::{draw_flames, Hazard, HK};
use super::*;

pub(super) const HOARD_TOTAL: i32 = 24;
pub(super) const ZOMBIES_FOR_LORD: i32 = 6;
pub(super) const POISON_TIME: i32 = 480;
const POISON_TICK: i32 = 70;
const FRUIT_MAX: i32 = 3;
/// A shaken-bare fruit tree regrows after three minutes.
pub(super) const REGROW: i32 = 3 * 60 * 60;

pub(super) struct Fruit {
    pub room: usize,
    pub c: i32,
    pub r: i32,
    pub fruit: i32,
    pub regrow: i32,
    pub shake: i32,
    pub cool: i32,
    pub treant: bool,
}

/// Damage multiplier including encounter creatures' special weaknesses.
pub(super) fn enemy_mult(e: &Enemy, el: Elem) -> f32 {
    match e.k {
        // Rotting and dry things burn: neutral, but weak to fire.
        EK::Zombie | EK::GraveLord | EK::Dryad if el == Elem::Fire => 2.0,
        _ => mult(el, e.el),
    }
}

impl Game {
    fn bit(id: u8) -> u32 {
        1 << id
    }
    fn done(&self, id: u8) -> bool {
        self.s.mini_done(id)
    }
    fn finish_mini(&mut self, id: u8) {
        self.s.mini_done |= Self::bit(id);
        self.save();
    }
    fn room_mini(&self) -> u8 {
        if self.overworld() {
            self.rooms[self.room].mini
        } else {
            0
        }
    }
    fn theme_now(&self) -> usize {
        self.rooms[self.room].theme.min(3)
    }

    // ------------------------------------------------------------ setup & room entry
    /// Build runtime state from the world and the save (fruit trees, a departed treant).
    pub(super) fn setup_minis(&mut self) {
        self.fruit.clear();
        let treant_done = self.done(MINI_TREANT);
        let themes = &self.themes;
        for r in self.rooms.iter_mut() {
            for (i, &(c, rr)) in r.trees.iter().enumerate() {
                let treant = r.treant == Some(i);
                if treant && treant_done {
                    continue;
                }
                self.fruit.push(Fruit { room: r.i, c, r: rr, fruit: FRUIT_MAX, regrow: 0, shake: 0, cool: 0, treant });
            }
            if treant_done {
                if let Some(ti) = r.treant.take() {
                    let (c, rr) = r.trees.remove(ti);
                    r.tiles[rr as usize][c as usize] = T_FLOOR;
                    let th = r.theme;
                    render(r, &themes[th]);
                }
            }
        }
        self.dryad_stage = 0;
        self.poison = 0;
    }

    pub(super) fn enter_mini_room(&mut self) {
        self.hazards.clear();
        self.mini_hint = false;
        let (mini, n_graves, has_pool) = {
            let r = &self.rooms[self.room];
            (r.mini, r.graves.len(), r.pool.is_some())
        };
        if mini != 0 {
            self.s.mini_seen |= Self::bit(mini);
        }
        self.graves_open = vec![false; n_graves];
        let th = self.theme_now();
        match mini {
            MINI_HOARD if !self.done(MINI_HOARD) => {
                if self.s.hoard_left < 0 {
                    self.s.hoard_left = HOARD_TOTAL;
                }
                let left = self.s.hoard_left;
                for i in 0..left {
                    let a = i as f32 * 2.4;
                    let rad = 10.0 + (i % 4) as f32 * 7.0;
                    let (x, y) = (128.0 + a.cos() * rad * 1.5, 138.0 + a.sin() * rad * 0.6);
                    self.items.push(Item {
                        kind: IK::Coin, x, y, val: 5 * (th as i32 + 2), el: Elem::Neutral, life: PLACED_LIFE, dead: false,
                        tag: ITEM_HOARD,
                    });
                }
                let taken = 1.0 - left as f32 / HOARD_TOTAL as f32;
                self.hoard_anger = taken * 0.6;
                let mut d = self.make_mini(EK::HoardDragon, MINI_HOARD, 128.0, 116.0);
                d.mode = if taken > 0.0 { 1 } else { 0 };
                d.touch = if d.mode == 1 { 3 } else { 0 };
                d.timer = 90;
                self.enemies.push(d);
                if taken > 0.0 {
                    self.show_msg("THE DRAGON REMEMBERS YOU, THIEF. IT GLARES FROM ITS HOARD.");
                } else {
                    self.show_msg("A DRAGON SLEEPS ON A MOUNTAIN OF GOLD...");
                }
            }
            MINI_DRYAD if !self.done(MINI_DRYAD) && self.dryad_stage == 0 => {
                let mut d = self.make_mini(EK::Dryad, MINI_DRYAD, 128.0, 150.0);
                d.touch = 0;
                self.enemies.push(d);
                self.show_msg("A VILLAGER WAVES: FOLLOW ME, TRAVELLER! I KNOW A SAFE PATH.");
            }
            MINI_TREANT => {}
            MINI_GRAVE => {
                if self.s.zombies >= ZOMBIES_FOR_LORD && !self.done(MINI_GRAVE) {
                    self.raise_grave_lord();
                } else {
                    self.show_msg("OLD GRAVES... THE DEAD HERE STIR AT THE TOUCH OF LIGHTNING.");
                }
            }
            _ => {}
        }
        // The dryad waits for you beyond her poison pool.
        if has_pool && self.dryad_stage == 1 && !self.done(MINI_DRYAD) {
            let (cx, cy) = self.pool_center();
            let a = ang(self.pl.x, self.pl.y, cx, cy);
            let (x, y) = ((cx + a.cos() * 56.0).clamp(28.0, 228.0), (cy + a.sin() * 40.0).clamp(HUDF + 28.0, HF - 28.0));
            let mut d = self.make_mini(EK::Dryad, MINI_DRYAD, x, y);
            d.touch = 0;
            d.timer = -1; // waiting, not leading
            self.enemies.push(d);
            self.show_msg("THE VILLAGER BECKONS: THIS WAY... JUST A LITTLE FURTHER...");
        }
    }
    fn make_mini(&mut self, k: EK, id: u8, x: f32, y: f32) -> Enemy {
        let th = self.theme_now();
        let mut e = self.make_enemy(k, x, y, th);
        e.mini = id;
        e.spawn = 0;
        e
    }
    fn pool_center(&self) -> (f32, f32) {
        let (c, r) = self.rooms[self.room].pool.unwrap_or((6, 5));
        let (x, y) = tile_center(c, r);
        (x + (POOL_W - 1) as f32 * 8.0, y + (POOL_H - 1) as f32 * 8.0)
    }
    fn in_pool(&self, x: f32, y: f32) -> bool {
        let Some((c, r)) = self.rooms[self.room].pool else { return false };
        let (x0, y0) = ((c * TS) as f32, (HUD + r * TS) as f32);
        x > x0 + 2.0 && x < x0 + (POOL_W * TS) as f32 - 2.0 && y > y0 + 2.0 && y < y0 + (POOL_H * TS) as f32 - 2.0
    }

    // ------------------------------------------------------------ per-frame
    pub(super) fn update_minis(&mut self) {
        for f in self.fruit.iter_mut() {
            if f.shake > 0 {
                f.shake -= 1;
            }
            if f.cool > 0 {
                f.cool -= 1;
            }
            if f.regrow > 0 {
                f.regrow -= 1;
                if f.regrow == 0 {
                    f.fruit = FRUIT_MAX;
                }
            }
        }
        if !self.overworld() {
            return;
        }
        self.update_mini_hazards();
        // Poison pool.
        if self.rooms[self.room].pool.is_some() && self.in_pool(self.pl.x, self.pl.y + 4.0) {
            if self.poison <= 0 {
                self.float("POISONED!", self.pl.x - 36.0, self.pl.y - 20.0, rgb(0x58d854));
                self.sfx(Sfx::Hurt);
            }
            self.poison = POISON_TIME;
            let revealed = self.reveal_dryad();
            if revealed {
                self.show_msg("FOOLISH MORTAL! THE VILLAGER TWISTS INTO A THORNED DRYAD!");
            }
        }
    }
    fn update_mini_hazards(&mut self) {
        let (px, py) = (self.pl.x, self.pl.y);
        let mut hurt = 0;
        let mut landed: Vec<(f32, f32)> = vec![];
        for h in self.hazards.iter_mut() {
            if h.warn > 0 {
                h.warn -= 1;
                if h.warn == 0 {
                    landed.push((h.x, h.y));
                }
                continue;
            }
            if h.act > 0 {
                h.act -= 1;
                if h.hits(px, py) {
                    hurt = hurt.max(h.dmg);
                }
            }
        }
        self.hazards.retain(|h| h.warn > 0 || h.act > 0);
        for (x, y) in landed {
            self.impact(x, y, Elem::Earth);
        }
        if hurt > 0 {
            self.hurt(hurt);
        }
    }

    // ------------------------------------------------------------ player statuses
    pub(super) fn tick_player_status(&mut self) {
        if self.poison <= 0 {
            return;
        }
        self.poison -= 1;
        if self.poison % 9 == 0 {
            let ox = self.rng.range(-4.0, 4.0);
            self.part(self.pl.x + ox, self.pl.y - 6.0, 0.0, -0.4, 22, rgb(0x58d854), 1, PK::Dot);
        }
        if self.poison % POISON_TICK == 0 && self.s.hp > 1 {
            self.s.hp -= 1;
            self.float("-1", self.pl.x - 6.0, self.pl.y - 16.0, rgb(0x58d854));
        }
        if self.poison == 0 {
            self.float("POISON WORE OFF", self.pl.x - 60.0, self.pl.y - 20.0, rgb(0xb8f818));
        }
    }
    pub(super) fn cure_poison(&mut self) {
        self.poison = 0;
        self.float("CURED!", self.pl.x - 24.0, self.pl.y - 20.0, rgb(0xb8f818));
        self.part(self.pl.x, self.pl.y, 0.0, 0.0, 14, rgb(0xb8f818), 1, PK::Glow(12.0));
        self.sfx(Sfx::Heal);
    }
    pub(super) fn use_antidote(&mut self) {
        if self.s.antidotes > 0 && self.poison > 0 {
            self.s.antidotes -= 1;
            self.cure_poison();
        }
    }

    // ------------------------------------------------------------ damage hooks
    /// React to a hit. Returns false when the hit does no damage.
    pub(super) fn mini_hit(&mut self, e: &mut Enemy, _el: Elem) -> bool {
        match e.k {
            EK::HoardDragon => {
                self.hoard_anger = (self.hoard_anger + 0.12).min(1.5);
                if e.mode == 0 {
                    self.wake_dragon(e);
                    self.show_msg("THE DRAGON WAKES! IT CAN'T BE HURT... BUT IT HATES BEING PESTERED.");
                }
                if e.tip <= 0 {
                    e.tip = 30;
                    self.float("GRRR!", e.x - 20.0, e.y - 24.0, rgb(0xfc9838));
                }
                e.flash = 3;
                self.spark(e.x, e.y, rgb(0xfcbc3c));
                self.sfx(Sfx::Click);
                false
            }
            EK::Dryad if e.mode == 0 => {
                self.dryad_reveal(e);
                self.show_msg("YOU SEE THROUGH HER DISGUISE! THE DRYAD ATTACKS!");
                true
            }
            _ => true,
        }
    }
    fn wake_dragon(&mut self, e: &mut Enemy) {
        e.mode = 1;
        e.touch = 3;
        e.timer = 50;
        self.sfx(Sfx::Roar);
        self.shake = 10;
    }
    fn dryad_reveal(&mut self, e: &mut Enemy) {
        e.mode = 1;
        e.touch = 3;
        e.timer = 50;
        self.dryad_stage = 2;
        self.part(e.x, e.y, 0.0, 0.0, 16, rgb(0x58d854), 1, PK::Glow(18.0));
        self.sfx(Sfx::Roar);
    }
    /// Reveal a friendly dryad on this screen (stepping in her pool).
    fn reveal_dryad(&mut self) -> bool {
        let mut en = std::mem::take(&mut self.enemies);
        let mut any = false;
        for e in en.iter_mut() {
            if e.k == EK::Dryad && e.mode == 0 && !e.dead {
                self.dryad_reveal(e);
                any = true;
            }
        }
        en.append(&mut self.enemies);
        self.enemies = en;
        any
    }
    pub(super) fn mini_defeated(&mut self, e: &Enemy) {
        let th = self.theme_now() as i32;
        match e.k {
            EK::Dryad => {
                self.s.max_hp += 4;
                self.s.hp = self.s.max_hp;
                self.show_msg("THE DRYAD WITHERS TO DUST. AMONG THE THORNS: A HEART CONTAINER! MAX LIFE UP.");
                self.sfx(Sfx::Fanfare);
                self.finish_mini(MINI_DRYAD);
            }
            EK::Treant => {
                self.add_item(IK::GoldenApple, e.x, e.y, 0, Elem::Neutral);
                self.items.last_mut().unwrap().life = PLACED_LIFE;
                self.show_msg("THE TREANT CRASHES DOWN, LEAVING A GLOWING GOLDEN APPLE.");
                self.sfx(Sfx::Fanfare);
                self.finish_mini(MINI_TREANT);
            }
            EK::GraveLord => {
                for i in 0..6 {
                    let a = i as f32 * PI / 3.0;
                    self.add_item(IK::Coin, e.x + a.cos() * 14.0, e.y + a.sin() * 10.0, 10 * (th + 1), Elem::Neutral);
                }
                self.s.max_mp += 20;
                self.s.mp = self.s.max_mp as f32;
                self.show_msg("THE GRAVE LORD FALLS. YOU CLAIM HIS AMULET: MAXIMUM MANA UP!");
                self.sfx(Sfx::Fanfare);
                self.finish_mini(MINI_GRAVE);
            }
            _ => {}
        }
    }
    pub(super) fn hoard_coin_taken(&mut self) {
        if self.done(MINI_HOARD) {
            return;
        }
        self.s.hoard_left = (self.s.hoard_left - 1).max(0);
        self.hoard_anger = (self.hoard_anger + 1.5 / HOARD_TOTAL as f32).min(1.5);
        let wake = self.rng.f() < 0.35 || self.hoard_anger > 0.3;
        let left = self.s.hoard_left;
        let mut en = std::mem::take(&mut self.enemies);
        for e in en.iter_mut().filter(|e| e.k == EK::HoardDragon && !e.dead) {
            if left == 0 {
                e.mode = 2;
                e.timer = 0;
                e.touch = 0;
                self.sfx(Sfx::Roar);
                self.shake = 14;
            } else if e.mode == 0 && wake {
                self.wake_dragon(e);
                self.show_msg("THE CLINK OF GOLD WAKES THE DRAGON!");
            }
        }
        en.append(&mut self.enemies);
        self.enemies = en;
        if left == 0 {
            self.show_msg("WITH ITS HOARD GONE, THE DRAGON TAKES TO THE SKY IN DISGUST!");
            self.finish_mini(MINI_HOARD);
        }
        self.save();
    }

    // ------------------------------------------------------------ behaviours
    pub(super) fn upd_mini_enemy(&mut self, e: &mut Enemy, f: f32) {
        let (px, py) = (self.pl.x, self.pl.y);
        let d = dist(e.x, e.y, px, py);
        let a = ang(e.x, e.y, px, py);
        let th = e.lvl as f32;
        match e.k {
            EK::HoardDragon => match e.mode {
                0 => {
                    if e.t % 50 == 0 {
                        self.float("Z", e.x + 10.0, e.y - 16.0, rgb(0xa4e4fc));
                    }
                }
                1 => {
                    e.vx = a.cos();
                    e.timer -= 1;
                    if e.timer <= 0 {
                        let anger = self.hoard_anger;
                        if d < 52.0 {
                            // Claw swipe.
                            self.hazards.push(Hazard::circle(HK::Tail, e.x, e.y + 6.0, 32.0, 0, 10, 3));
                            for i in 0..8 {
                                let sa = i as f32 * PI / 4.0;
                                self.eshot_x(e.x, e.y + 4.0, sa, 2.4, Elem::Neutral, Shot::Wave, 3.0, 2.0, 14);
                            }
                            self.sfx(Sfx::Shoot);
                        } else {
                            e.mode = 3;
                            e.turn = 20 + (anger * 30.0) as i32;
                            self.sfx(Sfx::Roar);
                        }
                        e.timer = (140.0 - anger * 90.0).max(40.0) as i32;
                    }
                }
                3 => {
                    // Fire breath: longer the angrier it is.
                    e.turn -= 1;
                    if e.turn % 3 == 0 {
                        let j = self.rng.range(-0.18, 0.18);
                        let (mx, my) = (e.x + a.cos() * 16.0, e.y - 4.0);
                        let s = 2.2 + self.rng.f() * 0.4;
                        self.eshot_x(mx, my, a + j, s, Elem::Fire, Shot::Flame, 3.0, 2.0, 50);
                    }
                    if e.turn <= 0 {
                        e.mode = 1;
                    }
                }
                _ => {
                    // Flying away with no gold left to guard.
                    e.timer += 1;
                    e.y -= 1.6;
                    if e.timer % 6 == 0 {
                        self.embers(e.x, e.y + 8.0, 2, 1.0);
                    }
                    if e.timer > 100 {
                        e.dead = true;
                    }
                    return;
                }
            },
            EK::Dryad => {
                if e.mode == 0 {
                    if e.timer >= 0 {
                        // Leading the way toward her pool's screen, waiting if you fall behind.
                        let dir = self.rooms[self.room].mini_dir;
                        let (tx, ty) = match dir {
                            0 => (128.0, HUDF + 10.0),
                            1 => (128.0, HF - 10.0),
                            2 => (WF - 10.0, 136.0),
                            _ => (10.0, 136.0),
                        };
                        if d < 90.0 {
                            let ta = ang(e.x, e.y, tx, ty);
                            e.x += ta.cos() * 0.7;
                            e.y += ta.sin() * 0.7;
                            e.turn += 1;
                            if e.turn % 10 == 0 {
                                // Flowers wilt where she walks.
                                self.part(e.x, e.y + 6.0, 0.0, 0.0, 420, rgb(0x6c5c3c), 1, PK::Dot);
                                self.part(e.x + 1.0, e.y + 5.0, 0.0, 0.0, 420, rgb(0x4c3c2c), 1, PK::Dot);
                            }
                        }
                        if dist(e.x, e.y, tx, ty) < 10.0 {
                            e.dead = true;
                            self.dryad_stage = 1;
                        }
                    }
                    return;
                }
                // Revealed: keeps a thorny distance, volleys thorns, lashes with vines up close.
                let want = if d < 44.0 { -1.0 } else if d > 76.0 { 1.0 } else { 0.0 };
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, a.cos() * e.spd * f * want, a.sin() * e.spd * f * want);
                e.timer -= 1;
                if e.timer <= 0 {
                    e.turn += 1;
                    if e.turn % 3 == 0 {
                        self.hazards.push(Hazard::circle(HK::Root, e.x, e.y + 4.0, 28.0, 24, 12, 3));
                    } else {
                        for i in 0..5 {
                            let da = (i as f32 - 2.0) * 0.2;
                            self.eshot_x(e.x, e.y - 2.0, a + da, 1.8 + th * 0.1, Elem::Neutral, Shot::Thorn, 2.5, 2.0, 200);
                        }
                        self.sfx(Sfx::Shoot);
                    }
                    e.timer = 80 - e.lvl * 6;
                }
            }
            EK::Treant => {
                self.move_box(&mut e.x, &mut e.y, e.w * 0.6, e.h * 0.5, a.cos() * e.spd * f, a.sin() * e.spd * f);
                e.timer -= 1;
                if e.timer <= 0 {
                    match e.turn % 3 {
                        0 => {
                            for i in 0..3 {
                                let (ox, oy) = if i == 0 { (0.0, 0.0) } else { (self.rng.range(-30.0, 30.0), self.rng.range(-24.0, 24.0)) };
                                self.hazards.push(Hazard::circle(HK::Root, px + ox, py + oy, 11.0, 40, 16, 3));
                            }
                            self.sfx(Sfx::Rumble);
                        }
                        1 => {
                            for i in -1..=1 {
                                self.eshot_x(e.x, e.y - 10.0, a + i as f32 * 0.25, 1.8, Elem::Neutral, Shot::Apple, 3.0, 2.0, 200);
                            }
                            self.sfx(Sfx::Shoot);
                        }
                        _ => {
                            for i in 0..12 {
                                let sa = i as f32 * PI / 6.0;
                                self.eshot_x(e.x, e.y + 10.0, sa, 1.4, Elem::Earth, Shot::Wave, 3.0, 2.0, 250);
                            }
                            self.shake = 10;
                            self.sfx(Sfx::Quake);
                        }
                    }
                    e.turn += 1;
                    e.timer = 110 - e.lvl * 8;
                }
            }
            EK::Zombie => {
                // A slow shamble with a lurching gait.
                let lurch = if (e.t / 20) % 2 == 0 { 1.0 } else { 0.4 };
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, a.cos() * e.spd * f * lurch, a.sin() * e.spd * f * lurch);
            }
            EK::GraveLord => {
                let want = if d < 50.0 { -1.0 } else if d > 90.0 { 1.0 } else { 0.0 };
                e.x += a.cos() * e.spd * f * want;
                e.y += a.sin() * e.spd * f * want;
                e.timer -= 1;
                if e.timer <= 0 {
                    match e.turn % 3 {
                        0 => {
                            for i in 0..2 {
                                if self.enemies.len() >= 5 {
                                    break;
                                }
                                let (x, y) = (e.x + (i as f32 * 2.0 - 1.0) * 26.0, e.y + 18.0);
                                let mut z = self.make_enemy(EK::Zombie, x.clamp(24.0, 232.0), y.clamp(HUDF + 24.0, HF - 24.0), e.lvl as usize);
                                z.spawn = 40;
                                self.enemies.push(z);
                            }
                            self.sfx(Sfx::Rumble);
                        }
                        1 => {
                            for i in 0..10 {
                                let sa = i as f32 * PI / 5.0 + e.t as f32 * 0.01;
                                self.eshot_x(e.x, e.y, sa, 1.3, Elem::Neutral, Shot::Dark, 2.5, 2.0, 400);
                            }
                            self.sfx(Sfx::Spell);
                        }
                        _ => {
                            // Blink to a new spot.
                            self.part(e.x, e.y, 0.0, 0.0, 12, rgb(0x9818a8), 1, PK::Glow(14.0));
                            if let Some((x, y)) = self.free_spot(70.0, false) {
                                e.x = x;
                                e.y = y;
                            }
                            self.part(e.x, e.y, 0.0, 0.0, 12, rgb(0x9818a8), 1, PK::Ring);
                            self.sfx(Sfx::Zap);
                        }
                    }
                    e.turn += 1;
                    e.timer = 95 - e.lvl * 6;
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ trees and graves
    fn fruit_at(&self, c: i32, r: i32) -> Option<usize> {
        let room = self.room;
        self.fruit.iter().position(|f| f.room == room && f.c == c && f.r == r)
    }
    fn grave_at(&self, c: i32, r: i32) -> Option<usize> {
        self.rooms[self.room].graves.iter().position(|&g| g == (c, r))
    }
    /// A player bolt struck a solid overworld tile.
    pub(super) fn world_bolt_hit(&mut self, c: i32, r: i32, el: Elem) {
        if !self.overworld() {
            return;
        }
        if let Some(i) = self.fruit_at(c, r) {
            self.shake_tree(i);
        } else if let Some(g) = self.grave_at(c, r) {
            if el == Elem::Storm {
                self.raise_zombie(g);
            } else if !self.mini_hint {
                self.mini_hint = true;
                let (x, y) = tile_center(c, r);
                self.float("THE DEAD SLEEP", x - 56.0, y - 18.0, rgb(0xbcbcbc));
            }
        }
    }
    /// Walking into a fruit tree shakes it too.
    pub(super) fn overworld_bump(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        let axis = if ix != 0.0 && iy == 0.0 && blocked.0 {
            (ix as i32, 0)
        } else if iy != 0.0 && ix == 0.0 && blocked.1 {
            (0, iy as i32)
        } else {
            return;
        };
        let probe = (self.pl.x + axis.0 as f32 * (self.pl.w / 2.0 + 3.0), self.pl.y + axis.1 as f32 * (self.pl.h / 2.0 + 3.0));
        let (c, r) = tile_of(probe.0, probe.1);
        if let Some(i) = self.fruit_at(c, r) {
            if self.fruit[i].cool == 0 {
                self.shake_tree(i);
            }
        }
    }
    fn shake_tree(&mut self, i: usize) {
        let (c, r, treant) = (self.fruit[i].c, self.fruit[i].r, self.fruit[i].treant);
        let (x, y) = tile_center(c, r);
        if treant && !self.done(MINI_TREANT) {
            // The "tree" was a sleeping treant.
            self.fruit.remove(i);
            let ri = self.room;
            if let Some(ti) = self.rooms[ri].treant.take() {
                self.rooms[ri].trees.remove(ti);
            }
            self.rooms[ri].tiles[r as usize][c as usize] = T_FLOOR;
            let th = self.rooms[ri].theme;
            render(&mut self.rooms[ri], &self.themes[th]);
            let mut t = self.make_mini(EK::Treant, MINI_TREANT, x, y + 2.0);
            t.spawn = 30;
            t.timer = 70;
            self.enemies.push(t);
            self.show_msg("THE TREE SHUDDERS AND UPROOTS ITSELF... AN ANGRY TREANT!");
            self.sfx(Sfx::Roar);
            self.shake = 12;
            return;
        }
        let f = &mut self.fruit[i];
        f.shake = 12;
        f.cool = 24;
        if f.fruit > 0 {
            f.fruit -= 1;
            if f.fruit == 0 {
                f.regrow = REGROW;
            }
            let ox = self.rng.range(-6.0, 6.0);
            self.add_item(IK::Apple, x + ox, y + 14.0, 0, Elem::Neutral);
            self.sfx(Sfx::Pickup);
        } else {
            self.float("NO FRUIT", x - 32.0, y - 18.0, rgb(0x747474));
        }
    }
    fn raise_zombie(&mut self, g: usize) {
        if self.graves_open.get(g).copied().unwrap_or(true) {
            return;
        }
        self.graves_open[g] = true;
        let (c, r) = self.rooms[self.room].graves[g];
        let (x, y) = tile_center(c, r);
        let th = self.theme_now();
        for k in 0..2 {
            self.add_item(IK::Coin, x - 6.0 + k as f32 * 12.0, y + 12.0, 4 * (th as i32 + 1), Elem::Neutral);
        }
        let mut z = self.make_enemy(EK::Zombie, x, y + 16.0, th);
        z.spawn = 40;
        self.enemies.push(z);
        for _ in 0..10 {
            let vx = self.rng.range(-1.2, 1.2);
            self.part(x + vx * 4.0, y + 12.0, vx, -1.2, 22, rgb(0x5c3c10), 2, PK::Shard);
        }
        self.part(x, y, 0.0, 0.0, 10, rgb(0xfce040), 1, PK::Burst(Elem::Storm, 10.0));
        self.sfx(Sfx::Rumble);
        self.s.zombies += 1;
        if self.s.zombies >= ZOMBIES_FOR_LORD && !self.done(MINI_GRAVE) && !self.enemies.iter().any(|e| e.k == EK::GraveLord) {
            self.raise_grave_lord();
        }
        self.save();
    }
    fn raise_grave_lord(&mut self) {
        let mut l = self.make_mini(EK::GraveLord, MINI_GRAVE, 128.0, 128.0);
        l.spawn = 60;
        l.timer = 90;
        self.enemies.push(l);
        self.show_msg("THE EARTH SPLITS... THE GRAVE LORD RISES!");
        self.sfx(Sfx::Roar);
        self.shake = 16;
        self.flash = 8;
    }
    /// Chain Bolt arcs into up to three nearby graves.
    pub(super) fn chain_graves(&mut self, px: f32, py: f32) {
        if self.room_mini() != MINI_GRAVE {
            return;
        }
        let graves = self.rooms[self.room].graves.clone();
        let mut n = 0;
        for (g, &(c, r)) in graves.iter().enumerate() {
            let (x, y) = tile_center(c, r);
            if n < 3 && !self.graves_open[g] && dist(px, py, x, y) < 150.0 {
                self.part(px, py, 0.0, 0.0, 12, rgb(0xfce040), 1, PK::Line(x, y));
                self.raise_zombie(g);
                n += 1;
            }
        }
    }

    // ============================================================ drawing
    /// Trees, graves, the poison pool and the hoard pile on an overworld screen.
    pub(super) fn draw_mini_room(&self, scr: &mut Screen, ri: usize, ox: i32, oy: i32) {
        let room = &self.rooms[ri];
        let f = self.frame;
        if let Some((c, r)) = room.pool {
            let (x0, y0) = (c * TS + ox, HUD + r * TS + oy);
            let (w, h) = (POOL_W * TS, POOL_H * TS);
            scr.blend_ellipse(x0 + w / 2, y0 + h / 2, w / 2, h / 2, rgb(0x2c5c1c), 0.85);
            scr.blend_ellipse(x0 + w / 2, y0 + h / 2, w / 2 - 4, h / 2 - 3, rgb(0x58a828), 0.6);
            for i in 0..4 {
                let ph = ((f as i32) + i * 23) % 60;
                let (bx, by) = (x0 + 8 + i * 14, y0 + h / 2 + 3 - ph / 12);
                if ph < 48 {
                    scr.ring(bx, by, 1 + ph / 24, rgb(0xb8f818));
                }
            }
        }
        if room.mini == MINI_HOARD && !self.done(MINI_HOARD) {
            let left = self.s.hoard_left.max(0).min(HOARD_TOTAL);
            let size = if self.s.hoard_left < 0 { 1.0 } else { left as f32 / HOARD_TOTAL as f32 };
            let (cx, cy) = (128 + ox, 138 + oy);
            let rx = (8.0 + 30.0 * size) as i32;
            let ry = (4.0 + 10.0 * size) as i32;
            scr.blend_ellipse(cx, cy + 3, rx + 2, ry / 2 + 2, BLACK, 0.3);
            for layer in 0..4 {
                let k = 1.0 - layer as f32 * 0.22;
                let c = [rgb(0xa86000), rgb(0xd88c10), rgb(0xfcbc3c), rgb(0xfce078)][layer];
                scr.blend_ellipse(cx, cy - layer as i32 * (ry / 3).max(1), (rx as f32 * k) as i32, (ry as f32 * k * 0.6) as i32 + 1, c, 1.0);
            }
            if f % 20 < 3 {
                scr.pset(cx - rx / 2 + (f as i32 % 17), cy - 3, WHITE);
            }
        }
        for fr in self.fruit.iter().filter(|fr| fr.room == ri) {
            let (x, y) = tile_center(fr.c, fr.r);
            let (x, y) = (x as i32 + ox, y as i32 + oy);
            let sx = if fr.shake > 0 { (fr.shake % 4) - 2 } else { 0 };
            scr.blend_ellipse(x, y + 7, 9, 3, BLACK, 0.35);
            scr.fill(x - 2, y + 1, 4, 7, rgb(0x5c3410));
            scr.fill(x - 2, y + 1, 1, 7, rgb(0x8c5020));
            scr.disc(x + sx, y - 5, 9, rgb(0x0c4414));
            scr.disc(x + sx, y - 6, 8, rgb(0x1c7a24));
            scr.disc(x - 2 + sx, y - 8, 4, rgb(0x2c9030));
            for (k, &(ax, ay)) in [(-4, -7), (3, -9), (5, -3), (-2, -2), (0, -11)].iter().enumerate() {
                if (k as i32) < fr.fruit + (fr.fruit > 0) as i32 * 2 {
                    scr.disc(x + ax + sx, y + ay, 1, rgb(0xd82800));
                    scr.pset(x + ax + sx - 1, y + ay - 1, rgb(0xfc7460));
                }
            }
            // The disguised treant's only tell: two knots that look a lot like eyes.
            if fr.treant && dist(self.pl.x, self.pl.y, x as f32, y as f32) < 40.0 && f % 90 < 45 {
                scr.pset(x - 2, y + 3, rgb(0xfce040));
                scr.pset(x + 1, y + 3, rgb(0xfce040));
            }
        }
        for (g, &(c, r)) in room.graves.iter().enumerate() {
            let (x, y) = tile_center(c, r);
            let (x, y) = (x as i32 + ox, y as i32 + oy);
            let open = ri == self.room && self.graves_open.get(g).copied().unwrap_or(false);
            scr.blend_ellipse(x, y + 7, 7, 2, BLACK, 0.35);
            scr.fill(x - 5, y - 6, 10, 13, rgb(0x6c6c78));
            scr.fill(x - 4, y - 8, 8, 2, rgb(0x6c6c78));
            scr.fill(x - 5, y - 6, 2, 13, rgb(0x9c9ca8));
            scr.fill(x + 3, y - 6, 2, 13, rgb(0x3c3c48));
            scr.fill(x - 1, y - 4, 2, 7, rgb(0x3c3c48));
            scr.fill(x - 3, y - 2, 6, 2, rgb(0x3c3c48));
            if open {
                scr.fill(x - 6, y + 8, 12, 5, rgb(0x100804));
                scr.fill(x - 7, y + 12, 14, 2, rgb(0x5c3c10));
            } else {
                scr.fill(x - 6, y + 9, 12, 3, rgb(0x3c2c14));
            }
        }
    }
    /// Map marker for a discovered encounter screen (red until finished).
    pub(super) fn mini_marker(&self, id: u8) -> Option<u32> {
        if id == 0 || self.s.mini_seen & Self::bit(id) == 0 {
            return None;
        }
        Some(if self.done(id) { rgb(0x747474) } else { rgb(0xfc3c3c) })
    }
    /// Status icons under the HUD (poison, and future curses).
    pub(super) fn draw_status_icons(&self, scr: &mut Screen) {
        if self.poison <= 0 {
            return;
        }
        let (x, y) = (4, HUD + 3);
        scr.fill(x, y, 78, 12, rgb(0x08140c));
        scr.frame_rect(x, y, 78, 12, rgb(0x2c7c1c));
        scr.disc(x + 6, y + 6, 3, rgb(0x58d854));
        scr.pset(x + 5, y + 5, WHITE);
        scr.text("POISON", x + 12, y + 2, rgb(0xb8f818), Align::Left, 8);
        let w = (18 * self.poison / POISON_TIME).max(1);
        scr.fill(x + 60, y + 4, w, 4, rgb(0x58d854));
        if self.s.antidotes > 0 {
            scr.text(&format!("Y:CURE {}", self.s.antidotes), x + 84, y + 2, rgb(0xb8f818), Align::Left, 8);
        }
    }

    /// Draw an encounter creature. Returns false for ordinary monsters.
    pub(super) fn draw_mini_enemy(&self, scr: &mut Screen, e: &Enemy) -> bool {
        let white = e.flash > 0;
        let c = |col: u32| if white { WHITE } else { rgb(col) };
        let (x, y) = (e.x as i32, e.y as i32);
        let t = e.t;
        match e.k {
            EK::HoardDragon => {
                let awake = e.mode != 0;
                let lift = if e.mode == 2 { e.timer * 2 } else { 0 };
                let y = y - lift / 2;
                let f = if e.vx < 0.0 { -1 } else { 1 };
                let breath = (t as f32 * if awake { 0.2 } else { 0.05 }).sin();
                // Folded (asleep) or half-spread (awake / flying) wings.
                let span = if e.mode == 2 { 26 + (breath * 8.0) as i32 } else if awake { 18 } else { 10 };
                for s in [-1, 1] {
                    for k in 0..6 {
                        scr.line(x + s * 4, y - 6, x + s * span, y - 14 + k * 2 - (breath * 3.0) as i32, c(0x681008));
                    }
                    scr.line(x + s * 4, y - 6, x + s * span, y - 14 - (breath * 3.0) as i32, c(0xb82010));
                }
                scr.disc(x, y, 11 + (breath * 0.8) as i32, c(0xb82010));
                scr.disc(x + f * 2, y + 3, 6, c(0xfc9838));
                for i in 1..5 {
                    scr.disc(x - f * (8 + i * 5), y + 6 - i, (5 - i).max(2), c(0xa01808));
                }
                let (hx, hy) = if awake { (x + f * 16, y - 12) } else { (x + f * 13, y + 2) };
                scr.disc(hx, hy, 5, c(0xb82010));
                scr.disc(hx + f * 5, hy + 1, 3, c(0xb82010));
                scr.line(hx - f * 2, hy - 4, hx - f * 7, hy - 9, c(0xfcfcfc));
                if awake {
                    scr.fill(hx, hy - 2, 2, 2, c(0xfce040));
                    scr.blend_disc(hx + 1, hy - 1, 4, rgb(0xfce040), 0.3);
                    if e.mode == 3 {
                        scr.blend_disc(hx + f * 8, hy + 2, 5, rgb(0xfc9838), 0.5);
                    }
                } else {
                    scr.fill(hx - 1, hy - 1, 3, 1, c(0x3c0c04));
                }
            }
            EK::Dryad => {
                let s = if e.mode == 0 { &self.spr.dryad_friend } else { &self.spr.dryad_true };
                let img = if white { &s.white } else { &s.img };
                // Tell: she casts no shadow while disguised.
                if e.mode != 0 {
                    scr.blend_ellipse(x, y + 8, 5, 2, BLACK, 0.3);
                }
                let bob = if e.mode == 0 && (t / 16) % 2 == 0 { 1.0 } else { 0.0 };
                scr.spr(img, e.x, e.y - bob, e.x > self.pl.x);
            }
            EK::Treant => {
                let sway = ((t as f32) * 0.08).sin() * 2.0;
                scr.blend_ellipse(x, y + 13, 12, 3, BLACK, 0.35);
                for i in 0..4 {
                    let rx = x - 9 + i * 6;
                    let step = if (t / 12 + i) % 2 == 0 { 2 } else { 0 };
                    scr.line(rx, y + 8, rx + (i - 2) * 3, y + 13 - step, c(0x3c2408));
                }
                scr.fill(x - 7, y - 6, 14, 16, c(0x4c2c0c));
                scr.fill(x - 6, y - 6, 3, 16, c(0x6c4418));
                scr.line(x - 7, y - 2, x - 16, y - 8 + sway as i32, c(0x4c2c0c));
                scr.line(x + 7, y - 2, x + 16, y - 8 - sway as i32, c(0x4c2c0c));
                scr.disc(x, y - 14, 10, c(0x0c4c14));
                scr.disc(x - 3, y - 17, 5, c(0x2c7c1c));
                for &(ax, ay) in &[(-5, -16), (4, -18), (6, -11)] {
                    scr.disc(x + ax, y + ay, 1, c(0xd82800));
                }
                scr.fill(x - 4, y - 2, 3, 2, c(0xfce040));
                scr.fill(x + 1, y - 2, 3, 2, c(0xfce040));
                scr.fill(x - 3, y + 3, 6, 3, c(0x100804));
            }
            EK::Zombie => {
                let s = &self.spr.zombie;
                let img = if white { &s.white } else if e.st.frozen() || e.st.chilled() { &s.frost } else { &s.img };
                scr.blend_ellipse(x, y + 7, 5, 2, BLACK, 0.3);
                let sway = if (t / 20) % 2 == 0 { -1.0 } else { 1.0 };
                scr.spr(img, e.x + sway * 0.5, e.y, e.x > self.pl.x);
            }
            EK::GraveLord => {
                let bob = ((t as f32) * 0.07).sin() * 2.0;
                let y = y + bob as i32;
                scr.blend_ellipse(x, y + 14, 8, 2, BLACK, 0.3);
                for row in 0..20 {
                    let w = 3 + row / 3;
                    let col = if row > 16 { 0x301040 } else { 0x201028 };
                    scr.fill(x - w, y - 4 + row, w * 2, 1, c(col));
                }
                for i in 0..5 {
                    scr.fill(x - 7 + i * 3, y + 16, 2, 2 + (i % 2), c(0x201028));
                }
                scr.disc(x, y - 9, 5, c(0xd8d8c8));
                scr.fill(x - 3, y - 10, 2, 2, c(0x9818a8));
                scr.fill(x + 1, y - 10, 2, 2, c(0x9818a8));
                scr.blend_disc(x, y - 10, 6, rgb(0xb040fc), 0.25);
                scr.fill(x - 5, y - 16, 10, 3, c(0xfcbc3c));
                for i in 0..3 {
                    scr.fill(x - 5 + i * 4, y - 18, 2, 2, c(0xfcbc3c));
                }
                scr.line(x + 9, y - 14, x + 9, y + 14, c(0x5c3410));
                scr.disc(x + 9, y - 16, 2, c(0xb040fc));
            }
            _ => return false,
        }
        if e.st.burning() {
            draw_flames(scr, e.x, e.y, e.w, e.h, e.st.flame_scale(), self.frame, e.id);
        }
        true
    }
}
