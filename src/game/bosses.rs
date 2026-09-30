//! The six fantasy bosses. Each is drawn procedurally in the game's pixel style
//! and runs a telegraph -> attack -> recover state machine with two or three phases.
use super::status::IceHit;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BKind {
    Treant,
    Guardian,
    Golem,
    Dragon,
    Sorcerer,
    DarkSorcerer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Atk {
    None,
    Roots,
    Branch,
    Shock,
    Summon,
    Breath,
    Fly,
    Tail,
    Fireball,
    Slash,
    Guard,
    Raise,
    DarkOrbs,
    Slam,
    Throw,
    Teleport,
    Bolts,
    Circles,
    Shield,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BState {
    Intro,
    Idle,
    Tele,
    Act,
    Recover,
}

/// Ground danger zones: warned first, then active.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum HK {
    Root,
    Slam,
    Magic,
    Rock,
    Sweep,
    Tail,
}

#[derive(Clone, Debug)]
pub(super) struct Hazard {
    pub k: HK,
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub w: f32,
    pub h: f32,
    pub vx: f32,
    pub warn: i32,
    pub warn_max: i32,
    pub act: i32,
    pub dmg: i32,
    pub track: bool,
}
impl Hazard {
    pub(super) fn circle(k: HK, x: f32, y: f32, r: f32, warn: i32, act: i32, dmg: i32) -> Self {
        Hazard { k, x, y, r, w: 0.0, h: 0.0, vx: 0.0, warn, warn_max: warn.max(1), act, dmg, track: false }
    }
    pub(super) fn hits(&self, px: f32, py: f32) -> bool {
        if self.w > 0.0 {
            hit(self.x, self.y, self.w, self.h, px, py, 8.0, 10.0)
        } else {
            dist(self.x, self.y, px, py) < self.r + 4.0
        }
    }
}

pub(super) struct Boss {
    pub kind: BKind,
    pub name: &'static str,
    pub n: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub hp: f32,
    pub max: f32,
    pub t: i32,
    pub st: Status,
    pub flash: i32,
    pub tip: i32,
    pub alive: bool,
    pub gone: bool,
    pub weak: Elem,
    pub resist: Elem,
    pub el: Elem,
    pub phase: u8,
    pub state: BState,
    pub st_t: i32,
    pub st_max: i32,
    pub atk: Atk,
    pub seq: usize,
    pub cool: i32,
    pub face: f32,
    pub tx: f32,
    pub ty: f32,
    pub ax: f32,
    pub ay: f32,
    pub shield: i32,
    pub shield_t: i32,
    pub armor: f32,
    pub armor_max: f32,
    pub hidden: bool,
    pub z: f32,
    pub guard: bool,
    pub stagger: i32,
    pub hurt_t: i32,
    pub moving: bool,
    pub hazards: Vec<Hazard>,
    pub intro: i32,
    pub bang: f32,
}

pub(super) fn boss_kind(n: usize) -> BKind {
    match n {
        1 => BKind::Treant,
        2 => BKind::Guardian,
        3 => BKind::Golem,
        4 => BKind::Dragon,
        5 => BKind::Sorcerer,
        _ => BKind::DarkSorcerer,
    }
}

pub(super) fn make_boss(n: usize) -> Boss {
    let kind = boss_kind(n);
    // (name, hp, weak, resist, attack element, w, h, y)
    let (name, hp, weak, resist, el, w, h, y) = match kind {
        BKind::Treant => ("ANCIENT TREANT", 110.0, Elem::Fire, Elem::Earth, Elem::Earth, 40.0, 50.0, 90.0),
        BKind::Guardian => ("UNDEAD GUARDIAN", 150.0, Elem::Storm, Elem::Ice, Elem::Ice, 18.0, 30.0, 96.0),
        BKind::Golem => ("STONE GOLEM", 150.0, Elem::Storm, Elem::Fire, Elem::Earth, 36.0, 40.0, 100.0),
        BKind::Dragon => ("CRIMSON DRAGON", 180.0, Elem::Ice, Elem::Fire, Elem::Fire, 46.0, 28.0, 94.0),
        BKind::Sorcerer => ("ARCANE SORCERER", 220.0, Elem::Earth, Elem::Storm, Elem::Storm, 16.0, 26.0, 92.0),
        BKind::DarkSorcerer => ("DARK SORCERER", 340.0, Elem::Ice, Elem::Fire, Elem::Fire, 20.0, 30.0, 92.0),
    };
    let armor = if kind == BKind::Golem { 50.0 } else { 0.0 };
    Boss {
        kind, name, n, x: 128.0, y, w, h, hp, max: hp, t: 0, st: Status::default(), flash: 0, tip: 0, alive: true,
        gone: false, weak, resist, el, phase: 1, state: BState::Intro, st_t: 0, st_max: 1, atk: Atk::None, seq: 0,
        cool: 40, face: 1.0, tx: 128.0, ty: y, ax: 128.0, ay: 180.0, shield: 0, shield_t: 0, armor,
        armor_max: armor.max(1.0), hidden: false, z: 0.0, guard: false, stagger: 0, hurt_t: 0, moving: false,
        hazards: vec![], intro: 0, bang: 0.0,
    }
}

fn attacks(kind: BKind, phase: u8) -> &'static [Atk] {
    use Atk::*;
    match (kind, phase) {
        (BKind::Treant, 1) => &[Roots, Shock, Branch, Summon],
        (BKind::Treant, _) => &[Roots, Branch, Shock, Roots, Summon, Branch],
        (BKind::Guardian, 1) => &[Slash, Guard, DarkOrbs, Slash, Raise],
        (BKind::Guardian, _) => &[Slash, DarkOrbs, Guard, Slash, Raise, DarkOrbs],
        (BKind::Golem, 1) => &[Throw, Slam, Shock, Throw],
        (BKind::Golem, _) => &[Slam, Throw, Shock, Slam, Throw],
        (BKind::Dragon, 1) => &[Fireball, Breath, Fly, Fireball, Tail],
        (BKind::Dragon, _) => &[Breath, Fly, Fireball, Breath, Tail, Fly],
        (_, 1) => &[Bolts, Teleport, Circles, Shield, Bolts, Teleport],
        (_, 2) => &[Circles, Bolts, Teleport, Summon, Shield, Bolts, Circles, Teleport],
        _ => &[Circles, Bolts, Summon, Teleport, Circles, Shield, Bolts, Teleport],
    }
}
fn tele_time(a: Atk) -> i32 {
    use Atk::*;
    match a {
        Roots => 40,
        Branch => 36,
        Shock => 28,
        Summon => 26,
        Breath => 40,
        Fly => 18,
        Tail => 22,
        Fireball => 22,
        Slash => 30,
        Guard => 8,
        Raise => 28,
        DarkOrbs => 24,
        Slam => 40,
        Throw => 28,
        Teleport => 18,
        Bolts => 20,
        Circles => 12,
        Shield => 14,
        None => 1,
    }
}
fn act_time(a: Atk) -> i32 {
    use Atk::*;
    match a {
        Roots => 30,
        Branch => 50,
        Breath => 60,
        Fly => 110,
        Slash => 24,
        Guard => 80,
        DarkOrbs => 20,
        Teleport => 40,
        Bolts => 36,
        Circles => 30,
        Tail => 12,
        _ => 10,
    }
}
fn recover_time(a: Atk) -> i32 {
    match a {
        Atk::Slash => 45,
        Atk::Slam => 30,
        Atk::Breath | Atk::Fly => 24,
        _ => 16,
    }
}
/// Bosses whose opening attacks deserve a "!" warning.
fn big_attack(a: Atk) -> bool {
    matches!(a, Atk::Roots | Atk::Branch | Atk::Breath | Atk::Fly | Atk::Slash | Atk::Slam | Atk::Circles | Atk::Tail)
}

impl Game {
    // ------------------------------------------------------------ setup
    pub(super) fn start_boss(&mut self, n: usize) {
        self.arena = Some(make_arena(&self.themes[dungeon::dungeon_theme(n)], (n - 1).min(3)));
        self.in_lair = n;
        self.dungeon = None;
        self.clear_entities();
        self.scroll = None;
        self.msg = None;
        self.paused = false;
        self.pl = Player::at(128.0, 200.0);
        self.pl.inv = 60;
        self.boss = Some(make_boss(n));
        self.boss_dead = false;
        self.clear_t = 0;
        self.mode = Mode::BossIntro;
        self.t = 0;
        self.fade = 40;
        self.pan_y = 0.0;
        self.follow_cam(true);
        self.play_song(Some(Song::Lair));
    }

    pub(super) fn boss_box(&self) -> Option<(f32, f32, f32, f32)> {
        self.boss
            .as_ref()
            .filter(|b| b.alive && !b.hidden && b.state != BState::Intro)
            .map(|b| (b.x, b.y - b.z, b.w, b.h))
    }

