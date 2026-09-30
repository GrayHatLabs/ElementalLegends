//! Casting, projectiles, damage, elemental status application, enemy AI and particles.
use super::status::{IceHit, FREEZE_TIME};
use super::*;

const FIRE_COLS: [u32; 4] = [rgb(0xfce040), rgb(0xfc9838), rgb(0xd82800), rgb(0xfcfcfc)];
const ICE_COLS: [u32; 3] = [WHITE, rgb(0xa4e4fc), rgb(0x78c8ec)];

impl Game {
    // ------------------------------------------------------------ particles
    pub(super) fn part(&mut self, x: f32, y: f32, vx: f32, vy: f32, life: i32, c: u32, sz: i32, kind: PK) {
        if self.parts.len() > 700 {
            return; // keep the handheld happy during big fights
        }
        self.parts.push(Part { x, y, vx, vy, life, max: life, c, sz, kind });
    }
    pub(super) fn boom(&mut self, x: f32, y: f32, n: usize, el: Elem) {
        let cols = if el == Elem::Neutral {
            [WHITE, rgb(0xfcbc3c), rgb(0xfc7460), rgb(0xd82800)]
        } else {
            [WHITE, el.light(), el.main(), el.light()]
        };
        for _ in 0..n {
            let a = self.rng.range(0.0, PI * 2.0);
            let s = self.rng.range(0.4, 2.6);
            let life = self.rng.irange(14, 30);
            let c = self.rng.pick(&cols);
            let sz = self.rng.pick(&[1, 2, 2, 3]);
            self.part(x, y, a.cos() * s, a.sin() * s, life, c, sz, PK::Dot);
        }
        self.part(x, y, 0.0, 0.0, 12, WHITE, 1, PK::Ring);
    }
    pub(super) fn spark(&mut self, x: f32, y: f32, c: u32) {
        for _ in 0..3 {
            let (vx, vy) = (self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0));
            self.part(x, y, vx, vy, 8, c, 1, PK::Dot);
        }
    }
    pub(super) fn embers(&mut self, x: f32, y: f32, n: usize, spread: f32) {
        for _ in 0..n {
            let (vx, vy) = (self.rng.range(-spread, spread), self.rng.range(-1.6, -0.3));
            let life = self.rng.irange(12, 26);
            let c = self.rng.pick(&FIRE_COLS[..3]);
            self.part(x, y, vx, vy, life, c, 1, PK::Ember);
        }
    }
    /// Ice breaking apart: fragments fly outward and fall, plus a crystal flash.
    pub(super) fn shatter(&mut self, x: f32, y: f32, w: f32, h: f32) {
        for i in 0..16 {
            let ox = self.rng.range(-w / 2.0 - 2.0, w / 2.0 + 2.0);
            let oy = self.rng.range(-h / 2.0 - 2.0, h / 2.0 + 2.0);
            let (vx, vy) = (ox * 0.12 + self.rng.range(-0.6, 0.6), self.rng.range(-2.4, -0.6));
            let c = ICE_COLS[i % 3];
            let life = self.rng.irange(24, 38);
            self.part(x + ox, y + oy, vx, vy, life, c, if i % 3 == 0 { 3 } else { 2 }, PK::Shard);
        }
        self.part(x, y, 0.0, 0.0, 14, rgb(0xa4e4fc), 1, PK::Crystal);
        self.part(x, y, 0.0, 0.0, 10, rgb(0xa4e4fc), 1, PK::Glow(w.max(h)));
        self.sfx(Sfx::Shatter);
    }
    /// Short elemental impact animation where a bolt lands.
    pub(super) fn impact(&mut self, x: f32, y: f32, el: Elem) {
        match el {
            Elem::Fire => {
                self.part(x, y, 0.0, 0.0, 14, rgb(0xfc9838), 1, PK::Burst(Elem::Fire, 10.0));
                self.embers(x, y, 8, 1.2);
            }
            Elem::Ice => {
                self.part(x, y, 0.0, 0.0, 12, rgb(0xa4e4fc), 1, PK::Crystal);
                for _ in 0..5 {
                    let (vx, vy) = (self.rng.range(-1.4, 1.4), self.rng.range(-1.8, -0.2));
                    let c = self.rng.pick(&ICE_COLS);
                    self.part(x, y, vx, vy, 20, c, 1, PK::Shard);
                }
            }
            Elem::Storm => {
                self.part(x, y, 0.0, 0.0, 8, rgb(0xfce040), 1, PK::Burst(Elem::Storm, 7.0));
                self.spark(x, y, WHITE);
                self.spark(x, y, rgb(0xfce040));
            }
            _ => {
                self.part(x, y, 0.0, 0.0, 10, rgb(0x98d858), 1, PK::Burst(Elem::Earth, 8.0));
                for _ in 0..6 {
                    let (vx, vy) = (self.rng.range(-1.5, 1.5), self.rng.range(-2.0, -0.5));
                    let c = self.rng.pick(&[rgb(0x8c5020), rgb(0x5c3c10), rgb(0x98d858)]);
                    self.part(x, y, vx, vy, 18, c, 2, PK::Shard);
                }
            }
        }
    }
    pub(super) fn float(&mut self, s: impl Into<String>, x: f32, y: f32, c: u32) {
        self.parts.push(Part { x, y, vx: 0.0, vy: -0.4, life: 50, max: 50, c, sz: 1, kind: PK::Text(s.into()) });
    }
    pub(super) fn show_msg(&mut self, s: impl Into<String>) {
        self.msg = Some((s.into(), 200));
    }
    pub(super) fn update_parts(&mut self) {
        for p in self.parts.iter_mut() {
            p.x += p.vx;
            p.y += p.vy;
            match p.kind {
                PK::Dot => {
                    p.vx *= 0.95;
                    p.vy *= 0.95;
                }
                PK::Shard => {
                    p.vy += 0.15;
                    p.vx *= 0.98;
                }
                PK::Ember => {
                    p.vy -= 0.02;
                    p.vx *= 0.94;
                }
                _ => {}
            }
            p.life -= 1;
        }
        self.parts.retain(|p| p.life > 0);
    }
    pub(super) fn update_stars(&mut self, sp: f32) {
        for i in 0..self.stars.len() {
            self.stars[i].y += self.stars[i].s * sp;
            if self.stars[i].y > HF {
                self.stars[i].y -= HF;
                self.stars[i].x = self.rng.range(0.0, WF);
            }
        }
    }

    // ------------------------------------------------------------ casting
    /// How far a bolt flies: magic power-ups extend it (full screen at level 3),
    /// and it shrinks as the mage's health runs low (down to about half).
    pub(super) fn bolt_range(&self) -> f32 {
        let base = match self.s.spell_lv {
            1 => 80.0,
            2 => 120.0,
            _ => 190.0,
        };
        let health = (self.s.hp.max(0) as f32 / self.s.max_hp.max(1) as f32).clamp(0.0, 1.0);
        base * (0.45 + 0.55 * health)
    }
    pub(super) fn cast_bolt(&mut self) {
        let el = self.el();
        let (spd, dmg, r, cd, _) = bolt_stats(el);
        let life = ((self.bolt_range() / spd).round() as i32).max(8);
        let p = self.pl;
        let a = p.fy.atan2(p.fx);
        // A single bolt until the end stages: twin bolts arrive with the 4th lair's power-up.
        let spread: &[f32] = if self.s.spell_lv >= 3 { &[-0.12, 0.12] } else { &[0.0] };
        for &da in spread {
            let b = a + da;
            self.pb.push(Bullet {
                x: p.x + p.fx * 6.0,
                y: p.y - 2.0 + p.fy * 6.0,
                vx: b.cos() * spd,
                vy: b.sin() * spd,
                r,
                dmg,
                el,
                life,
                age: 0,
                style: Shot::Orb,
                pierce: if el == Elem::Storm { Some(vec![]) } else { None },
                dead: false,
            });
        }
        // Muzzle flash at the staff.
        let (sx, sy) = (p.x + p.fx * 8.0, p.y - 3.0 + p.fy * 8.0);
        self.part(sx, sy, 0.0, 0.0, 6, el.light(), 1, PK::Glow(5.0));
        self.pl.cd = cd;
        self.pl.cast = 6;
        self.sfx(Sfx::Shoot);
    }
    pub(super) fn cast_spell(&mut self) {
        let el = self.el();
        let cost = spell_cost(el);
        let (px, py) = (self.pl.x, self.pl.y);
        if self.s.mp < cost {
            if self.p(Btn::Sub) {
                self.sfx(Sfx::Deny);
                self.float("NO MANA", px - 28.0, py - 18.0, rgb(0x3cbcfc));
            }
            return;
        }
        self.s.mp -= cost;
        self.pl.scd = 40;
        self.pl.cast = 12;
        match el {
            Elem::Fire => {
                for i in 0..12 {
                    let a = i as f32 * PI / 6.0;
                    self.pb.push(Bullet {
                        x: px, y: py - 2.0, vx: a.cos() * 3.0, vy: a.sin() * 3.0, r: 4.0, dmg: 2.5, el, life: 60,
                        age: 0, style: Shot::Orb, pierce: None, dead: false,
                    });
                }
                self.part(px, py, 0.0, 0.0, 16, rgb(0xfc9838), 1, PK::Burst(Elem::Fire, 22.0));
                self.flash = 6;
                self.sfx(Sfx::Spell);
            }
            Elem::Ice => {
                let mut en = std::mem::take(&mut self.enemies);
                for e in en.iter_mut() {
                    if !e.dead && e.active() {
                        self.damage_enemy(e, 2.0, el);
                        if !e.dead {
                            e.st.freeze_now(FREEZE_TIME);
                        }
                    }
                }
                en.append(&mut self.enemies);
                self.enemies = en;
                self.boss_status(Elem::Ice);
                self.boss_status(Elem::Ice);
                self.boss_hit(3.0, el, px, py, 0.0, 0.0);
                self.frost_water_near(px, py, 60.0);
                for r in 0..3 {
                    self.part(px, py, 0.0, 0.0, 14 + r * 6, el.light(), 1, PK::Ring);
                }
                self.part(px, py, 0.0, 0.0, 16, rgb(0xa4e4fc), 1, PK::Crystal);
                self.flash = 8;
                self.sfx(Sfx::Freeze);
            }
            Elem::Storm => {
                let mut en = std::mem::take(&mut self.enemies);
                let (mut cx, mut cy) = (px, py);
                let mut hitlist: Vec<u32> = vec![];
                for n in 0..5 {
                    let range = if n == 0 { 170.0 } else { 110.0 };
                    let mut best: Option<usize> = None;
                    let mut bd = range;
                    for (i, e) in en.iter().enumerate() {
                        if e.dead || !e.active() || hitlist.contains(&e.id) {
                            continue;
                        }
                        let d = dist(cx, cy, e.x, e.y);
                        if d < bd {
                            bd = d;
                            best = Some(i);
                        }
                    }
                    let Some(i) = best else { break };
                    let (ex, ey) = (en[i].x, en[i].y);
                    hitlist.push(en[i].id);
                    self.part(cx, cy, 0.0, 0.0, 12, el.light(), 1, PK::Line(ex, ey));
                    self.damage_enemy(&mut en[i], 4.0, el);
                    cx = ex;
                    cy = ey;
                }
                en.append(&mut self.enemies);
                self.enemies = en;
                if let Some((bx, by, _, _)) = self.boss_box() {
                    self.part(px, py, 0.0, 0.0, 12, el.light(), 1, PK::Line(bx, by));
                    self.boss_hit(5.0, el, px, py, 0.0, 0.0);
                }
                // Chain Bolt also leaps into nearby graves and raises the dead.
                self.chain_graves(px, py);
                self.flash = 4;
                self.sfx(Sfx::Zap);
            }
            _ => {
                let mut en = std::mem::take(&mut self.enemies);
                for e in en.iter_mut() {
                    if !e.dead && e.active() {
                        if e.st.break_freeze() {
                            let (x, y, w, h) = (e.x, e.y, e.w, e.h);
                            self.shatter(x, y, w, h);
                            self.damage_enemy(e, 2.0, el);
                        }
                        e.st.stun(50);
                        self.damage_enemy(e, 4.5, el);
                    }
                }
                en.append(&mut self.enemies);
                self.enemies = en;
                self.boss_hit(7.0, el, px, py, 0.0, 0.0);
                self.quake_cracks();
                for _ in 0..24 {
                    let (x, y) = (self.rng.range(self.cam.0 + 8.0, self.cam.0 + VIEW_W - 8.0), self.rng.range(self.cam.1 + 8.0, self.cam.1 + VIEW_H - 8.0));
                    let c = self.rng.pick(&[rgb(0x8c5020), rgb(0x98d858), rgb(0x5c3c10)]);
                    self.part(x, y, 0.0, -1.0, 20, c, 3, PK::Dot);
                }
                self.shake = 24;
                self.sfx(Sfx::Quake);
            }
        }
    }

    // ------------------------------------------------------------ damage
    pub(super) fn damage_enemy(&mut self, e: &mut Enemy, dmg: f32, el: Elem) {
        if e.dead {
            return;
        }
        // Encounter creatures may react instead of taking damage (the Hoard Dragon can't be hurt).
        if !self.mini_hit(e, el) {
            return;
        }
        let m = enemy_mult(e, el);
        e.hp -= dmg * m;
        e.flash = 5;
        if e.tip <= 0 && m != 1.0 {
            e.tip = 30;
            if m > 1.0 {
                self.float("WEAK!", e.x - 20.0, e.y - 16.0, rgb(0xfce040));
            } else {
                self.float("RESIST", e.x - 24.0, e.y - 16.0, rgb(0x747474));
            }
        }
        if e.hp <= 0.0 {
            self.kill_enemy(e);
        } else {
            self.sfx(if m > 1.5 { Sfx::Weak } else { Sfx::Hit });
        }
    }
    fn kill_enemy(&mut self, e: &mut Enemy) {
        e.dead = true;
        if e.st.frozen() {
            self.shatter(e.x, e.y, e.w, e.h);
        }
        self.boom(e.x, e.y, 12, e.el);
        if e.st.burning() {
            self.embers(e.x, e.y, 10, 1.4);
        }
        self.sfx(Sfx::Boom);
        self.drop_from(e);
        if e.mini != 0 {
            self.mini_defeated(e);
        }
    }
    /// Burn damage-over-time tick: fire multipliers apply, but no knockback or hit sound.
    fn burn_tick(&mut self, e: &mut Enemy, dmg: f32) {
        if e.k == EK::HoardDragon {
            return;
        }
        e.hp -= dmg * enemy_mult(e, Elem::Fire);
        e.flash = e.flash.max(2);
        self.embers(e.x, e.y - e.h / 2.0, 3, 0.8);
        if e.hp <= 0.0 && !e.dead {
            self.kill_enemy(e);
        }
    }
    /// Elemental side effects of a bolt hit.
    fn status_hit(&mut self, e: &mut Enemy, el: Elem, vx: f32, vy: f32) {
        if e.dead {
            return;
        }
        let l = vx.hypot(vy).max(0.01);
        let (ux, uy) = (vx / l, vy / l);
        // Knockback, as in A Link to the Past: about a tile, heavy monsters budge less.
        let mut kb = 3.0;
        match el {
            Elem::Fire => {
                if e.st.break_freeze() {
                    let (x, y, w, h) = (e.x, e.y, e.w, e.h);
                    self.shatter(x, y, w, h);
                }
                if e.st.ignite() {
                    self.sfx(Sfx::Ignite);
                    self.embers(e.x, e.y, 6, 1.0);
                }
            }
            Elem::Ice => match e.st.ice_hit(false) {
                IceHit::Froze => {
                    self.float("FROZEN!", e.x - 28.0, e.y - 18.0, rgb(0xa4e4fc));
                    self.part(e.x, e.y, 0.0, 0.0, 14, rgb(0xa4e4fc), 1, PK::Crystal);
                    self.sfx(Sfx::Freeze);
                    kb = 0.0;
                }
                IceHit::Chilled => {
                    if e.tip <= 0 {
                        e.tip = 20;
                        self.float("CHILLED", e.x - 28.0, e.y - 18.0, rgb(0x78c8ec));
                    }
                }
                IceHit::NoEffect => kb = 0.0,
            },
            Elem::Earth => {
                if e.st.break_freeze() {
                    let (x, y, w, h) = (e.x, e.y, e.w, e.h);
                    self.shatter(x, y, w, h);
                    self.damage_enemy(e, 1.0, el);
                }
                kb = 4.5;
            }
            _ => kb = 2.2,
        }
        let kb = kb * kb_weight(e.k);
        if !e.st.frozen() && kb > 0.0 {
            e.kbx = ux * kb;
            e.kby = uy * kb;
            // A brief stagger so it can't walk straight back in.
            e.st.stun(10);
        }
    }

    // ------------------------------------------------------------ bullets
    pub(super) fn eshot(&mut self, x: f32, y: f32, a: f32, s: f32, el: Elem) {
        self.eshot_x(x, y, a, s, el, Shot::Orb, 2.0, 2.0, 500);
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn eshot_x(&mut self, x: f32, y: f32, a: f32, s: f32, el: Elem, style: Shot, r: f32, dmg: f32, life: i32) {
        self.eb.push(Bullet {
            x, y, vx: a.cos() * s, vy: a.sin() * s, r, dmg, el, life, age: 0, style, pierce: None, dead: false,
        });
    }
    /// Per-frame projectile trail particles.
    fn bolt_trail(&mut self, b: &Bullet) {
        match b.el {
            Elem::Fire => {
                let c = self.rng.pick(&FIRE_COLS[..3]);
                self.part(b.x, b.y, 0.0, 0.0, 9, c, 2, PK::Ember);
                if b.age % 2 == 0 {
                    self.embers(b.x, b.y, 1, 0.6);
                }
            }
            Elem::Ice => {
                if b.age % 2 == 0 {
                    let (ox, oy) = (self.rng.range(-2.0, 2.0), self.rng.range(-2.0, 2.0));
                    let c = self.rng.pick(&ICE_COLS);
                    self.part(b.x + ox, b.y + oy, 0.0, 0.2, 12, c, 1, PK::Dot);
                }
            }
            Elem::Storm => {
                if b.age % 3 == 0 {
                    self.part(b.x, b.y, 0.0, 0.0, 5, rgb(0xfce040), 1, PK::Dot);
                }
            }
            _ => {
                if b.age % 4 == 0 {
                    self.part(b.x, b.y, 0.0, 0.3, 10, rgb(0x5c3c10), 1, PK::Dot);
                }
            }
        }
    }
    fn bolt_impact(&mut self, b: &Bullet, en: &mut [Enemy]) {
        self.impact(b.x, b.y, b.el);
        if b.el == Elem::Fire {
            for e in en.iter_mut() {
                if !e.dead && e.active() && dist(b.x, b.y, e.x, e.y) < 16.0 {
                    self.damage_enemy(e, 0.5, Elem::Fire);
                }
            }
        }
    }
    pub(super) fn update_pbullets(&mut self) {
        let (rw, rh) = (self.room_wf(), self.room_hf());
        let mut pb = std::mem::take(&mut self.pb);
        let mut en = std::mem::take(&mut self.enemies);
        for b in pb.iter_mut() {
            b.x += b.vx;
            b.y += b.vy;
            b.life -= 1;
            b.age += 1;
            self.bolt_trail(b);
            if b.life <= 0 || b.x < -8.0 || b.x > rw + 8.0 || b.y < HUDF - 8.0 || b.y > rh + 8.0 {
                if b.life <= 0 {
                    self.impact(b.x, b.y, b.el);
                }
                b.dead = true;
                continue;
            }
            let (c, r) = tile_of(b.x, b.y);
            if b.el == Elem::Ice && self.tile_at(c, r) == T_WATER {
                self.freeze_water(c, r);
            }
            if self.shot_blocked(b.x, b.y) {
                b.dead = true;
                self.dungeon_bolt_hit(c, r, b.el);
                self.world_bolt_hit(c, r, b.el);
                self.bolt_impact(b, &mut en);
                continue;
            }
            for i in 0..en.len() {
                let e = &en[i];
                if e.dead || !e.active() || !hit(b.x, b.y, b.r * 2.0, b.r * 2.0, e.x, e.y, e.w, e.h) {
                    continue;
                }
                if let Some(pl) = b.pierce.as_mut() {
                    if pl.contains(&e.id) {
                        continue;
                    }
                    pl.push(e.id);
                } else {
                    b.dead = true;
                }
                let mut e = en[i].clone();
                self.damage_enemy(&mut e, b.dmg, b.el);
                self.status_hit(&mut e, b.el, b.vx, b.vy);
                en[i] = e;
                if b.dead {
                    break;
                }
            }
            if b.dead {
                self.bolt_impact(b, &mut en);
                continue;
            }
            if let Some((bx, by, bw, bh)) = self.boss_box() {
                if hit(b.x, b.y, b.r * 2.0, b.r * 2.0, bx, by, bw, bh) {
                    let already = b.pierce.as_ref().map_or(false, |p| p.contains(&u32::MAX));
                    if !already {
                        match b.pierce.as_mut() {
                            Some(p) => p.push(u32::MAX),
                            None => b.dead = true,
                        }
                        if self.boss_hit(b.dmg, b.el, b.x, b.y, b.vx, b.vy) {
                            self.boss_status(b.el);
                        }
                        self.impact(b.x, b.y, b.el);
                    }
                }
            }
        }
        pb.retain(|b| !b.dead);
        pb.append(&mut self.pb);
        self.pb = pb;
        en.append(&mut self.enemies);
        self.enemies = en;
    }
    pub(super) fn update_ebullets(&mut self) {
        let (px, py) = (self.pl.x, self.pl.y);
        let (rw, rh) = (self.room_wf(), self.room_hf());
        let mut eb = std::mem::take(&mut self.eb);
        for b in eb.iter_mut() {
            b.x += b.vx;
            b.y += b.vy;
            b.life -= 1;
            b.age += 1;
            if b.style == Shot::Flame && b.age % 3 == 0 {
                self.embers(b.x, b.y, 1, 0.4);
            }
            if b.life <= 0 || b.x < -8.0 || b.x > rw + 8.0 || b.y < HUDF - 8.0 || b.y > rh + 8.0 || self.shot_blocked(b.x, b.y) {
                if b.style == Shot::Boulder || b.style == Shot::Fireball {
                    let el = if b.style == Shot::Boulder { Elem::Earth } else { Elem::Fire };
                    self.impact(b.x, b.y, el);
                }
                b.dead = true;
                continue;
            }
            if self.mode == Mode::Play && self.pl.inv <= 0 && hit(b.x, b.y, b.r * 2.0, b.r * 2.0, px, py, 6.0, 6.0) {
                b.dead = true;
                self.hurt_from(b.dmg as i32, b.x - b.vx * 4.0, b.y - b.vy * 4.0);
            }
        }
        eb.retain(|b| !b.dead);
        eb.append(&mut self.eb);
        self.eb = eb;
    }

    // ------------------------------------------------------------ enemies
    fn wander(&mut self, e: &mut Enemy) {
        e.turn -= 1;
        if e.turn <= 0 {
            let a = self.rng.irange(0, 7) as f32 * PI / 4.0;
            e.vx = a.cos() * e.spd;
            e.vy = a.sin() * e.spd;
            e.turn = self.rng.irange(50, 130);
        }
    }
    fn upd_enemy(&mut self, e: &mut Enemy, counts: &[(u32, usize)]) {
        e.t += 1;
        if e.flash > 0 {
            e.flash -= 1;
        }
        if e.tip > 0 {
            e.tip -= 1;
        }
        if e.spawn > 0 {
            e.spawn -= 1;
            return;
        }
        // Status effects: burn ticks, thaw shatter, and ambient flames / frost.
        let tk = e.st.tick();
        if tk.burn_dmg > 0.0 {
            self.burn_tick(e, tk.burn_dmg);
            if e.dead {
                return;
            }
        }
        if tk.thawed {
            let (x, y, w, h) = (e.x, e.y, e.w, e.h);
            self.shatter(x, y, w, h);
        }
        if e.st.burning() && self.frame % 4 == 0 {
            let ox = self.rng.range(-e.w / 2.0, e.w / 2.0);
            self.embers(e.x + ox, e.y - e.h / 2.0, 1, 0.5);
        }
        if e.st.chilled() && self.frame % 10 == 0 {
            let ox = self.rng.range(-e.w / 2.0, e.w / 2.0);
            self.part(e.x + ox, e.y - e.h / 2.0, 0.0, 0.4, 16, WHITE, 1, PK::Dot);
        }
        let flying = matches!(e.k, EK::Bat | EK::Ghost);
        if e.kbx.abs() + e.kby.abs() > 0.05 {
            if flying {
                e.x += e.kbx;
                e.y += e.kby;
            } else {
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, e.kbx, e.kby);
            }
            e.kbx *= 0.75;
            e.kby *= 0.75;
        }
        if e.st.immobile() {
            return;
        }
        let f = e.st.speed();
        let (px, py) = (self.pl.x, self.pl.y);
        let d = dist(e.x, e.y, px, py);
        let a = ang(e.x, e.y, px, py);
        // Chilled enemies also attack less often.
        let fire_now = |t: i32, rate: i32| {
            let r = if f < 1.0 { rate * 2 } else { rate };
            t % r == 0
        };
        match e.k {
            EK::Slime => {
                self.wander(e);
                let (hx, hy) = self.move_box(&mut e.x, &mut e.y, e.w, e.h, e.vx * f, e.vy * f);
                if hx {
                    e.vx = -e.vx;
                }
                if hy {
                    e.vy = -e.vy;
                }
            }
            EK::Bat => {
                if e.t % 20 == 0 {
                    let b = a + self.rng.range(-1.3, 1.3);
                    e.vx = b.cos() * e.spd;
                    e.vy = b.sin() * e.spd;
                }
                e.x += e.vx * f;
                e.y += e.vy * f;
            }
            EK::Skeleton => {
                if d < 130.0 {
                    e.vx = a.cos() * e.spd;
                    e.vy = a.sin() * e.spd;
                } else {
                    self.wander(e);
                }
                let (hx, hy) = self.move_box(&mut e.x, &mut e.y, e.w, e.h, e.vx * f, e.vy * f);
                if hx || hy {
                    e.turn = 0;
                }
                if e.lvl >= 1 && fire_now(e.t, e.rate) && d < 150.0 {
                    self.eshot(e.x, e.y, a, 1.6, Elem::Neutral);
                }
            }
            EK::Imp => {
                let (mx, my) = if d < 60.0 {
                    (-a.cos(), -a.sin())
                } else if d > 100.0 {
                    (a.cos(), a.sin())
                } else {
                    ((a + PI / 2.0).cos() * 0.6, (a + PI / 2.0).sin() * 0.6)
                };
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, mx * e.spd * f, my * e.spd * f);
                if fire_now(e.t, e.rate) {
                    self.eshot(e.x, e.y, a, 1.7, Elem::Fire);
                }
            }
            EK::Ghost => {
                e.vx += a.cos() * 0.03;
                e.vy += a.sin() * 0.03;
                let s = e.vx.hypot(e.vy);
                if s > e.spd {
                    e.vx *= e.spd / s;
                    e.vy *= e.spd / s;
                }
                e.x += e.vx * f;
                e.y += e.vy * f;
                if e.lvl >= 3 && fire_now(e.t, 170) {
                    self.eshot(e.x, e.y, a, 1.4, Elem::Ice);
                }
            }
            EK::Golem => {
                e.vx = a.cos() * e.spd;
                e.vy = a.sin() * e.spd;
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, e.vx * f, e.vy * f);
            }
            EK::HoardDragon | EK::Dryad | EK::Treant | EK::Zombie | EK::GraveLord => self.upd_mini_enemy(e, f),
            EK::Generator => {
                let alive = counts.iter().find(|c| c.0 == e.id).map_or(0, |c| c.1);
                if e.t % e.rate == 0 && alive < 4 && self.in_lair == 0 {
                    let th = self.cur_room().theme.min(3);
                    let k = self.rng.pick(POOLS[th]);
                    let (ox, oy) = (self.rng.range(-6.0, 6.0), self.rng.range(-6.0, 6.0));
                    let mut c = self.make_enemy(k, e.x + ox, e.y + oy, th);
                    c.spawn = 20;
                    c.gen_id = e.id;
                    self.enemies.push(c);
                }
            }
        }
        e.x = e.x.clamp(10.0, self.room_wf() - 10.0);
        e.y = e.y.clamp(HUDF + 10.0, self.room_hf() - 10.0);
    }
    pub(super) fn update_enemies(&mut self) {
        let mut en = std::mem::take(&mut self.enemies);
        if self.boss_dead {
            for e in en.iter() {
                let (x, y, el) = (e.x, e.y, e.el);
                self.boom(x, y, 8, el);
            }
            en.clear();
        }
        let counts: Vec<(u32, usize)> = en
            .iter()
            .filter(|e| e.k == EK::Generator)
            .map(|g| (g.id, en.iter().filter(|c| c.gen_id == g.id && !c.dead).count()))
            .collect();
        for e in en.iter_mut() {
            self.upd_enemy(e, &counts);
            let p = self.pl;
            // Frozen enemies are harmless blocks of ice.
            if !e.dead && e.active() && e.touch > 0 && !e.st.frozen() && hit(e.x, e.y, e.w, e.h, p.x, p.y, p.w, p.h) {
                let was = self.pl.inv;
                self.hurt_from(e.touch, e.x, e.y);
                if was <= 0 && self.pl.inv > 0 {
                    // The monster bounces off too and staggers, so it can't cling to the mage.
                    let (dx, dy) = (e.x - p.x, e.y - p.y);
                    let l = dx.hypot(dy).max(0.01);
                    let w = kb_weight(e.k);
                    e.kbx = dx / l * 2.5 * w;
                    e.kby = dy / l * 2.5 * w;
                    e.st.stun(24);
                }
            }
        }
        en.retain(|e| !e.dead);
        en.append(&mut self.enemies);
        self.enemies = en;
    }
}

/// How far a monster is pushed by hits (1 = a normal monster).
fn kb_weight(k: EK) -> f32 {
    match k {
        EK::Generator | EK::HoardDragon => 0.0,
        EK::Golem => 0.4,
        EK::Zombie => 0.7,
        EK::Dryad | EK::Treant | EK::GraveLord => 0.25,
        _ => 1.0,
    }
}