    // ------------------------------------------------------------ damage
    /// A hit from the player. Returns true if it landed (so status effects apply).
    pub(super) fn boss_hit(&mut self, dmg: f32, el: Elem, sx: f32, sy: f32, vx: f32, vy: f32) -> bool {
        let Some(mut b) = self.boss.take() else { return false };
        let r = self.hit_boss(&mut b, dmg, el, sx, sy, vx, vy);
        self.boss = Some(b);
        r
    }
    #[allow(clippy::too_many_arguments)]
    fn hit_boss(&mut self, b: &mut Boss, dmg: f32, el: Elem, sx: f32, _sy: f32, vx: f32, vy: f32) -> bool {
        if !b.alive || b.hidden || b.state == BState::Intro {
            return false;
        }
        let (bx, by) = (b.x, b.y - b.z);
        // Undead Guardian: the raised shield stops attacks from the side it faces.
        let projectile = vx != 0.0 || vy != 0.0;
        if b.guard && projectile && ((sx - b.x).signum() == b.face || (sx - b.x).abs() < 6.0) {
            if b.tip <= 0 {
                b.tip = 24;
                self.float("BLOCK", bx - 20.0, by - 30.0, rgb(0xbcbcbc));
            }
            self.spark(bx + b.face * 10.0, by, WHITE);
            self.sfx(Sfx::Click);
            return false;
        }
        // Sorcerer shields soak hits; their weakness breaks them three times faster.
        if b.shield > 0 {
            b.shield -= if el == b.weak { 3 } else { 1 };
            self.spark(bx, by, b.el.light());
            self.sfx(Sfx::Click);
            if b.shield <= 0 {
                b.shield = 0;
                self.float("SHIELD BROKEN!", bx - 56.0, by - 34.0, WHITE);
                self.part(bx, by, 0.0, 0.0, 16, b.el.light(), 1, PK::Crystal);
                self.part(bx, by, 0.0, 0.0, 14, b.el.light(), 1, PK::Glow(22.0));
                self.shatter(bx, by, 30.0, 30.0);
            }
            return false;
        }
        b.flash = 4;
        b.hurt_t = 6;
        // Stone Golem: armor takes the blows (double from Earth) until it breaks.
        if b.armor > 0.0 {
            b.armor -= dmg * if el == Elem::Earth { 2.0 } else { 1.0 };
            b.hp -= dmg * 0.25;
            for _ in 0..3 {
                let (vx2, vy2) = (self.rng.range(-1.5, 1.5), self.rng.range(-2.0, -0.5));
                self.part(bx, by - 6.0, vx2, vy2, 20, rgb(0x9c9c90), 2, PK::Shard);
            }
            if b.tip <= 0 {
                b.tip = 30;
                let s = if el == Elem::Earth { "CRACK!" } else { "ARMOR" };
                self.float(s, bx - 20.0, by - 34.0, rgb(0xbcbcbc));
            }
            self.sfx(Sfx::Hit);
            if b.armor <= 0.0 {
                b.armor = 0.0;
                self.show_msg("THE GOLEM'S ARMOR SHATTERS! STRIKE ITS GLOWING CORE!");
                for _ in 0..24 {
                    let (vx2, vy2) = (self.rng.range(-2.5, 2.5), self.rng.range(-3.0, -0.5));
                    self.part(bx, by, vx2, vy2, 34, rgb(0x9c9c90), 3, PK::Shard);
                }
                self.sfx(Sfx::Rumble);
                self.shake = 16;
            }
        } else {
            let mut m = if el == Elem::Neutral {
                1.0
            } else if el == b.weak {
                2.0
            } else if el == b.resist {
                0.5
            } else {
                1.0
            };
            if b.stagger > 0 {
                m *= 1.5;
            }
            b.hp -= dmg * m;
            if b.tip <= 0 && m != 1.0 {
                b.tip = 30;
                if m > 1.0 {
                    self.float("WEAK!", bx - 20.0, by - 40.0, rgb(0xfce040));
                } else {
                    self.float("RESIST", bx - 24.0, by - 40.0, rgb(0x747474));
                }
            }
            self.sfx(if m > 1.5 { Sfx::Weak } else { Sfx::Hit });
        }
        if b.hp <= 0.0 {
            self.kill_boss(b);
        }
        true
    }
    fn kill_boss(&mut self, b: &mut Boss) {
        b.hp = 0.0;
        b.alive = false;
        b.hazards.clear();
        b.guard = false;
        b.shield = 0;
        b.z = 0.0;
        b.hidden = false;
        self.boss_dead = true;
        self.clear_t = 0;
        self.eb.clear();
        self.play_song(None);
        self.sfx(Sfx::Roar);
        self.shake = 20;
    }
    /// Elemental side effects of a landed hit (bosses thaw fast and resist refreezing).
    pub(super) fn boss_status(&mut self, el: Elem) {
        let Some(mut b) = self.boss.take() else { return };
        if b.alive && !b.hidden && b.state != BState::Intro {
            let (bx, by) = (b.x, b.y - b.z);
            match el {
                Elem::Fire => {
                    if b.st.break_freeze() {
                        self.shatter(bx, by, b.w, b.h);
                    }
                    if b.st.ignite() {
                        self.sfx(Sfx::Ignite);
                    }
                }
                Elem::Ice => {
                    if b.st.ice_hit(true) == IceHit::Froze {
                        self.float("FROZEN!", bx - 28.0, by - 44.0, rgb(0xa4e4fc));
                        self.part(bx, by, 0.0, 0.0, 14, rgb(0xa4e4fc), 1, PK::Crystal);
                        self.sfx(Sfx::Freeze);
                    }
                }
                Elem::Earth => {
                    if b.st.break_freeze() {
                        self.shatter(bx, by, b.w, b.h);
                        self.hit_boss(&mut b, 2.0, Elem::Neutral, bx, by, 0.0, 0.0);
                    }
                }
                _ => {}
            }
        }
        self.boss = Some(b);
    }

    // ------------------------------------------------------------ update
    pub(super) fn update_boss(&mut self) {
        let Some(mut b) = self.boss.take() else { return };
        self.upd_boss(&mut b);
        self.boss = Some(b);
    }
    fn upd_boss(&mut self, b: &mut Boss) {
        b.t += 1;
        if b.flash > 0 {
            b.flash -= 1;
        }
        if b.tip > 0 {
            b.tip -= 1;
        }
        if b.hurt_t > 0 {
            b.hurt_t -= 1;
        }
        if !b.alive || b.state == BState::Intro {
            return;
        }
        // Statuses: burning damages over time, freezing halts everything but hazards already placed.
        let tk = b.st.tick();
        if tk.burn_dmg > 0.0 {
            let m = if b.weak == Elem::Fire { 2.0 } else if b.resist == Elem::Fire { 0.5 } else { 1.0 };
            if b.armor > 0.0 {
                b.armor = (b.armor - tk.burn_dmg).max(0.0);
            } else {
                b.hp -= tk.burn_dmg * m;
            }
            b.flash = b.flash.max(2);
            self.embers(b.x, b.y - b.z - b.h / 2.0, 4, 1.2);
            if b.hp <= 0.0 {
                self.kill_boss(b);
                return;
            }
        }
        if tk.thawed {
            self.shatter(b.x, b.y - b.z, b.w, b.h);
        }
        if b.st.burning() && self.frame % 3 == 0 {
            let ox = self.rng.range(-b.w / 2.0, b.w / 2.0);
            self.embers(b.x + ox, b.y - b.z - b.h / 2.0, 1, 0.6);
        }
        if b.shield_t > 0 {
            b.shield_t -= 1;
            if b.shield_t == 0 {
                b.shield = 0;
            }
        }
        if b.stagger > 0 {
            b.stagger -= 1;
        }
        self.update_hazards(b);
        if b.st.immobile() {
            return;
        }
        // Phase changes.
        let next_phase = match b.kind {
            BKind::Golem => {
                if b.armor <= 0.0 { 2 } else { 1 }
            }
            BKind::DarkSorcerer => {
                if b.hp < b.max * 0.33 {
                    3
                } else if b.hp < b.max * 0.66 {
                    2
                } else {
                    1
                }
            }
            _ => {
                if b.hp < b.max * 0.5 { 2 } else { 1 }
            }
        };
        if next_phase > b.phase {
            b.phase = next_phase;
            b.seq = 0;
            b.state = BState::Recover;
            b.st_t = 40;
            b.guard = false;
            b.flash = 16;
            self.shake = 16;
            self.sfx(Sfx::Roar);
            let s = match b.kind {
                BKind::Treant => "THE TREANT'S CORRUPTION SPREADS!",
                BKind::Guardian => "THE GUARDIAN'S EYES BURN WITH DARK FIRE!",
                BKind::Golem => "THE CORE BLAZES! THE GOLEM QUICKENS!",
                BKind::Dragon => "THE DRAGON ROARS IN FURY!",
                BKind::Sorcerer => "THE SORCERER UNLEASHES HIS FULL POWER!",
                BKind::DarkSorcerer => {
                    if b.phase == 3 {
                        "THE DARK SORCERER'S FINAL FORM!"
                    } else {
                        "DARKNESS GATHERS AROUND THE SORCERER!"
                    }
                }
            };
            self.show_msg(s);
        }
        let (px, py) = (self.pl.x, self.pl.y);
        if b.state != BState::Act || !matches!(b.atk, Atk::Slash | Atk::Fly) {
            b.face = if px < b.x { -1.0 } else { 1.0 };
        }
        let spd = b.st.speed() * if b.phase > 1 { 1.4 } else { 1.0 };
        self.boss_move(b, spd);
        match b.state {
            BState::Idle => {
                if b.st.chilled() && self.frame % 2 == 0 {
                    // chilled bosses think slower
                } else {
                    b.cool -= 1;
                }
                if b.cool <= 0 {
                    let list = attacks(b.kind, b.phase);
                    let mut a = list[b.seq % list.len()];
                    b.seq += 1;
                    if a == Atk::Tail && dist(b.x, b.y, px, py) > 70.0 {
                        a = Atk::Fireball;
                    }
                    if matches!(a, Atk::Summon | Atk::Raise) && self.enemies.len() >= 5 {
                        a = if matches!(b.kind, BKind::Guardian) { Atk::DarkOrbs } else { Atk::Bolts };
                    }
                    b.atk = a;
                    b.state = BState::Tele;
                    let tt = tele_time(a);
                    b.st_t = if b.phase > 1 { tt * 4 / 5 } else { tt };
                    b.st_max = b.st_t;
                    self.boss_tele_start(b);
                }
            }
            BState::Tele => {
                self.boss_tele_frame(b);
                b.st_t -= 1;
                if b.st_t <= 0 {
                    b.state = BState::Act;
                    b.st_t = act_time(b.atk);
                    b.st_max = b.st_t;
                    self.boss_act_start(b);
                }
            }
            BState::Act => {
                let f = b.st_max - b.st_t;
                self.boss_act_frame(b, f);
                b.st_t -= 1;
                if b.st_t <= 0 {
                    self.boss_act_end(b);
                    b.state = BState::Recover;
                    b.st_t = recover_time(b.atk);
                    b.st_max = b.st_t;
                }
            }
            BState::Recover => {
                b.st_t -= 1;
                if b.st_t <= 0 {
                    b.state = BState::Idle;
                    b.cool = match b.phase {
                        1 => 50,
                        2 => 32,
                        _ => 22,
                    };
                }
            }
            BState::Intro => {}
        }
        // Contact damage.
        let p = self.pl;
        if !b.hidden && b.z < 20.0 && hit(b.x, b.y - b.z, b.w, b.h, p.x, p.y, p.w, p.h) {
            self.hurt(4);
        }
        // Ambient effects.
        match b.kind {
            BKind::Treant if self.frame % 14 == 0 => {
                let lx = b.x + self.rng.range(-26.0, 26.0);
                let c = if b.phase > 1 { rgb(0x6c2c7c) } else { rgb(0x2c7c1c) };
                let vx = self.rng.range(-0.3, 0.3);
                self.part(lx, b.y - 30.0, vx, 0.5, 60, c, 2, PK::Dot);
            }
            BKind::Guardian if b.phase > 1 && self.frame % 5 == 0 => {
                let vx = self.rng.range(-0.4, 0.4);
                self.part(b.x, b.y - 16.0, vx, -0.6, 20, rgb(0xb040fc), 1, PK::Ember);
            }
            BKind::DarkSorcerer if b.phase == 3 && self.frame % 3 == 0 => {
                let a = self.rng.range(0.0, PI * 2.0);
                self.part(b.x + a.cos() * 20.0, b.y + a.sin() * 20.0, 0.0, -0.5, 18, b.el.light(), 1, PK::Ember);
            }
            _ => {}
        }
    }

    fn arena_clamp(b: &mut Boss) {
        b.x = b.x.clamp(24.0 + b.w / 2.0, WF - 24.0 - b.w / 2.0);
        b.y = b.y.clamp(HUDF + 24.0 + b.h / 2.0, HF - 30.0 - b.h / 2.0);
    }
    fn boss_move(&mut self, b: &mut Boss, spd: f32) {
        let (px, py) = (self.pl.x, self.pl.y);
        b.moving = false;
        let free = matches!(b.state, BState::Idle | BState::Recover) || (b.state == BState::Tele && b.atk == Atk::Guard);
        match b.kind {
            BKind::Treant => {}
            BKind::Guardian | BKind::Golem => {
                if free && b.stagger == 0 {
                    let base = if b.kind == BKind::Golem { 0.35 } else { 0.5 };
                    let d = dist(b.x, b.y, px, py);
                    if d > 42.0 {
                        let a = ang(b.x, b.y, px, py);
                        let (w, h) = (b.w * 0.6, b.h * 0.5);
                        let (mut x, mut y) = (b.x, b.y);
                        self.move_box(&mut x, &mut y, w, h, a.cos() * base * spd, a.sin() * base * spd);
                        b.x = x;
                        b.y = y;
                        b.moving = true;
                        if b.kind == BKind::Golem && b.t % 32 == 0 {
                            self.shake = self.shake.max(3);
                            for _ in 0..4 {
                                let vx = self.rng.range(-1.0, 1.0);
                                self.part(b.x + vx * 10.0, b.y + b.h / 2.0, vx, -0.4, 16, rgb(0x747468), 2, PK::Dot);
                            }
                            self.sfx(Sfx::Push);
                        }
                    }
                }
            }
            BKind::Dragon => {
                if free {
                    if dist(b.x, b.y, b.tx, b.ty) < 6.0 {
                        b.tx = self.rng.range(60.0, 196.0);
                        b.ty = self.rng.range(72.0, 118.0);
                    }
                    let a = ang(b.x, b.y, b.tx, b.ty);
                    b.x += a.cos() * 0.9 * spd;
                    b.y += a.sin() * 0.9 * spd;
                    b.moving = true;
                }
            }
            BKind::Sorcerer | BKind::DarkSorcerer => {
                if free {
                    if dist(b.x, b.y, b.tx, b.ty) < 4.0 {
                        b.tx = self.rng.range(50.0, 206.0);
                        b.ty = self.rng.range(70.0, 130.0);
                    }
                    let a = ang(b.x, b.y, b.tx, b.ty);
                    b.x += a.cos() * 0.6 * spd;
                    b.y += a.sin() * 0.6 * spd;
                }
            }
        }
        Self::arena_clamp(b);
    }

    fn summon(&mut self, b: &Boss, k: EK, n: usize, around_player: bool) {
        for i in 0..n {
            if self.enemies.len() >= 5 {
                break;
            }
            let (x, y) = if around_player {
                let a = i as f32 * PI * 2.0 / n as f32 + self.rng.f();
                (self.pl.x + a.cos() * 56.0, self.pl.y + a.sin() * 40.0)
            } else {
                (b.x + (i as f32 - (n - 1) as f32 / 2.0) * 26.0, b.y + b.h / 2.0 + 10.0)
            };
            let (x, y) = (x.clamp(28.0, WF - 28.0), y.clamp(HUDF + 28.0, HF - 28.0));
            if self.box_solid(x, y, 12.0, 12.0) {
                continue;
            }
            let mut m = self.make_enemy(k, x, y, (b.n - 1).min(3));
            if matches!(b.kind, BKind::Sorcerer | BKind::DarkSorcerer | BKind::Treant) {
                m.el = if b.kind == BKind::Treant { Elem::Earth } else { b.el };
            }
            m.spawn = 36;
            for _ in 0..6 {
                let vx = self.rng.range(-1.0, 1.0);
                self.part(x, y + 6.0, vx, -0.8, 20, rgb(0x5c4880), 2, PK::Dot);
            }
            self.enemies.push(m);
        }
    }

    fn boss_tele_start(&mut self, b: &mut Boss) {
        let (px, py) = (self.pl.x, self.pl.y);
        let tele = b.st_t;
        match b.atk {
            Atk::Roots => {
                let mut spots = vec![(px, py)];
                let extra = if b.phase > 1 { 4 } else { 2 };
                for i in 0..extra {
                    let a = i as f32 * PI * 2.0 / extra as f32 + self.rng.f();
                    spots.push((px + a.cos() * 30.0, py + a.sin() * 24.0));
                }
                spots.push((self.rng.range(40.0, 216.0), self.rng.range(130.0, 210.0)));
                for (x, y) in spots {
                    b.hazards.push(Hazard::circle(HK::Root, x.clamp(24.0, 232.0), y.clamp(60.0, 220.0), 11.0, tele, 18, 3));
                }
                self.sfx(Sfx::Rumble);
            }
            Atk::Branch => {
                let y = py.clamp(128.0, 212.0);
                let (x, vx) = if px > 128.0 { (20.0, 4.6) } else { (236.0, -4.6) };
                b.ax = y;
                b.tx = if vx > 0.0 { -1.0 } else { 1.0 };
                b.hazards.push(Hazard { k: HK::Sweep, x, y, r: 0.0, w: 30.0, h: 14.0, vx, warn: tele, warn_max: tele, act: 50, dmg: 3, track: false });
            }
            Atk::Slam => {
                b.hazards.push(Hazard::circle(HK::Slam, b.x, b.y + b.h / 2.0 - 4.0, 38.0, tele, 10, 4));
                let rocks = if b.phase > 1 { 7 } else { 4 };
                for i in 0..rocks {
                    let (x, y) = if i == 0 {
                        (px, py)
                    } else {
                        (self.rng.range(36.0, 220.0), self.rng.range(70.0, 214.0))
                    };
                    b.hazards.push(Hazard::circle(HK::Rock, x, y, 10.0, tele + 10 + i * 3, 8, 3));
                }
            }
            Atk::Circles => {
                let n = match b.phase {
                    1 => 3,
                    2 => 5,
                    _ => 6,
                };
                for i in 0..n {
                    let (x, y) = if i == 0 {
                        (px, py)
                    } else {
                        let a = self.rng.range(0.0, PI * 2.0);
                        let r = self.rng.range(40.0, 70.0);
                        (px + a.cos() * r, py + a.sin() * r)
                    };
                    b.hazards.push(Hazard::circle(HK::Magic, x.clamp(28.0, 228.0), y.clamp(60.0, 218.0), 18.0, 50, 14, 4));
                }
                self.sfx(Sfx::Spell);
            }
            Atk::Breath => {
                b.bang = ang(b.x + b.face * 26.0, b.y - 8.0, px, py);
                self.sfx(Sfx::Roar);
            }
            Atk::Slash => {
                b.ax = px;
                b.ay = py;
                self.sfx(Sfx::Select);
            }
            Atk::Guard => b.guard = true,
            Atk::Teleport => {
                self.sfx(Sfx::Spell);
            }
            _ => {}
        }
    }
    fn boss_tele_frame(&mut self, b: &mut Boss) {
        let (px, py) = (self.pl.x, self.pl.y);
        match b.atk {
            Atk::Slash if b.st_t > 8 => {
                b.ax = px;
                b.ay = py;
            }
            Atk::Breath => {
                b.bang = ang(b.x + b.face * 26.0, b.y - 8.0, px, py);
                if self.frame % 2 == 0 {
                    let (mx, my) = (b.x + b.face * 30.0, b.y - 10.0);
                    let vx = self.rng.range(-0.5, 0.5);
                    self.part(mx, my, vx, -0.5, 12, rgb(0xfc9838), 1, PK::Ember);
                }
            }
            Atk::Teleport | Atk::Bolts | Atk::Shield if self.frame % 2 == 0 => {
                let a = self.rng.range(0.0, PI * 2.0);
                let (sx, sy) = (b.x + 10.0 + a.cos() * 8.0, b.y - 20.0 + a.sin() * 8.0);
                self.part(sx, sy, -a.cos() * 0.5, -a.sin() * 0.5, 14, b.el.light(), 1, PK::Dot);
            }
            Atk::Raise | Atk::DarkOrbs if self.frame % 2 == 0 => {
                let a = self.rng.range(0.0, PI * 2.0);
                self.part(b.x + a.cos() * 14.0, b.y + a.sin() * 14.0, 0.0, -0.6, 16, rgb(0x9818a8), 2, PK::Dot);
            }
            _ => {}
        }
    }
    fn boss_act_start(&mut self, b: &mut Boss) {
        let (px, py) = (self.pl.x, self.pl.y);
        let ph = b.phase as f32;
        match b.atk {
            Atk::Shock => {
                let n = if b.phase > 1 { 20 } else { 14 };
                for i in 0..n {
                    let a = i as f32 * PI * 2.0 / n as f32;
                    self.eshot_x(b.x, b.y + 22.0, a, 1.5, Elem::Earth, Shot::Wave, 3.0, 2.0, 300);
                }
                self.part(b.x, b.y + 22.0, 0.0, 0.0, 16, rgb(0x98d858), 1, PK::Ring);
                self.shake = 10;
                self.sfx(Sfx::Quake);
            }
            Atk::Summon => match b.kind {
                BKind::Treant => self.summon(b, EK::Slime, 1 + b.phase as usize, false),
                _ => {
                    let k = if b.el == Elem::Fire { EK::Imp } else if b.el == Elem::Ice { EK::Ghost } else { EK::Bat };
                    self.summon(b, k, 2, false);
                }
            },
            Atk::Raise => {
                self.summon(b, EK::Skeleton, 1 + b.phase as usize, true);
                self.sfx(Sfx::Rumble);
            }
            Atk::DarkOrbs => self.dark_ring(b, 0.0),
            Atk::Fireball => {
                let n = if b.phase > 1 { 5 } else { 3 };
                let (mx, my) = (b.x + b.face * 30.0, b.y - 10.0);
                let a = ang(mx, my, px, py);
                for i in 0..n {
                    let da = (i as f32 - (n - 1) as f32 / 2.0) * 0.24;
                    self.eshot_x(mx, my, a + da, 1.9, Elem::Fire, Shot::Fireball, 4.0, 3.0, 300);
                }
                self.sfx(Sfx::Boom);
            }
            Atk::Tail => {
                b.hazards.push(Hazard::circle(HK::Tail, b.x, b.y, 42.0, 0, 12, 3));
                self.sfx(Sfx::Shoot);
            }
            Atk::Slam => {
                self.shake = 16;
                self.sfx(Sfx::Quake);
                let n = if b.phase > 1 { 20 } else { 14 };
                for i in 0..n {
                    let a = i as f32 * PI * 2.0 / n as f32;
                    self.eshot_x(b.x, b.y + b.h / 2.0, a, 1.4, Elem::Earth, Shot::Wave, 3.0, 2.0, 300);
                }
            }
            Atk::Throw => {
                let n = if b.phase > 1 { 3 } else { 1 };
                let a = ang(b.x, b.y - 30.0, px, py);
                for i in 0..n {
                    let da = (i as f32 - (n - 1) as f32 / 2.0) * 0.3;
                    self.eshot_x(b.x, b.y - 30.0, a + da, 2.3, Elem::Earth, Shot::Boulder, 5.0, 3.0, 300);
                }
                self.sfx(Sfx::Push);
            }
            Atk::Teleport => {
                b.hidden = true;
                self.part(b.x, b.y, 0.0, 0.0, 14, b.el.light(), 1, PK::Glow(18.0));
                // Somewhere well away from the player.
                let mut best = (128.0, 80.0);
                for _ in 0..12 {
                    let (x, y) = (self.rng.range(44.0, 212.0), self.rng.range(64.0, 180.0));
                    if dist(x, y, px, py) > 90.0 {
                        best = (x, y);
                        break;
                    }
                }
                b.tx = best.0;
                b.ty = best.1;
            }
            Atk::Shield => {
                b.shield = if b.kind == BKind::DarkSorcerer { 10 } else { 8 };
                b.shield_t = 360;
                self.part(b.x, b.y, 0.0, 0.0, 14, b.el.light(), 1, PK::Ring);
                self.sfx(Sfx::Freeze);
            }
            Atk::Slash => self.sfx(Sfx::Shoot),
            _ => {}
        }
        let _ = ph;
    }
    fn dark_ring(&mut self, b: &Boss, off: f32) {
        let n = if b.phase > 1 { 14 } else { 10 };
        for i in 0..n {
            let a = off + i as f32 * PI * 2.0 / n as f32;
            self.eshot_x(b.x, b.y, a, 1.3, Elem::Neutral, Shot::Dark, 2.5, 2.0, 400);
        }
        self.sfx(Sfx::Spell);
    }
    fn boss_act_frame(&mut self, b: &mut Boss, f: i32) {
        let (px, py) = (self.pl.x, self.pl.y);
        match b.atk {
            Atk::Breath => {
                if f % 2 == 0 {
                    let amp = if b.phase > 1 { 0.5 } else { 0.18 };
                    let a = b.bang + (f as f32 * 0.09).sin() * amp;
                    let (mx, my) = (b.x + b.face * 30.0, b.y - 10.0);
                    let s = 2.4 + self.rng.f() * 0.4;
                    let j = self.rng.range(-0.08, 0.08);
                    self.eshot_x(mx, my, a + j, s, Elem::Fire, Shot::Flame, 3.0, 2.0, 55);
                }
            }
            Atk::Fly => {
                if f < 30 {
                    b.z += 5.0;
                    if b.z > 40.0 {
                        b.hidden = true;
                    }
                } else if f == 30 {
                    let mut h = Hazard::circle(HK::Slam, px, py, 24.0, 45, 10, 4);
                    h.track = true;
                    b.hazards.push(h);
                } else if f < 75 {
                    if let Some(h) = b.hazards.iter().find(|h| h.track) {
                        b.x = h.x;
                        b.y = h.y - 8.0;
                    }
                } else if f < 90 {
                    b.hidden = false;
                    b.z = (b.z - 10.0).max(0.0);
                } else if f == 90 {
                    b.z = 0.0;
                    for i in 0..12 {
                        let a = i as f32 * PI / 6.0;
                        self.eshot_x(b.x, b.y + 10.0, a, 1.6, Elem::Fire, Shot::Fireball, 3.0, 2.0, 200);
                    }
                    self.shake = 14;
                    self.sfx(Sfx::Boom);
                    for _ in 0..12 {
                        let vx = self.rng.range(-2.0, 2.0);
                        self.part(b.x + vx * 6.0, b.y + 14.0, vx, -0.6, 22, rgb(0x747468), 2, PK::Dot);
                    }
                }
            }
            Atk::Slash => {
                if f < 12 {
                    let a = ang(b.x, b.y, b.ax, b.ay);
                    let (mut x, mut y) = (b.x, b.y);
                    let (w, h) = (b.w * 0.6, b.h * 0.5);
                    self.move_box(&mut x, &mut y, w, h, a.cos() * 4.2, a.sin() * 4.2);
                    b.x = x;
                    b.y = y;
                    if self.frame % 2 == 0 {
                        self.part(b.x, b.y + 10.0, 0.0, 0.0, 10, rgb(0x5c5c64), 2, PK::Dot);
                    }
                } else if f == 12 {
                    let a = ang(b.x, b.y, px, py);
                    b.hazards.push(Hazard::circle(HK::Tail, b.x + a.cos() * 18.0, b.y + a.sin() * 18.0, 24.0, 0, 8, 4));
                    if b.phase > 1 {
                        for i in -1..=1 {
                            self.eshot_x(b.x, b.y, a + i as f32 * 0.3, 2.2, Elem::Neutral, Shot::Dark, 3.0, 2.0, 200);
                        }
                    }
                    self.shake = 6;
                    self.sfx(Sfx::Hit);
                }
            }
            Atk::DarkOrbs if f == 12 && b.phase > 1 => self.dark_ring(b, 0.22),
            Atk::Teleport => {
                if f > 20 && self.frame % 2 == 0 {
                    let a = self.rng.range(0.0, PI * 2.0);
                    self.part(b.tx + a.cos() * 12.0, b.ty + a.sin() * 12.0, -a.cos() * 0.6, -a.sin() * 0.6, 12, b.el.light(), 1, PK::Dot);
                }
            }
            Atk::Bolts => {
                let every = if b.phase > 1 { 8 } else { 12 };
                if f % every == 0 {
                    let n = if b.phase > 1 { 5 } else { 3 };
                    let a = ang(b.x + 10.0, b.y - 20.0, px, py);
                    for i in 0..n {
                        let da = (i as f32 - (n - 1) as f32 / 2.0) * 0.2;
                        self.eshot_x(b.x + 10.0, b.y - 20.0, a + da, 2.0, b.el, Shot::Orb, 2.5, 2.0, 400);
                    }
                    self.sfx(Sfx::Shoot);
                }
                if b.kind == BKind::DarkSorcerer && b.phase == 3 && f % 4 == 0 {
                    self.eshot_x(b.x, b.y, b.t as f32 * 0.17, 1.5, b.el, Shot::Orb, 2.5, 2.0, 400);
                }
            }
            _ => {}
        }
    }
    fn boss_act_end(&mut self, b: &mut Boss) {
        match b.atk {
            Atk::Guard => b.guard = false,
            Atk::Slash => b.stagger = recover_time(Atk::Slash),
            Atk::Fly => {
                b.z = 0.0;
                b.hidden = false;
            }
            Atk::Teleport => {
                b.x = b.tx;
                b.y = b.ty;
                b.hidden = false;
                b.el = Elem::from_idx((b.el.idx() + 1) % 4);
                b.weak = b.el.opposite();
                b.resist = b.el;
                self.float(format!("{} FORM!", b.el.name()), b.x - 36.0, b.y - 40.0, b.el.light());
                self.part(b.x, b.y, 0.0, 0.0, 14, b.el.light(), 1, PK::Ring);
                self.part(b.x, b.y, 0.0, 0.0, 12, b.el.light(), 1, PK::Glow(18.0));
            }
            _ => {}
        }
    }
    fn update_hazards(&mut self, b: &mut Boss) {
        let (px, py) = (self.pl.x, self.pl.y);
        let mut hurt = 0;
        for h in b.hazards.iter_mut() {
            if h.warn > 0 {
                h.warn -= 1;
                if h.track && h.warn > 15 {
                    h.x += (px - h.x) * 0.12;
                    h.y += (py - h.y) * 0.12;
                }
                if h.warn == 0 && h.k != HK::Sweep {
                    let el = match h.k {
                        HK::Magic => Elem::Storm,
                        HK::Root | HK::Rock | HK::Slam => Elem::Earth,
                        _ => Elem::Neutral,
                    };
                    let (x, y) = (h.x, h.y);
                    self.impact(x, y, el);
                    if matches!(h.k, HK::Slam | HK::Rock) {
                        self.shake = self.shake.max(4);
                    }
                }
                continue;
            }
            if h.act > 0 {
                h.act -= 1;
                h.x += h.vx;
                if h.hits(px, py) {
                    hurt = hurt.max(h.dmg);
                }
            }
        }
        b.hazards.retain(|h| h.warn > 0 || h.act > 0);
        if hurt > 0 {
            self.hurt(hurt);
        }
    }

    /// Boss defeat sequence, then the reward.
    pub(super) fn update_lair_state(&mut self) {
        if !self.boss_dead {
            return;
        }
        self.clear_t += 1;
        let (bx, by, bw, bh, el) = self.boss.as_ref().map_or((128.0, 100.0, 30.0, 30.0, Elem::Neutral), |b| (b.x, b.y, b.w, b.h, b.el));
        if self.clear_t < 110 && self.clear_t % 7 == 0 {
            let (ox, oy) = (self.rng.range(-bw / 2.0, bw / 2.0), self.rng.range(-bh / 2.0, bh / 2.0));
            self.boom(bx + ox, by + oy, 14, el);
            self.sfx(Sfx::Boom);
            self.shake = 6;
        }
        if self.clear_t == 110 {
            self.flash = 20;
            if let Some(b) = self.boss.as_mut() {
                b.gone = true;
            }
            self.boom(bx, by, 70, el);
            self.sfx(Sfx::BigBoom);
            self.shake = 20;
        }
        if self.clear_t >= 200 {
            self.finish_lair();
        }
    }

    // ============================================================ drawing
    pub(super) fn draw_hazards(&self, scr: &mut Screen) {
        if let Some(b) = &self.boss {
            self.draw_hazard_list(scr, &b.hazards, b.el);
        }
        self.draw_hazard_list(scr, &self.hazards, Elem::Earth);
    }
    /// Ground danger zones (bosses and overworld encounters share the look).
    pub(super) fn draw_hazard_list(&self, scr: &mut Screen, list: &[Hazard], el: Elem) {
        struct Tint {
            el: Elem,
        }
        let b = Tint { el };
        let blink = (self.frame >> 2) & 1 == 1;
        for h in list {
            let (x, y, r) = (h.x as i32, h.y as i32, h.r as i32);
            if h.warn > 0 {
                let p = 1.0 - h.warn as f32 / h.warn_max as f32;
                match h.k {
                    HK::Sweep => {
                        scr.blend(16, y - 7, W - 32, 14, rgb(0xd82800), 0.12 + 0.12 * blink as i32 as f32);
                    }
                    HK::Rock => {
                        scr.blend_ellipse(x, y, (r as f32 * (0.4 + p * 0.6)) as i32, (r as f32 * 0.5 * (0.4 + p * 0.6)) as i32, BLACK, 0.5);
                        let fall = (1.0 - p) * 120.0;
                        if fall < 110.0 {
                            scr.disc(x, y - fall as i32 - 6, 6, rgb(0x5c5c58));
                            scr.disc(x - 2, y - fall as i32 - 8, 2, rgb(0x9c9c90));
                        }
                    }
                    HK::Magic => {
                        let c = b.el.light();
                        scr.blend_disc(x, y, r, c, 0.10 + p * 0.2);
                        scr.ring(x, y, r, if blink { WHITE } else { c });
                        for i in 0..6 {
                            let a = i as f32 * PI / 3.0 + self.frame as f32 * 0.08;
                            scr.fill(x + (a.cos() * (r - 4) as f32) as i32, y + (a.sin() * (r - 4) as f32) as i32, 2, 2, c);
                        }
                    }
                    HK::Root => {
                        scr.blend_ellipse(x, y, r, r / 2 + 2, rgb(0x3c2408), 0.35 + p * 0.3);
                        let c = if blink { rgb(0xd82800) } else { rgb(0x8c5020) };
                        scr.ellipse(x, y, r, r / 2 + 2, c);
                        scr.line(x - r / 2, y - 1, x + r / 3, y + 2, rgb(0x2c1804));
                        scr.line(x + 1, y - 3, x - 2, y + 3, rgb(0x2c1804));
                    }
                    _ => {
                        let c = if blink { rgb(0xd82800) } else { rgb(0xfc9838) };
                        scr.blend_ellipse(x, y, r, r / 2 + 2, rgb(0xd82800), 0.12 + p * 0.2);
                        scr.ellipse(x, y, r, r / 2 + 2, c);
                    }
                }
            } else {
                let q = h.act;
                match h.k {
                    HK::Root => {
                        for i in 0..5 {
                            let sx = x - r + i * r / 2;
                            let ht = 6 + (i % 2) * 5 + q.min(6);
                            scr.line(sx, y + 2, sx + 1, y + 2 - ht, rgb(0x5c3c10));
                            scr.line(sx + 1, y + 2, sx + 2, y + 2 - ht + 2, rgb(0x2c7c1c));
                        }
                    }
                    HK::Sweep => {
                        let (hx, hy) = (x - h.w as i32 / 2, y - h.h as i32 / 2);
                        scr.fill(hx, hy + 4, h.w as i32, 6, rgb(0x4c2c0c));
                        scr.fill(hx, hy + 4, h.w as i32, 1, rgb(0x8c5c20));
                        for i in 0..4 {
                            scr.disc(hx + 4 + i * 8, hy + 2 + (i % 2) * 8, 3, rgb(0x2c7c1c));
                        }
                    }
                    HK::Magic => {
                        let c = b.el.light();
                        scr.blend_disc(x, y, r, c, 0.5);
                        scr.ring(x, y, r + (14 - q).max(0), WHITE);
                    }
                    HK::Rock => {
                        scr.disc(x, y - 2, 7, rgb(0x5c5c58));
                        scr.disc(x - 2, y - 4, 3, rgb(0x9c9c90));
                    }
                    HK::Slam => {
                        scr.ring(x, y, r - q, rgb(0x9c9c90));
                        scr.ring(x, y, r - q + 3, rgb(0x5c5c58));
                    }
                    HK::Tail => {
                        scr.ring(x, y, r, rgb(0xfc9838));
                        scr.ring(x, y, r - 2, rgb(0xd82800));
                    }
                }
            }
        }
    }

    /// Generated boss art (sheet `boss_<kind>` with idle / attack / optional hurt rows).
    /// The sprite's feet sit on the boss's ground point; during the entrance it rises out
    /// of the floor. Returns false to fall back to the procedural drawing.
    fn draw_boss_hd(&self, scr: &mut Screen, b: &Boss, shake: f32, intro_p: f32, dark: f32) -> bool {
        let name = match b.kind {
            BKind::Treant => "boss_treant",
            BKind::Guardian => "boss_guardian",
            BKind::Golem => "boss_golem",
            BKind::Dragon => "boss_dragon",
            BKind::Sorcerer => "boss_sorcerer",
            BKind::DarkSorcerer => "boss_darksorcerer",
        };
        let Some(sh) = self.art.sheet(name) else { return false };
        let attacking = matches!(b.state, BState::Tele | BState::Act);
        let anim = if b.hurt_t > 0 && b.alive { sh.anim("hurt") } else { None }
            .or_else(|| if attacking { sh.anim("attack") } else { None })
            .or_else(|| sh.anim("idle"));
        let Some(anim) = anim else { return false };
        let img = anim.at(if b.st.immobile() { 0 } else { b.t as u32 });
        let fx = if b.flash > 0 {
            Tint::Solid(WHITE)
        } else if dark > 0.0 {
            Tint::Mix(BLACK, dark)
        } else if b.st.frozen() {
            Tint::Mix(rgb(0xc4ecfc), 0.5)
        } else {
            Tint::None
        };
        let ground = b.y + b.h / 2.0;
        let feet = ground - b.z;
        scr.blend_ellipse(b.x as i32, ground as i32, (b.w * 0.6) as i32, 4, BLACK, 0.35);
        let (ax, ay) = (sh.cell.0 / 2, sh.cell.1 - 2);
        if intro_p < 1.0 {
            // Rise out of the ground: clip at the floor line and slide up.
            let sink = (1.0 - intro_p) * sh.cell.1 as f32 / ZOOM;
            let line = scr.ty(ground);
            scr.clip_screen(0, HUD_PX, SW, line.max(HUD_PX));
            scr.spr_hd_anchor(img, b.x + shake, feet + sink, ax, ay, false, fx);
            scr.reset_clip();
        } else {
            scr.spr_hd_anchor(img, b.x + shake, feet, ax, ay, false, fx);
        }
        true
    }

    pub(super) fn draw_boss(&self, scr: &mut Screen) {
        let Some(b) = &self.boss else { return };
        if b.gone {
            return;
        }
        let dying = !b.alive;
        if dying && self.frame & 2 != 0 && self.clear_t > 60 {
            return;
        }
        if b.hidden {
            // Sorcerer re-materialising / dragon high above: only a shadow or sparkles.
            if b.kind == BKind::Dragon {
                scr.blend_ellipse(b.x as i32, b.y as i32 + 16, 22, 6, BLACK, 0.35);
            }
            return;
        }
        let intro_p = if b.state == BState::Intro { (b.intro as f32 / 110.0).min(1.0) } else { 1.0 };
        let dark = if dying { (self.clear_t as f32 / 140.0).min(0.7) } else { 0.0 };
        let shake = if b.hurt_t > 0 { (b.hurt_t % 2) * 2 - 1 } else { 0 };
        let (x, y) = (b.x as i32 + shake, (b.y - b.z) as i32);
        let t = b.t;
        if !self.draw_boss_hd(scr, b, shake as f32, intro_p, dark) {
            let mut pen = Pen { s: &mut *scr, white: b.flash > 0, dark };
            match b.kind {
                BKind::Treant => draw_treant(&mut pen, b, x, y, t, intro_p, dying, self.clear_t),
                BKind::Guardian => draw_guardian(&mut pen, b, x, y, t, intro_p),
                BKind::Golem => draw_golem(&mut pen, b, x, y, t, intro_p, dying, self.clear_t),
                BKind::Dragon => draw_dragon(&mut pen, b, x, y, t),
                BKind::Sorcerer | BKind::DarkSorcerer => draw_sorcerer(&mut pen, b, x, y, t, intro_p),
            }
        }
        let (bx, by, bw, bh) = (x, y, b.w as i32, b.h as i32);
        // Status overlays follow the boss.
        if b.st.chilled() {
            scr.blend(bx - bw / 2, by - bh / 2, bw, bh, rgb(0xa4e4fc), 0.25);
        }
        if b.st.frozen() {
            draw_ice_block(scr, bx as f32, by as f32, bw as f32, bh as f32, b.st.encase(), self.frame);
        }
        if b.st.burning() {
            draw_flames(scr, bx as f32, by as f32, bw as f32, bh as f32, b.st.flame_scale(), self.frame, 7);
        }
        if b.shield > 0 {
            let r = (b.w.max(b.h) * 0.8) as i32 + 4;
            scr.blend_disc(bx, by, r, b.el.light(), 0.15);
            scr.ring(bx, by, r, if self.frame % 6 < 3 { WHITE } else { b.el.light() });
            for i in 0..b.shield.min(10) {
                let a = i as f32 * PI * 2.0 / 10.0 + self.frame as f32 * 0.05;
                scr.fill(bx + (a.cos() * r as f32) as i32, by + (a.sin() * r as f32) as i32, 2, 2, WHITE);
            }
        }
        if b.stagger > 0 {
            for i in 0..3 {
                let a = self.frame as f32 * 0.15 + i as f32 * 2.1;
                scr.fill(bx + (a.cos() * 9.0) as i32, by - bh / 2 - 4 + (a.sin() * 3.0) as i32, 2, 2, rgb(0xfce040));
            }
        }
        // Telegraph warning over big attacks.
        if b.alive && b.state == BState::Tele && big_attack(b.atk) && (self.frame >> 2) & 1 == 1 {
            let (ix, iy) = (bx, by - bh / 2 - 14);
            scr.fill(ix - 1, iy, 3, 7, rgb(0xfc3c3c));
            scr.fill(ix - 1, iy + 9, 3, 2, rgb(0xfc3c3c));
        }
    }
}

// ---------------------------------------------------------------- painter
/// Wraps the screen so boss drawings can flash white or darken as a whole.
pub(super) struct Pen<'a> {
    pub s: &'a mut Screen,
    pub white: bool,
    pub dark: f32,
}
impl Pen<'_> {
    fn c(&self, c: u32) -> u32 {
        if self.white {
            WHITE
        } else if self.dark > 0.0 {
            mix(c, BLACK, self.dark)
        } else {
            c
        }
    }
    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let c = self.c(rgb(c));
        self.s.fill(x, y, w, h, c);
    }
    fn disc(&mut self, x: i32, y: i32, r: i32, c: u32) {
        let c = self.c(rgb(c));
        self.s.disc(x, y, r, c);
    }
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        let c = self.c(rgb(c));
        self.s.line(x0, y0, x1, y1, c);
    }
    fn thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32, w: i32) {
        for o in 0..w {
            self.line(x0, y0 + o, x1, y1 + o, c);
            self.line(x0 + o, y0, x1 + o, y1, c);
        }
    }
    fn glow(&mut self, x: i32, y: i32, r: i32, c: u32, a: f32) {
        if !self.white {
            self.s.blend_disc(x, y, r, rgb(c), a);
        }
    }
}

/// Ice encasing: translucent block growing up from the ground, with highlights.
pub(super) fn draw_ice_block(scr: &mut Screen, x: f32, y: f32, w: f32, h: f32, encase: f32, frame: u64) {
    let (bw, bh) = (w as i32 + 6, h as i32 + 6);
    let full = bh;
    let cur = ((full as f32) * encase).max(2.0) as i32;
    let (x0, y1) = (x as i32 - bw / 2, y as i32 + bh / 2);
    let y0 = y1 - cur;
    scr.blend(x0, y0, bw, cur, rgb(0xa4e4fc), 0.42);
    scr.frame_rect(x0, y0, bw, cur, rgb(0xd4f4fc));
    scr.fill(x0 + 2, y0 + 2, 1, (cur - 4).max(0), WHITE);
    if cur > 8 {
        scr.line(x0 + 3, y0 + 3, x0 + bw / 2, y0 + 3 + bw / 3, WHITE);
    }
    if frame % 24 < 3 {
        scr.fill(x0 + bw - 4, y0 + 2, 2, 2, WHITE);
    }
}

/// Animated flames licking up around a burning target.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_flames(scr: &mut Screen, x: f32, y: f32, w: f32, h: f32, scale: f32, frame: u64, seed: u32) {
    let n = ((w / 5.0) as i32).clamp(3, 8);
    let base = (y + h / 2.0) as i32;
    for i in 0..n {
        let fx = (x - w / 2.0 + (i as f32 + 0.5) * w / n as f32) as i32;
        let ph = frame as f32 * 0.45 + i as f32 * 1.7 + seed as f32 * 0.37;
        let ht = ((5.0 + h * 0.35 + ph.sin() * 3.0) * scale) as i32;
        let fy = base - 2 - ((i * 7 + seed as i32) % 5) - (h * 0.25) as i32 * (i % 2);
        for k in 0..ht.max(1) {
            let t = k as f32 / ht.max(1) as f32;
            let c = if t < 0.35 {
                rgb(0xd82800)
            } else if t < 0.7 {
                rgb(0xfc9838)
            } else {
                rgb(0xfce040)
            };
            let wid = ((1.0 - t) * 3.0).round() as i32 + 1;
            let sway = ((ph + k as f32 * 0.4).sin() * 1.2) as i32;
            if (frame as i32 + k + i) % 7 != 0 {
                scr.fill(fx - wid / 2 + sway, fy - k, wid, 1, c);
            }
        }
    }
}

// ---------------------------------------------------------------- boss art
#[allow(clippy::too_many_arguments)]
fn draw_treant(p: &mut Pen, b: &Boss, x: i32, y: i32, t: i32, intro: f32, dying: bool, clear_t: i32) {
    let sink = ((1.0 - intro) * 60.0) as i32 + if dying { (clear_t / 3).min(40) } else { 0 };
    let y = y + sink;
    let ground = (b.y as i32) + 30;
    p.s.clip(0, HUD, W, ground + 1);
    let sway = ((t as f32) * 0.05).sin() * 2.0;
    let ph2 = b.phase > 1;
    // Roots.
    for i in 0..6 {
        let rx = x - 25 + i * 10;
        p.thick(rx, y + 22, rx + (i - 3) * 5, y + 30, 0x3c2408, 2);
    }
    // Trunk with bark.
    p.fill(x - 13, y - 10, 26, 36, 0x4c2c0c);
    p.fill(x - 11, y - 10, 5, 36, 0x6c4418);
    for i in 0..5 {
        p.fill(x - 8 + i * 4, y - 6 + (i % 2) * 6, 1, 14, 0x2c1804);
    }
    // Branch arms (raised or swinging when attacking).
    let (lx, ly, rx2, ry) = match (b.state, b.atk) {
        (BState::Tele, Atk::Branch) => {
            if b.tx < 0.0 {
                (x - 30, y - 38, x + 36, y - 6)
            } else {
                (x - 36, y - 6, x + 30, y - 38)
            }
        }
        (BState::Act, Atk::Branch) => {
            let sweep = b.hazards.iter().find(|h| h.k == HK::Sweep).map(|h| (h.x as i32, h.y as i32));
            match sweep {
                Some((hx, hy)) if b.tx < 0.0 => (hx, hy, x + 36, y - 6),
                Some((hx, hy)) => (x - 36, y - 6, hx, hy),
                None => (x - 36, y - 6, x + 36, y - 6),
            }
        }
        (BState::Tele, Atk::Shock) => (x - 32, y + 14, x + 32, y + 14),
        _ => (x - 36, y - 6 + sway as i32, x + 36, y - 6 - sway as i32),
    };
    p.thick(x - 12, y - 2, lx, ly, 0x4c2c0c, 3);
    p.thick(x + 12, y - 2, rx2, ry, 0x4c2c0c, 3);
    for (ex, ey) in [(lx, ly), (rx2, ry)] {
        p.line(ex, ey, ex - 4, ey - 5, 0x6c4418);
        p.line(ex, ey, ex + 4, ey - 4, 0x6c4418);
        p.disc(ex, ey - 4, 4, if ph2 { 0x3c1c4c } else { 0x0c4c14 });
    }
    // Canopy.
    let (c0, c1) = if ph2 { (0x3c1c4c, 0x6c2c7c) } else { (0x0c4c14, 0x2c7c1c) };
    let s = sway as i32;
    for &(dx, dy, r) in &[(-17, -24, 12), (17, -24, 12), (0, -36, 14), (-9, -14, 10), (9, -14, 10)] {
        p.disc(x + dx + s / 2, y + dy, r, c0);
    }
    for &(dx, dy, r) in &[(-19, -27, 6), (15, -27, 6), (-3, -40, 7)] {
        p.disc(x + dx + s / 2, y + dy, r, c1);
    }
    // Face.
    let eye = if ph2 { 0xfc3c3c } else { 0xfce040 };
    let pulse = 0.25 + ((t as f32) * 0.1).sin().abs() * 0.3;
    p.glow(x - 5, y - 1, 5, eye, pulse);
    p.glow(x + 5, y - 1, 5, eye, pulse);
    p.fill(x - 8, y - 2, 5, 3, eye);
    p.fill(x + 3, y - 2, 5, 3, eye);
    let mouth = if b.state == BState::Act { 7 } else { 4 };
    p.fill(x - 5, y + 8, 10, mouth, 0x100804);
    p.fill(x - 4, y + 8, 2, 2, 0x3c2408);
    p.fill(x + 2, y + 8, 2, 2, 0x3c2408);
    p.s.reset_clip();
}

fn draw_guardian(p: &mut Pen, b: &Boss, x: i32, y: i32, t: i32, intro: f32) {
    if intro < 1.0 && ((t >> 1) % 4) as f32 > intro * 4.0 {
        return; // flickering in from the dark mist
    }
    let f = if b.face < 0.0 { -1 } else { 1 };
    let step = if b.moving { ((t / 8) % 2) * 2 - 1 } else { 0 };
    let ph2 = b.phase > 1;
    // Tattered cape.
    let sway = ((t as f32) * 0.08).sin() as i32;
    p.fill(x - 9 - f * 2, y - 8, 18, 22, 0x581010);
    for i in 0..9 {
        p.fill(x - 9 - f * 2 + i * 2 + sway, y + 14, 1, 2 + (i % 3), 0x581010);
    }
    // Legs.
    p.fill(x - 6, y + 8, 4, 10 + step, 0x404858);
    p.fill(x + 2, y + 8, 4, 10 - step, 0x404858);
    p.fill(x - 6, y + 16 + step, 5, 2, 0x2c3038);
    p.fill(x + 2, y + 16 - step, 5, 2, 0x2c3038);
    // Torso armor with exposed ribs.
    p.fill(x - 8, y - 8, 16, 16, 0x505a6c);
    p.fill(x - 8, y - 8, 16, 2, 0x7c8698);
    for i in 0..3 {
        p.fill(x - 5, y - 3 + i * 3, 10, 1, 0xd8d8c8);
    }
    // Skull, horned helmet, glowing eyes.
    p.disc(x, y - 14, 5, 0xd8d8c8);
    p.fill(x - 6, y - 21, 12, 5, 0x404858);
    p.line(x - 6, y - 20, x - 10, y - 26, 0xbcbcbc);
    p.line(x + 5, y - 20, x + 9, y - 26, 0xbcbcbc);
    let eye = if ph2 { 0xb040fc } else { 0x3cbcfc };
    p.glow(x, y - 14, 6, eye, 0.3);
    p.fill(x - 3, y - 15, 2, 2, eye);
    p.fill(x + 1, y - 15, 2, 2, eye);
    p.fill(x - 2, y - 11, 4, 1, 0x303030);
    // Sword (back hand).
    let (hx, hy) = (x - f * 10, y);
    let (sx, sy) = match (b.state, b.atk) {
        (BState::Tele, Atk::Slash) => (x - f * 6, y - 32),
        (BState::Act, Atk::Slash) => (x + f * 28, y + 2),
        (BState::Tele, Atk::Raise) => (x - f * 2, y - 34),
        _ => (x - f * 14, y + 16),
    };
    p.thick(hx, hy, sx, sy, 0xdcdce4, 1);
    p.line(hx - 2, hy, hx + 2, hy, 0x8c5020);
    if b.state == BState::Tele && b.atk == Atk::Slash && t % 6 < 3 {
        p.fill(sx - 2, sy, 5, 1, 0xfcfcfc);
        p.fill(sx, sy - 2, 1, 5, 0xfcfcfc);
    }
    // Shield (front hand).
    let raise = if b.guard { -6 } else { 0 };
    let (shx, shy) = (x + f * 9, y + raise);
    p.fill(shx - 5, shy - 8, 10, 14, 0x60687c);
    p.fill(shx - 5, shy - 8, 10, 1, 0xa0a8bc);
    p.fill(shx - 1, shy - 4, 2, 6, 0xd8d8c8);
    p.fill(shx - 3, shy - 2, 6, 2, 0xd8d8c8);
    if b.guard {
        p.glow(shx, shy, 10, 0x3cbcfc, 0.25);
        let c = p.c(rgb(0x3cbcfc));
        p.s.frame_rect(shx - 6, shy - 9, 12, 16, c);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_golem(p: &mut Pen, b: &Boss, x: i32, y: i32, t: i32, intro: f32, dying: bool, clear_t: i32) {
    let bob = if b.moving && (t % 32) < 4 { 2 } else { 0 };
    // Intro: parts fall from above and lock together. Death: parts slump apart.
    let drop = |k: i32| -> i32 {
        let d = ((1.0 - intro) * (120 + k * 30) as f32) as i32;
        let crumble = if dying { (clear_t * (k + 1) / 6).min(30) } else { 0 };
        -d + crumble
    };
    let ph2 = b.phase > 1;
    let y = y + bob;
    // Legs.
    p.fill(x - 16, y + 10 + drop(0), 11, 16, 0x5c5c58);
    p.fill(x + 5, y + 10 + drop(0), 11, 16, 0x5c5c58);
    // Torso.
    let ty = y + drop(1);
    p.fill(x - 20, ty - 16, 40, 28, 0x6c6c64);
    p.fill(x - 20, ty - 16, 40, 2, 0x8c8c84);
    p.line(x - 14, ty - 10, x - 8, ty + 2, 0x3c3c38);
    p.line(x + 10, ty - 12, x + 14, ty - 2, 0x3c3c38);
    // Core (glows through once the armor is gone).
    let core = if ph2 { 0xfc9838 } else { 0x3cbcfc };
    let pulse = 0.3 + ((t as f32) * 0.15).sin().abs() * 0.4;
    if b.armor <= 0.0 {
        p.glow(x, ty - 2, 10, core, pulse);
    }
    p.disc(x, ty - 2, 5, core);
    p.fill(x - 1, ty - 4, 2, 2, 0xfcfcfc);
    // Armor plates.
    if b.armor > 0.0 {
        let plates = ((b.armor / b.armor_max) * 4.0).ceil() as i32;
        let spots = [(x - 18, ty - 14), (x + 2, ty - 14), (x - 18, ty), (x + 2, ty)];
        for (i, &(px, py)) in spots.iter().enumerate() {
            if (i as i32) < plates {
                p.fill(px, py, 16, 12, 0x9c9c90);
                p.fill(px, py, 16, 1, 0xc8c8bc);
                p.fill(px + 2, py + 2, 1, 1, 0x3c3c38);
                p.fill(px + 13, py + 2, 1, 1, 0x3c3c38);
            }
        }
    }
    // Head.
    let hy = y + drop(2);
    p.fill(x - 8, hy - 28, 16, 12, 0x747468);
    let eye = if ph2 { 0xfc9838 } else { 0x3cbcfc };
    p.fill(x - 5, hy - 24, 3, 2, eye);
    p.fill(x + 2, hy - 24, 3, 2, eye);
    // Arms.
    let ay = y + drop(3);
    match (b.state, b.atk) {
        (BState::Tele, Atk::Slam) => {
            p.fill(x - 26, ay - 46, 12, 18, 0x5c5c58);
            p.fill(x + 14, ay - 46, 12, 18, 0x5c5c58);
        }
        (BState::Tele, Atk::Throw) => {
            p.fill(x - 20, ay - 40, 10, 16, 0x5c5c58);
            p.fill(x + 10, ay - 40, 10, 16, 0x5c5c58);
            p.disc(x, ay - 44, 8, 0x5c5c58);
            p.disc(x - 2, ay - 46, 3, 0x9c9c90);
        }
        (BState::Act, Atk::Slam) | (BState::Recover, Atk::Slam) => {
            p.fill(x - 30, ay + 12, 12, 12, 0x5c5c58);
            p.fill(x + 18, ay + 12, 12, 12, 0x5c5c58);
        }
        _ => {
            p.fill(x - 30, ay - 12, 10, 26, 0x5c5c58);
            p.fill(x + 20, ay - 12, 10, 26, 0x5c5c58);
        }
    }
}

fn draw_dragon(p: &mut Pen, b: &Boss, x: i32, y: i32, t: i32) {
    let f = if b.face < 0.0 { -1 } else { 1 };
    if b.z > 0.0 || b.state == BState::Intro {
        p.s.blend_ellipse(x, b.y as i32 + 16, 24, 6, BLACK, 0.35);
    }
    let flying = b.z > 0.0 || b.state == BState::Intro || (b.state == BState::Tele && b.atk == Atk::Fly);
    let flap = ((t as f32) * if flying { 0.4 } else { 0.15 }).sin();
    let (red, dark, belly) = if b.phase > 1 { (0xd82810, 0x801008, 0xfce040) } else { (0xb82010, 0x681008, 0xfc9838) };
    // Wings.
    for s in [-1, 1] {
        let (sx, sy) = (x + s * 6, y - 8);
        let tip = (x + s * 42, y - 22 - (flap * 14.0) as i32);
        let mid = (x + s * 30, y + 6 - (flap * 6.0) as i32);
        for k in 0..=10 {
            let q = k as f32 / 10.0;
            let ex = tip.0 + ((mid.0 - tip.0) as f32 * q) as i32;
            let ey = tip.1 + ((mid.1 - tip.1) as f32 * q) as i32;
            p.line(sx, sy, ex, ey, dark);
        }
        p.thick(sx, sy, tip.0, tip.1, red, 1);
        p.line(sx, sy, mid.0, mid.1, red);
    }
    // Tail.
    let tail_swing = if b.state == BState::Act && b.atk == Atk::Tail { 10.0 } else { 3.0 };
    for i in 1..7 {
        let tx = x - f * (10 + i * 6);
        let ty = y + 8 + ((t as f32) * 0.12 + i as f32).sin().mul_add(tail_swing, 0.0) as i32;
        p.disc(tx, ty, (6 - i).max(2) as i32, red);
    }
    let (tx, ty) = (x - f * 50, y + 8);
    p.line(tx, ty, tx - f * 5, ty - 4, 0xfcfcfc);
    // Body.
    p.disc(x, y, 13, red);
    p.disc(x + f * 2, y + 4, 8, belly);
    for i in 0..4 {
        p.fill(x - 8 + i * 5, y - 10, 2, 2, dark);
    }
    // Neck and head.
    let rear = if b.state == BState::Tele && b.atk == Atk::Breath { 5 } else { 0 };
    for i in 0..4 {
        p.disc(x + f * (8 + i * 4) - f * rear * i / 3, y - 6 - i * 2, 5, red);
    }
    let (hx, hy) = (x + f * 26 - f * rear, y - 12 - rear);
    p.disc(hx, hy, 7, red);
    p.disc(hx + f * 6, hy + 2, 4, red);
    p.line(hx - f * 3, hy - 6, hx - f * 9, hy - 12, 0xfcfcfc);
    p.line(hx + f * 1, hy - 6, hx - f * 4, hy - 13, 0xfcfcfc);
    p.fill(hx, hy - 3, 2, 2, 0xfce040);
    let open = b.state == BState::Act && matches!(b.atk, Atk::Breath | Atk::Fireball);
    if open {
        p.fill(hx + f * 4 - 2, hy + 3, 6, 3, 0x100404);
        p.glow(hx + f * 8, hy + 3, 5, 0xfce040, 0.5);
    }
    if b.state == BState::Tele && matches!(b.atk, Atk::Breath | Atk::Fireball) {
        let r = 2 + (b.st_max - b.st_t) / 6;
        p.glow(hx + f * 8, hy + 2, r, 0xfc9838, 0.45);
    }
}

fn draw_sorcerer(p: &mut Pen, b: &Boss, x: i32, y: i32, t: i32, intro: f32) {
    if intro < 1.0 && ((t >> 1) % 5) as f32 > intro * 5.0 {
        return;
    }
    if b.state == BState::Tele && b.atk == Atk::Teleport && t % 3 == 0 {
        return;
    }
    let dark = b.kind == BKind::DarkSorcerer;
    let bob = ((t as f32) * 0.08).sin() * 3.0;
    let y = y + bob as i32;
    let el = b.el;
    let robe = mix(el.main(), rgb(if dark { 0x100008 } else { 0x201030 }), if dark { 0.6 } else { 0.4 }) & 0xFFFFFF;
    let sc = if dark { 1.3 } else { 1.0 };
    // Robe.
    for row in 0..(24.0 * sc) as i32 {
        let w = (4.0 + row as f32 * 0.55) as i32;
        let c = if row > (20.0 * sc) as i32 { el.light() & 0xFFFFFF } else { robe };
        p.fill(x - w, y - 8 + row, w * 2, 1, c);
    }
    // Hood and face.
    let hr = (6.0 * sc) as i32;
    p.disc(x, y - 12, hr, if dark { 0x100008 } else { 0x201030 });
    p.disc(x, y - 11, hr - 2, 0x080410);
    let eye = el.light() & 0xFFFFFF;
    p.glow(x, y - 11, 5, eye, 0.35);
    p.fill(x - 3, y - 12, 2, 1, eye);
    p.fill(x + 1, y - 12, 2, 1, eye);
    if dark {
        p.line(x - 5, y - 16, x - 9, y - 24, 0xfcfcfc);
        p.line(x + 5, y - 16, x + 9, y - 24, 0xfcfcfc);
    }
    // Staff and orb.
    let (sx, sy) = (x + (10.0 * sc) as i32, y - 20);
    p.line(sx, sy, sx, y + 16, 0x8c5020);
    let charging = b.state == BState::Tele;
    let orb_r = if charging { 3 + (b.st_max - b.st_t) / 6 } else { 3 };
    p.glow(sx, sy - 2, orb_r + 4, eye, 0.4);
    p.disc(sx, sy - 2, orb_r, eye);
    p.fill(sx - 1, sy - 3, 1, 1, 0xfcfcfc);
}
