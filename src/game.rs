// ELEMENTAL LEGENDS - game rules, entities and scene drawing.
use crate::audio::{Sfx, Song, Synth, SFX_COUNT};
use crate::gfx::*;
use crate::sprites::*;
use crate::world::*;
use sdl2::audio::AudioDevice;
use std::f32::consts::PI;

pub const GATE_X: f32 = 128.0;
pub const GATE_Y: f32 = (HUD + 6 * TS + 8) as f32;
const WF: f32 = W as f32;
const HF: f32 = H as f32;
const HUDF: f32 = HUD as f32;
const SPAWN_Y: f32 = GATE_Y + 34.0;

#[allow(clippy::too_many_arguments)]
fn hit(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    (ax - bx).abs() * 2.0 < aw + bw && (ay - by).abs() * 2.0 < ah + bh
}
fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    (bx - ax).hypot(by - ay)
}
fn ang(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    (by - ay).atan2(bx - ax)
}

// ---------------------------------------------------------------- input
#[derive(Clone, Copy)]
pub enum Btn {
    Up = 0,
    Down,
    Left,
    Right,
    Fire,
    Sub,
    Cycle,
    Start,
    Mute,
}

#[derive(Clone, Copy, Default)]
pub struct Input {
    keys: [bool; 9],
    pad: [bool; 9],
    axis: [bool; 9],
    pressed: [bool; 9],
}
impl Input {
    pub fn held(&self, b: Btn) -> bool {
        let i = b as usize;
        self.keys[i] || self.pad[i] || self.axis[i]
    }
    pub fn hit(&self, b: Btn) -> bool {
        self.pressed[b as usize]
    }
    fn set(&mut self, i: usize, down: bool, src: u8) {
        let was = self.keys[i] || self.pad[i] || self.axis[i];
        match src {
            0 => self.keys[i] = down,
            1 => self.pad[i] = down,
            _ => self.axis[i] = down,
        }
        if down && !was {
            self.pressed[i] = true;
        }
    }
    pub fn set_key(&mut self, b: Btn, d: bool) {
        self.set(b as usize, d, 0)
    }
    pub fn set_pad(&mut self, b: Btn, d: bool) {
        self.set(b as usize, d, 1)
    }
    pub fn set_axis(&mut self, b: Btn, d: bool) {
        self.set(b as usize, d, 2)
    }
    pub fn clear_pressed(&mut self) {
        self.pressed = [false; 9];
    }
    pub fn release_all(&mut self) {
        *self = Input::default();
    }
}

// ---------------------------------------------------------------- rng
struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + self.f() * (b - a)
    }
    fn irange(&mut self, a: i32, b: i32) -> i32 {
        a + (self.f() * (b - a + 1) as f32) as i32
    }
    fn pick<T: Copy>(&mut self, a: &[T]) -> T {
        a[(self.f() * a.len() as f32) as usize % a.len()]
    }
}

// ---------------------------------------------------------------- elements
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Elem {
    Fire,
    Ice,
    Storm,
    Earth,
    Neutral,
}
impl Elem {
    fn idx(self) -> usize {
        self as usize
    }
    fn from_idx(i: usize) -> Elem {
        match i % 5 {
            0 => Elem::Fire,
            1 => Elem::Ice,
            2 => Elem::Storm,
            3 => Elem::Earth,
            _ => Elem::Neutral,
        }
    }
    /// Each element is strong against its opposite: fire<->ice, storm<->earth.
    fn opposite(self) -> Elem {
        match self {
            Elem::Fire => Elem::Ice,
            Elem::Ice => Elem::Fire,
            Elem::Storm => Elem::Earth,
            Elem::Earth => Elem::Storm,
            Elem::Neutral => Elem::Neutral,
        }
    }
    fn name(self) -> &'static str {
        ["FIRE", "ICE", "STORM", "EARTH", "NONE"][self.idx()]
    }
    fn light(self) -> u32 {
        EL_LIGHT[self.idx()]
    }
    fn main(self) -> u32 {
        EL_MAIN[self.idx()]
    }
}
/// Damage multiplier: 2x against the target's weakness, 0.5x against its own element.
fn mult(att: Elem, tgt: Elem) -> f32 {
    if tgt == Elem::Neutral || att == Elem::Neutral {
        1.0
    } else if att == tgt {
        0.5
    } else if att == tgt.opposite() {
        2.0
    } else {
        1.0
    }
}
/// (speed, damage, radius, cooldown, life)
fn bolt_stats(e: Elem) -> (f32, f32, f32, i32, i32) {
    match e {
        Elem::Fire => (3.6, 2.0, 3.0, 16, 70),
        Elem::Ice => (4.6, 1.5, 3.0, 11, 60),
        Elem::Storm => (7.0, 1.3, 3.0, 13, 40),
        _ => (3.0, 3.0, 4.0, 22, 70),
    }
}
fn spell_cost(e: Elem) -> f32 {
    match e {
        Elem::Fire => 18.0,
        Elem::Ice => 22.0,
        Elem::Storm => 18.0,
        _ => 24.0,
    }
}
const SPELL_NAMES: [&str; 4] = ["FLAME RING", "FROST NOVA", "CHAIN BOLT", "QUAKE"];
const REGION: [Elem; 4] = [Elem::Earth, Elem::Ice, Elem::Storm, Elem::Fire];

// ---------------------------------------------------------------- entities
#[derive(Clone, Copy)]
struct Player {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fx: f32,
    fy: f32,
    dir: u8,
    walk: i32,
    cd: i32,
    scd: i32,
    inv: i32,
    cast: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EK {
    Slime = 0,
    Bat,
    Skeleton,
    Imp,
    Ghost,
    Golem,
    Generator,
}
const POOLS: [&[EK]; 4] = [
    &[EK::Slime, EK::Slime, EK::Bat, EK::Skeleton],
    &[EK::Skeleton, EK::Ghost, EK::Slime, EK::Bat],
    &[EK::Slime, EK::Bat, EK::Imp, EK::Golem, EK::Skeleton],
    &[EK::Imp, EK::Golem, EK::Ghost, EK::Slime, EK::Skeleton],
];

#[derive(Clone)]
struct Enemy {
    id: u32,
    k: EK,
    el: Elem,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    hp: f32,
    spd: f32,
    touch: i32,
    t: i32,
    turn: i32,
    vx: f32,
    vy: f32,
    flash: i32,
    spawn: i32,
    rate: i32,
    lvl: i32,
    dead: bool,
    slow: i32,
    frozen: i32,
    gen_id: u32,
    kbx: f32,
    kby: f32,
    tip: i32,
}

struct Bullet {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
    dmg: f32,
    el: Elem,
    life: i32,
    pierce: Option<Vec<u32>>,
    dead: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum IK {
    Coin,
    Gem,
    Apple,
    Bread,
    Meat,
    Potion,
    Heart,
    Orb,
}
struct Item {
    kind: IK,
    x: f32,
    y: f32,
    val: i32,
    el: Elem,
    life: i32,
    dead: bool,
}

enum PK {
    Dot,
    Ring,
    Text(String),
    Line(f32, f32),
}
struct Part {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: i32,
    max: i32,
    c: u32,
    sz: i32,
    kind: PK,
}

struct Scroll {
    d: usize,
    t: f32,
    from: usize,
    to: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Pat {
    Aim,
    Ring,
    Spiral,
    Spiral2,
    Summon,
    Rain,
}

struct Boss {
    name: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    hp: f32,
    max: f32,
    t: i32,
    t0: i32,
    pat: usize,
    pat_t: i32,
    flash: i32,
    tip: i32,
    slow: i32,
    alive: bool,
    entering: bool,
    gone: bool,
    lv: usize,
    el: Elem,
    shifting: bool,
    sprs: Vec<Spr>,
    sc: i32,
    pats: &'static [Pat],
}

/// (name, element, shifts element, hp, attack patterns)
fn boss_def(n: usize) -> (&'static str, Elem, bool, f32, &'static [Pat]) {
    match n {
        1 => ("STONE TROLL", Elem::Earth, false, 60.0, &[Pat::Aim, Pat::Rain]),
        2 => ("FROST WYRM", Elem::Ice, false, 90.0, &[Pat::Aim, Pat::Spiral, Pat::Ring]),
        3 => ("THUNDER ROC", Elem::Storm, false, 120.0, &[Pat::Summon, Pat::Aim, Pat::Ring, Pat::Spiral]),
        4 => ("FLAME LORD", Elem::Fire, false, 150.0, &[Pat::Rain, Pat::Aim, Pat::Spiral, Pat::Ring]),
        5 => ("CHIMERA", Elem::Fire, true, 190.0, &[Pat::Aim, Pat::Spiral, Pat::Summon, Pat::Rain, Pat::Ring]),
        _ => (
            "DARK SORCERER",
            Elem::Storm,
            true,
            320.0,
            &[Pat::Spiral, Pat::Aim, Pat::Rain, Pat::Ring, Pat::Summon, Pat::Spiral2],
        ),
    }
}
const REWARDS: [&str; 6] =
    ["", "SPELL POWER UP", "MANA CRYSTAL: MAX MP UP", "HEART CONTAINER", "SPELL POWER UP", "WINGED BOOTS: SPEED UP"];

// ---------------------------------------------------------------- save data
#[derive(Clone)]
pub struct SaveData {
    max_hp: i32,
    hp: i32,
    max_mp: i32,
    mp: f32,
    food: f32,
    gold: i32,
    el: usize,
    spell_lv: i32,
    speed: f32,
    cleared: [bool; 7],
    tanks: Vec<usize>,
    caches: Vec<usize>,
    opened: Vec<usize>,
    visited: Vec<usize>,
    room: usize,
    time: u64,
    heart_price: i32,
}

impl SaveData {
    fn fresh() -> Self {
        SaveData {
            max_hp: 20,
            hp: 20,
            max_mp: 40,
            mp: 40.0,
            food: 100.0,
            gold: 30,
            el: 0,
            spell_lv: 1,
            speed: 1.5,
            cleared: [false; 7],
            tanks: vec![],
            caches: vec![],
            opened: vec![],
            visited: vec![],
            room: START_Y * WW + START_X,
            time: 0,
            heart_price: 150,
        }
    }
    fn to_text(&self) -> String {
        let b = |v: &[bool]| v.iter().map(|x| if *x { "1" } else { "0" }).collect::<Vec<_>>().join(",");
        let l = |v: &[usize]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        format!(
            "max_hp={}\nhp={}\nmax_mp={}\nmp={}\nfood={}\ngold={}\nel={}\nspell_lv={}\nspeed={}\ncleared={}\ntanks={}\ncaches={}\nopened={}\nvisited={}\nroom={}\ntime={}\nheart_price={}\n",
            self.max_hp, self.hp, self.max_mp, self.mp, self.food, self.gold, self.el, self.spell_lv, self.speed,
            b(&self.cleared), l(&self.tanks), l(&self.caches), l(&self.opened), l(&self.visited), self.room,
            self.time, self.heart_price
        )
    }
    fn from_text(txt: &str) -> Option<Self> {
        let mut s = Self::fresh();
        let mut ok = false;
        for line in txt.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim();
            let num = || v.parse::<f64>().unwrap_or(0.0);
            let list = || -> Vec<usize> { v.split(',').filter_map(|x| x.trim().parse().ok()).collect() };
            match k.trim() {
                "max_hp" => {
                    s.max_hp = num() as i32;
                    ok = true;
                }
                "hp" => s.hp = num() as i32,
                "max_mp" => s.max_mp = num() as i32,
                "mp" => s.mp = num() as f32,
                "food" => s.food = num() as f32,
                "gold" => s.gold = num() as i32,
                "el" => s.el = (num() as usize).min(3),
                "spell_lv" => s.spell_lv = (num() as i32).clamp(1, 3),
                "speed" => s.speed = (num() as f32).clamp(1.0, 2.5),
                "cleared" => {
                    for (i, x) in v.split(',').enumerate().take(7) {
                        s.cleared[i] = x.trim() == "1";
                    }
                }
                "tanks" => s.tanks = list(),
                "caches" => s.caches = list(),
                "opened" => s.opened = list(),
                "visited" => s.visited = list(),
                "room" => s.room = num() as usize,
                "time" => s.time = num() as u64,
                "heart_price" => s.heart_price = num() as i32,
                _ => {}
            }
        }
        if ok && s.max_hp > 0 {
            Some(s)
        } else {
            None
        }
    }
}
fn save_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("elementallegends.sav")))
        .unwrap_or_else(|| "elementallegends.sav".into())
}
fn load_save() -> Option<SaveData> {
    std::fs::read_to_string(save_path()).ok().and_then(|t| SaveData::from_text(&t))
}

// ---------------------------------------------------------------- game
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Title,
    Choose,
    Intro,
    Play,
    LairIn,
    Reward,
    Dying,
    GameOver,
    Victory,
}

struct Star {
    x: f32,
    y: f32,
    s: f32,
}

pub struct Game {
    mode: Mode,
    frame: u64,
    t: i32,
    menu: usize,
    rooms: Vec<Room>,
    start: usize,
    room: usize,
    in_lair: usize,
    arena: Option<Room>,
    pl: Player,
    enemies: Vec<Enemy>,
    pb: Vec<Bullet>,
    eb: Vec<Bullet>,
    items: Vec<Item>,
    parts: Vec<Part>,
    scroll: Option<Scroll>,
    msg: Option<(String, i32)>,
    flash: i32,
    shake: i32,
    paused: bool,
    boss: Option<Boss>,
    boss_dead: bool,
    clear_t: i32,
    lair_intro: i32,
    gate_n: usize,
    gate_room: usize,
    gate_warned: bool,
    stars: Vec<Star>,
    s: SaveData,
    rng: Rng,
    next_id: u32,
    spr: Sprites,
    themes: Vec<Theme>,
    audio: Option<AudioDevice<Synth>>,
    has_save: bool,
    last_sfx: [u64; SFX_COUNT],
    muted: bool,
    inp: Input,
    starve_t: i32,
    hungry_warned: bool,
    starving_warned: bool,
    shop_armed: [bool; 3],
    no_save: bool,
    pub quit: bool,
}

impl Game {
    pub fn new(audio: Option<AudioDevice<Synth>>) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9)
            | 1;
        let mut rng = Rng(seed);
        let stars = (0..70).map(|_| Star { x: rng.range(0.0, WF), y: rng.range(0.0, HF), s: rng.range(0.2, 1.6) }).collect();
        let mut g = Game {
            mode: Mode::Title,
            frame: 0,
            t: 0,
            menu: 0,
            rooms: vec![],
            start: 0,
            room: 0,
            in_lair: 0,
            arena: None,
            pl: Player { x: 128.0, y: 160.0, w: 10.0, h: 12.0, fx: 0.0, fy: -1.0, dir: b'u', walk: 0, cd: 0, scd: 0, inv: 0, cast: 0 },
            enemies: vec![],
            pb: vec![],
            eb: vec![],
            items: vec![],
            parts: vec![],
            scroll: None,
            msg: None,
            flash: 0,
            shake: 0,
            paused: false,
            boss: None,
            boss_dead: false,
            clear_t: 0,
            lair_intro: 0,
            gate_n: 0,
            gate_room: 0,
            gate_warned: false,
            stars,
            s: SaveData::fresh(),
            rng,
            next_id: 1,
            spr: Sprites::new(),
            themes: build_themes(),
            audio,
            has_save: load_save().is_some(),
            last_sfx: [0; SFX_COUNT],
            muted: false,
            inp: Input::default(),
            starve_t: 0,
            hungry_warned: false,
            starving_warned: false,
            shop_armed: [true; 3],
            no_save: false,
            quit: false,
        };
        g.play_song(Some(Song::Title));
        g
    }

    fn held(&self, b: Btn) -> bool {
        self.inp.held(b)
    }
    fn p(&self, b: Btn) -> bool {
        self.inp.hit(b)
    }
    fn sfx(&mut self, s: Sfx) {
        let i = s as usize;
        if self.frame < self.last_sfx[i] + 2 {
            return;
        }
        self.last_sfx[i] = self.frame;
        if let Some(a) = self.audio.as_mut() {
            a.lock().sfx(s);
        }
    }
    fn play_song(&mut self, s: Option<Song>) {
        if let Some(a) = self.audio.as_mut() {
            a.lock().play_song(s);
        }
    }
    fn save(&mut self) {
        if self.no_save {
            return;
        }
        self.s.visited = self.rooms.iter().filter(|r| r.visited).map(|r| r.i).collect();
        if std::fs::write(save_path(), self.s.to_text()).is_ok() {
            self.has_save = true;
        }
    }
    fn setup_world(&mut self) {
        let (rooms, start) = gen_world(&self.themes);
        self.rooms = rooms;
        self.start = start;
        for &i in &self.s.visited {
            if let Some(r) = self.rooms.get_mut(i) {
                r.visited = true;
            }
        }
    }
    fn el(&self) -> Elem {
        Elem::from_idx(self.s.el)
    }
    fn cur_room(&self) -> &Room {
        match (&self.arena, self.in_lair) {
            (Some(a), n) if n > 0 => a,
            _ => &self.rooms[self.room],
        }
    }

    // ------------------------------------------------------------ collision
    fn solid_at(&self, x: f32, y: f32) -> bool {
        let c = (x / TS as f32).floor() as i32;
        let r = ((y - HUDF) / TS as f32).floor() as i32;
        if c < 0 || c >= RC as i32 || r < 0 || r >= RR as i32 {
            return false;
        }
        self.cur_room().tiles[r as usize][c as usize] > 0
    }
    fn box_solid(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let (x0, x1, y0, y1) = (x - w / 2.0, x + w / 2.0 - 0.01, y - h / 2.0, y + h / 2.0 - 0.01);
        self.solid_at(x0, y0) || self.solid_at(x1, y0) || self.solid_at(x0, y1) || self.solid_at(x1, y1)
    }
    #[allow(clippy::too_many_arguments)]
    fn move_box(&self, x: &mut f32, y: &mut f32, w: f32, h: f32, dx: f32, dy: f32) -> (bool, bool) {
        let (mut hx, mut hy) = (false, false);
        if dx != 0.0 {
            *x += dx;
            if self.box_solid(*x, *y, w, h) {
                *x -= dx;
                hx = true;
            }
        }
        if dy != 0.0 {
            *y += dy;
            if self.box_solid(*x, *y, w, h) {
                *y -= dy;
                hy = true;
            }
        }
        (hx, hy)
    }
    /// Player movement with corner-sliding so doorways are easy to slip into.
    fn move_player(&self, p: &mut Player, dx: f32, dy: f32) {
        let (hx, hy) = self.move_box(&mut p.x, &mut p.y, p.w, p.h, dx, dy);
        let nudge = |g: &Game, p: &mut Player, along_y: bool, d: f32| {
            for k in 1..=6 {
                for s in [1.0f32, -1.0] {
                    let (tx, ty) = if along_y { (p.x + d, p.y + s * k as f32) } else { (p.x + s * k as f32, p.y + d) };
                    if !g.box_solid(tx, ty, p.w, p.h) {
                        if along_y {
                            g.move_box(&mut p.x, &mut p.y, p.w, p.h, 0.0, s);
                        } else {
                            g.move_box(&mut p.x, &mut p.y, p.w, p.h, s, 0.0);
                        }
                        return;
                    }
                }
            }
        };
        if hx && dy == 0.0 {
            nudge(self, p, true, dx);
        }
        if hy && dx == 0.0 {
            nudge(self, p, false, dy);
        }
    }

    // ------------------------------------------------------------ effects
    fn boom(&mut self, x: f32, y: f32, n: usize, el: Elem) {
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
            self.parts.push(Part { x, y, vx: a.cos() * s, vy: a.sin() * s, life, max: life, c, sz, kind: PK::Dot });
        }
        self.parts.push(Part { x, y, vx: 0.0, vy: 0.0, life: 12, max: 12, c: WHITE, sz: 1, kind: PK::Ring });
    }
    fn spark(&mut self, x: f32, y: f32, c: u32) {
        for _ in 0..3 {
            let (vx, vy) = (self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0));
            self.parts.push(Part { x, y, vx, vy, life: 8, max: 8, c, sz: 1, kind: PK::Dot });
        }
    }
    fn float(&mut self, s: impl Into<String>, x: f32, y: f32, c: u32) {
        self.parts.push(Part { x, y, vx: 0.0, vy: -0.4, life: 50, max: 50, c, sz: 1, kind: PK::Text(s.into()) });
    }
    fn show_msg(&mut self, s: impl Into<String>) {
        self.msg = Some((s.into(), 200));
    }
    fn update_parts(&mut self) {
        for p in self.parts.iter_mut() {
            p.x += p.vx;
            p.y += p.vy;
            if matches!(p.kind, PK::Dot) {
                p.vx *= 0.95;
                p.vy *= 0.95;
            }
            p.life -= 1;
        }
        self.parts.retain(|p| p.life > 0);
    }
    fn update_stars(&mut self, sp: f32) {
        for i in 0..self.stars.len() {
            self.stars[i].y += self.stars[i].s * sp;
            if self.stars[i].y > HF {
                self.stars[i].y -= HF;
                self.stars[i].x = self.rng.range(0.0, WF);
            }
        }
    }

    // ------------------------------------------------------------ main update
    pub fn update(&mut self, inp: &Input) {
        self.inp = *inp;
        self.frame += 1;
        if self.p(Btn::Mute) {
            self.muted = !self.muted;
            let m = self.muted;
            if let Some(a) = self.audio.as_mut() {
                a.lock().muted = m;
            }
        }
        match self.mode {
            Mode::Title => self.update_title(),
            Mode::Choose => self.update_choose(),
            Mode::Intro => self.update_intro(),
            Mode::Play => self.update_play(),
            Mode::LairIn => self.update_lairin(),
            Mode::Reward => self.update_reward(),
            Mode::Dying => self.update_dying(),
            Mode::GameOver => self.update_gameover(),
            Mode::Victory => self.update_victory(),
        }
        if self.flash > 0 {
            self.flash -= 1;
        }
        if self.shake > 0 {
            self.shake -= 1;
        }
    }

    fn menu_opts(&self) -> Vec<&'static str> {
        let mut v = vec![];
        if self.has_save {
            v.push("CONTINUE");
        }
        v.push("NEW GAME");
        v.push("QUIT");
        v
    }
    fn update_title(&mut self) {
        self.update_stars(0.6);
        let opts = self.menu_opts();
        if self.menu >= opts.len() {
            self.menu = 0;
        }
        if self.p(Btn::Up) {
            self.menu = (self.menu + opts.len() - 1) % opts.len();
            self.sfx(Sfx::Select);
        }
        if self.p(Btn::Down) {
            self.menu = (self.menu + 1) % opts.len();
            self.sfx(Sfx::Select);
        }
        if self.p(Btn::Start) || self.p(Btn::Fire) {
            self.sfx(Sfx::Select);
            match opts[self.menu] {
                "CONTINUE" => self.continue_game(),
                "NEW GAME" => {
                    self.s = SaveData::fresh();
                    self.setup_world();
                    self.mode = Mode::Choose;
                    self.menu = 0;
                }
                _ => self.quit = true,
            }
        }
    }
    fn update_choose(&mut self) {
        self.update_stars(0.6);
        if self.p(Btn::Left) || self.p(Btn::Up) {
            self.menu = (self.menu + 3) % 4;
            self.sfx(Sfx::Select);
        }
        if self.p(Btn::Right) || self.p(Btn::Down) {
            self.menu = (self.menu + 1) % 4;
            self.sfx(Sfx::Select);
        }
        if self.p(Btn::Start) || self.p(Btn::Fire) {
            self.s.el = self.menu;
            self.sfx(Sfx::Spell);
            self.mode = Mode::Intro;
            self.t = 0;
        }
    }
    fn continue_game(&mut self) {
        self.s = load_save().unwrap_or_else(SaveData::fresh);
        self.setup_world();
        self.s.hp = self.s.max_hp;
        self.s.food = self.s.food.max(50.0);
        let r = if self.s.room < self.rooms.len() { self.s.room } else { self.start };
        self.go_play(r, GATE_X, SPAWN_Y, false);
    }
    fn update_intro(&mut self) {
        self.t += 1;
        self.update_stars(0.4);
        let total = INTRO.iter().map(|l| l.len()).sum::<usize>() as i32 * 2 + 120;
        if self.p(Btn::Start) || self.p(Btn::Fire) || self.t > total {
            let s = self.start;
            self.go_play(s, GATE_X, SPAWN_Y, false);
        }
    }

    // ------------------------------------------------------------ overworld
    fn go_play(&mut self, room: usize, x: f32, y: f32, spawn: bool) {
        self.mode = Mode::Play;
        self.paused = false;
        self.in_lair = 0;
        self.arena = None;
        self.boss = None;
        self.boss_dead = false;
        self.room = room;
        self.scroll = None;
        self.pl = Player { x, y, w: 10.0, h: 12.0, fx: 0.0, fy: -1.0, dir: b'u', walk: 0, cd: 0, scd: 10, inv: 30, cast: 0 };
        self.enemies.clear();
        self.pb.clear();
        self.eb.clear();
        self.items.clear();
        self.parts.clear();
        self.msg = None;
        self.shop_armed = [true; 3];
        self.enter_room(spawn);
        self.play_song(Some(Song::Field));
    }
    fn enter_room(&mut self, spawn: bool) {
        let r = self.room;
        self.rooms[r].visited = true;
        self.s.room = r;
        self.gate_warned = false;
        if spawn {
            self.spawn_room_enemies();
        }
        if r == self.start {
            self.show_msg("WELCOME, MAGE! SPEND YOUR GOLD ON FOOD AND POTIONS.");
            if let Some(m) = self.msg.as_mut() {
                m.1 = 150;
            }
        }
        self.save();
    }
    fn next_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }
    fn make_enemy(&mut self, k: EK, x: f32, y: f32, theme: usize) -> Enemy {
        let (hp, spd, w, h, touch) = match k {
            EK::Slime => (3.0, 0.5, 12.0, 9.0, 2),
            EK::Bat => (2.0, 1.2, 12.0, 6.0, 2),
            EK::Skeleton => (4.0, 0.7, 10.0, 12.0, 3),
            EK::Imp => (3.0, 0.6, 10.0, 10.0, 2),
            EK::Ghost => (3.0, 0.55, 12.0, 10.0, 3),
            EK::Golem => (9.0, 0.35, 14.0, 12.0, 4),
            EK::Generator => (8.0, 0.0, 16.0, 12.0, 2),
        };
        let th = theme.min(3);
        let hp = hp + th as f32 * if k == EK::Golem { 2.0 } else if k == EK::Generator { 3.0 } else { 1.0 };
        let el = match k {
            EK::Slime => REGION[th],
            EK::Bat => Elem::Storm,
            EK::Skeleton | EK::Generator => Elem::Neutral,
            EK::Imp => Elem::Fire,
            EK::Ghost => Elem::Ice,
            EK::Golem => Elem::Earth,
        };
        let rate = match k {
            EK::Imp => 110 - th as i32 * 10,
            EK::Skeleton => 150,
            EK::Generator => (170 - th as i32 * 25).max(90),
            _ => 100,
        };
        let id = self.next_id();
        let t = self.rng.irange(0, 60);
        Enemy {
            id, k, el, x, y, w, h, hp, spd: spd + th as f32 * 0.08, touch, t, turn: 0, vx: 0.0, vy: 0.0, flash: 0,
            spawn: 30, rate, lvl: th as i32, dead: false, slow: 0, frozen: 0, gen_id: 0, kbx: 0.0, kby: 0.0, tip: 0,
        }
    }
    fn free_spot(&mut self, min_d: f32, special: bool) -> Option<(f32, f32)> {
        for _ in 0..40 {
            let c = self.rng.irange(1, RC as i32 - 2) as usize;
            let r = self.rng.irange(1, RR as i32 - 2) as usize;
            if self.cur_room().tiles[r][c] > 0 {
                continue;
            }
            let x = (c as i32 * TS + 8) as f32;
            let y = (HUD + r as i32 * TS + 8) as f32;
            if dist(x, y, self.pl.x, self.pl.y) < min_d {
                continue;
            }
            if special && (x - GATE_X).abs() < 24.0 && (y - GATE_Y).abs() < 24.0 {
                continue;
            }
            return Some((x, y));
        }
        None
    }
    fn spawn_room_enemies(&mut self) {
        if self.in_lair > 0 || self.room == self.start {
            return;
        }
        let (th, special, gate) = {
            let r = &self.rooms[self.room];
            (r.theme, r.gate > 0 || r.tank || r.cache || r.shrine.is_some(), r.gate > 0)
        };
        let n = 2 + th as i32 + self.rng.irange(0, 1) - gate as i32;
        for _ in 0..n {
            if let Some((x, y)) = self.free_spot(80.0, special) {
                let k = self.rng.pick(POOLS[th]);
                let e = self.make_enemy(k, x, y, th);
                self.enemies.push(e);
            }
        }
        if self.rng.f() < 0.3 + th as f32 * 0.1 {
            if let Some((x, y)) = self.free_spot(90.0, special) {
                let mut g = self.make_enemy(EK::Generator, x, y, th);
                g.spawn = 0;
                self.enemies.push(g);
            }
        }
    }
    fn start_scroll(&mut self, d: usize) {
        let r = self.room;
        let (rx, ry) = (self.rooms[r].x as i32 + DIRS[d].0, self.rooms[r].y as i32 + DIRS[d].1);
        if !self.rooms[r].doors[d] || rx < 0 || ry < 0 || rx >= WW as i32 || ry >= WH as i32 {
            return;
        }
        let to = ry as usize * WW + rx as usize;
        self.scroll = Some(Scroll { d, t: 0.0, from: r, to });
        self.pb.clear();
        self.eb.clear();
        self.enemies.clear();
        self.items.clear();
        self.parts.clear();
        match d {
            0 => self.pl.y = HF - 10.0,
            1 => self.pl.y = HUDF + 10.0,
            2 => self.pl.x = 10.0,
            _ => self.pl.x = WF - 10.0,
        }
    }

    fn update_needs(&mut self) {
        self.s.food = (self.s.food - 1.0 / 72.0).max(0.0);
        self.s.mp = (self.s.mp + 1.0 / 30.0).min(self.s.max_mp as f32);
        if self.s.food <= 0.0 {
            self.starve_t += 1;
            if !self.starving_warned {
                self.starving_warned = true;
                self.show_msg("YOU ARE STARVING! FIND FOOD FAST.");
            }
            if self.starve_t >= 90 {
                self.starve_t = 0;
                self.s.hp -= 1;
                let (x, y) = (self.pl.x, self.pl.y);
                self.float("-1", x - 6.0, y - 16.0, rgb(0xfc7460));
                self.sfx(Sfx::Hurt);
                if self.s.hp <= 0 {
                    self.die();
                }
            }
        } else {
            self.starve_t = 0;
            if self.s.food > 25.0 {
                self.hungry_warned = false;
                self.starving_warned = false;
            } else if !self.hungry_warned {
                self.hungry_warned = true;
                self.show_msg("YOUR STOMACH GROWLS. EAT SOMETHING SOON.");
            }
        }
    }

    fn update_play(&mut self) {
        if self.p(Btn::Start) {
            self.paused = !self.paused;
            self.sfx(Sfx::Select);
        }
        if self.paused {
            return;
        }
        self.s.time += 1;
        if let Some(sc) = self.scroll.as_mut() {
            sc.t += 1.0 / 36.0;
            if sc.t >= 1.0 {
                let to = sc.to;
                self.scroll = None;
                self.room = to;
                self.enter_room(true);
            }
            return;
        }
        self.update_needs();
        if self.mode != Mode::Play {
            return;
        }
        let mut p = self.pl;
        if p.inv > 0 {
            p.inv -= 1;
        }
        if p.cast > 0 {
            p.cast -= 1;
        }
        let mut dx = (self.held(Btn::Right) as i32 - self.held(Btn::Left) as i32) as f32;
        let mut dy = (self.held(Btn::Down) as i32 - self.held(Btn::Up) as i32) as f32;
        if dx != 0.0 || dy != 0.0 {
            let l = dx.hypot(dy);
            dx /= l;
            dy /= l;
            // Holding CAST locks facing so the mage can strafe (Gauntlet style).
            if !self.held(Btn::Fire) {
                p.fx = dx;
                p.fy = dy;
                p.dir = if dx != 0.0 {
                    if dx > 0.0 { b'r' } else { b'l' }
                } else if dy > 0.0 {
                    b'd'
                } else {
                    b'u'
                };
            }
            p.walk += 1;
            let sp = self.s.speed;
            self.move_player(&mut p, dx * sp, dy * sp);
        }
        self.pl = p;
        if self.in_lair == 0 {
            let exit = if p.x < 4.0 {
                Some(3)
            } else if p.x > WF - 4.0 {
                Some(2)
            } else if p.y < HUDF + 4.0 {
                Some(0)
            } else if p.y > HF - 4.0 {
                Some(1)
            } else {
                None
            };
            if let Some(d) = exit {
                self.start_scroll(d);
                return;
            }
        }
        if self.pl.cd > 0 {
            self.pl.cd -= 1;
        }
        if self.pl.scd > 0 {
            self.pl.scd -= 1;
        }
        if self.held(Btn::Fire) && self.pl.cd <= 0 && !self.boss_dead && !self.on_pedestal() {
            self.cast_bolt();
        }
        if self.held(Btn::Sub) && self.pl.scd <= 0 && !self.boss_dead {
            self.cast_spell();
        }
        self.update_pbullets();
        self.update_enemies();
        self.update_boss();
        self.update_ebullets();
        self.update_items();
        self.update_parts();
        if self.mode == Mode::Play && self.in_lair == 0 {
            self.room_objects();
        }
        if self.in_lair > 0 && self.mode == Mode::Play {
            self.update_lair_state();
        }
        if let Some(m) = self.msg.as_mut() {
            m.1 -= 1;
            if m.1 <= 0 {
                self.msg = None;
            }
        }
    }

    // ------------------------------------------------------------ casting
    fn cast_bolt(&mut self) {
        let el = self.el();
        let (spd, dmg, r, cd, life) = bolt_stats(el);
        let p = self.pl;
        let a = p.fy.atan2(p.fx);
        let spread: &[f32] = match self.s.spell_lv {
            1 => &[0.0],
            2 => &[-0.12, 0.12],
            _ => &[-0.22, 0.0, 0.22],
        };
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
                pierce: if el == Elem::Storm { Some(vec![]) } else { None },
                dead: false,
            });
        }
        self.pl.cd = cd;
        self.pl.cast = 6;
        self.sfx(Sfx::Shoot);
    }
    fn cast_spell(&mut self) {
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
                        x: px, y: py - 2.0, vx: a.cos() * 3.0, vy: a.sin() * 3.0, r: 4.0, dmg: 2.5, el,
                        life: 60, pierce: None, dead: false,
                    });
                }
                self.flash = 6;
                self.sfx(Sfx::Spell);
            }
            Elem::Ice => {
                let mut en = std::mem::take(&mut self.enemies);
                for e in en.iter_mut() {
                    if !e.dead && e.spawn <= 0 {
                        e.frozen = 180;
                        self.damage_enemy(e, 2.0, el);
                    }
                }
                en.append(&mut self.enemies);
                self.enemies = en;
                if let Some(b) = self.boss.as_mut() {
                    b.slow = 180;
                }
                self.damage_boss(3.0, el);
                for r in 0..3 {
                    self.parts.push(Part {
                        x: px, y: py, vx: 0.0, vy: 0.0, life: 14 + r * 6, max: 14 + r * 6, c: el.light(), sz: 1,
                        kind: PK::Ring,
                    });
                }
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
                        if e.dead || e.spawn > 0 || hitlist.contains(&e.id) {
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
                    self.parts.push(Part { x: cx, y: cy, vx: 0.0, vy: 0.0, life: 12, max: 12, c: el.light(), sz: 1, kind: PK::Line(ex, ey) });
                    self.damage_enemy(&mut en[i], 4.0, el);
                    cx = ex;
                    cy = ey;
                }
                en.append(&mut self.enemies);
                self.enemies = en;
                if let Some((bx, by)) = self.boss.as_ref().filter(|b| b.alive && !b.entering).map(|b| (b.x, b.y)) {
                    self.parts.push(Part { x: px, y: py, vx: 0.0, vy: 0.0, life: 12, max: 12, c: el.light(), sz: 1, kind: PK::Line(bx, by) });
                    self.damage_boss(5.0, el);
                }
                self.flash = 4;
                self.sfx(Sfx::Zap);
            }
            _ => {
                let mut en = std::mem::take(&mut self.enemies);
                for e in en.iter_mut() {
                    if !e.dead && e.spawn <= 0 {
                        e.frozen = e.frozen.max(50);
                        self.damage_enemy(e, 3.0, el);
                    }
                }
                en.append(&mut self.enemies);
                self.enemies = en;
                self.damage_boss(5.0, el);
                for _ in 0..24 {
                    let (x, y) = (self.rng.range(16.0, WF - 16.0), self.rng.range(HUDF + 16.0, HF - 16.0));
                    let c = self.rng.pick(&[rgb(0x8c5020), rgb(0x98d858), rgb(0x5c3c10)]);
                    self.parts.push(Part { x, y, vx: 0.0, vy: -1.0, life: 20, max: 20, c, sz: 3, kind: PK::Dot });
                }
                self.shake = 24;
                self.sfx(Sfx::Quake);
            }
        }
    }

    // ------------------------------------------------------------ damage
    fn damage_enemy(&mut self, e: &mut Enemy, dmg: f32, el: Elem) {
        if e.dead {
            return;
        }
        let m = mult(el, e.el);
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
            e.dead = true;
            self.boom(e.x, e.y, 12, e.el);
            self.sfx(Sfx::Boom);
            self.drop_from(e);
        } else {
            self.sfx(if m > 1.5 { Sfx::Weak } else { Sfx::Hit });
        }
    }
    fn damage_boss(&mut self, dmg: f32, el: Elem) {
        let (m, bx, by, tip, dead) = {
            let Some(b) = self.boss.as_mut() else { return };
            if !b.alive || b.entering {
                return;
            }
            let m = mult(el, b.el);
            b.hp -= dmg * m;
            b.flash = 4;
            let tip = b.tip <= 0 && m != 1.0;
            if tip {
                b.tip = 30;
            }
            let dead = b.hp <= 0.0;
            if dead {
                b.hp = 0.0;
                b.alive = false;
            }
            (m, b.x, b.y, tip, dead)
        };
        if tip {
            if m > 1.0 {
                self.float("WEAK!", bx - 20.0, by - 40.0, rgb(0xfce040));
            } else {
                self.float("RESIST", bx - 24.0, by - 40.0, rgb(0x747474));
            }
        }
        self.sfx(if m > 1.5 { Sfx::Weak } else { Sfx::Hit });
        if dead {
            self.boss_dead = true;
            self.clear_t = 0;
            self.eb.clear();
            self.play_song(None);
        }
    }
    fn hurt(&mut self, d: i32) {
        if self.pl.inv > 0 || self.mode != Mode::Play {
            return;
        }
        self.s.hp -= d;
        self.pl.inv = 60;
        self.shake = 8;
        self.sfx(Sfx::Hurt);
        if self.s.hp <= 0 {
            self.die();
        }
    }
    fn die(&mut self) {
        self.s.hp = 0;
        self.mode = Mode::Dying;
        self.t = 0;
        let (x, y) = (self.pl.x, self.pl.y);
        let el = self.el();
        self.boom(x, y, 40, el);
        self.boom(x, y, 20, Elem::Neutral);
        self.sfx(Sfx::BigBoom);
        self.play_song(None);
        self.shake = 20;
    }

    // ------------------------------------------------------------ items
    fn add_item(&mut self, kind: IK, x: f32, y: f32, val: i32, el: Elem) {
        self.items.push(Item { kind, x, y, val, el, life: 600, dead: false });
    }
    fn drop_from(&mut self, e: &Enemy) {
        let th = e.lvl;
        if e.k == EK::Generator {
            for _ in 0..3 {
                let (ox, oy) = (self.rng.range(-10.0, 10.0), self.rng.range(-8.0, 8.0));
                self.add_item(IK::Coin, e.x + ox, e.y + oy, 5 * (th + 1), Elem::Neutral);
            }
            if self.rng.f() < 0.4 {
                self.add_item(IK::Meat, e.x, e.y, 0, Elem::Neutral);
            }
            return;
        }
        let r = self.rng.f();
        if r < 0.30 {
            let v = self.rng.irange(2, 4) + th * 2;
            self.add_item(IK::Coin, e.x, e.y, v, Elem::Neutral);
        } else if r < 0.39 {
            self.add_item(IK::Apple, e.x, e.y, 0, Elem::Neutral);
        } else if r < 0.43 {
            self.add_item(IK::Potion, e.x, e.y, 0, Elem::Neutral);
        } else if r < 0.47 {
            self.add_item(IK::Heart, e.x, e.y, 0, Elem::Neutral);
        }
        if e.el != Elem::Neutral && self.rng.f() < 0.05 {
            self.add_item(IK::Orb, e.x + 6.0, e.y, 0, e.el);
        }
    }
    fn eat(&mut self, amount: f32, heal: i32, label: &str) {
        self.s.food = (self.s.food + amount).min(100.0);
        self.s.hp = (self.s.hp + heal).min(self.s.max_hp);
        let (x, y) = (self.pl.x, self.pl.y);
        self.float(label.to_string(), x - label.len() as f32 * 4.0, y - 18.0, rgb(0xfcbc3c));
        self.sfx(Sfx::Eat);
    }
    fn set_element(&mut self, el: Elem) {
        self.s.el = el.idx();
        self.show_msg(format!("YOU NOW WIELD THE POWER OF {}! SPELL: {}.", el.name(), SPELL_NAMES[el.idx()]));
        let (x, y) = (self.pl.x, self.pl.y);
        self.boom(x, y, 24, el);
        self.sfx(Sfx::Heal);
    }
    fn collect(&mut self, it: &Item) {
        let (x, y) = (self.pl.x, self.pl.y);
        match it.kind {
            IK::Coin => {
                self.s.gold = (self.s.gold + it.val).min(9999);
                self.float(format!("+{}", it.val), x - 8.0, y - 18.0, rgb(0xfcbc3c));
                self.sfx(Sfx::Coin);
            }
            IK::Gem => {
                self.s.gold = (self.s.gold + 100).min(9999);
                self.float("+100", x - 16.0, y - 18.0, rgb(0xf878f8));
                self.sfx(Sfx::Chest);
            }
            IK::Apple => self.eat(15.0, 0, "APPLE"),
            IK::Bread => self.eat(30.0, 0, "BREAD"),
            IK::Meat => self.eat(50.0, 4, "ROAST"),
            IK::Potion => {
                self.s.mp = (self.s.mp + 30.0).min(self.s.max_mp as f32);
                self.float("MANA", x - 16.0, y - 18.0, rgb(0x3cbcfc));
                self.sfx(Sfx::Pickup);
            }
            IK::Heart => {
                self.s.hp = (self.s.hp + 2).min(self.s.max_hp);
                self.float("LIFE", x - 16.0, y - 18.0, rgb(0xfc7460));
                self.sfx(Sfx::Heal);
            }
            IK::Orb => {
                if it.el != self.el() {
                    self.set_element(it.el);
                } else {
                    self.s.mp = self.s.max_mp as f32;
                    self.float("MANA", x - 16.0, y - 18.0, rgb(0x3cbcfc));
                    self.sfx(Sfx::Heal);
                }
            }
        }
    }
    fn update_items(&mut self) {
        let (px, py) = (self.pl.x, self.pl.y);
        let mut items = std::mem::take(&mut self.items);
        for it in items.iter_mut() {
            it.life -= 1;
            if it.life <= 0 {
                it.dead = true;
            }
            if !it.dead && self.mode == Mode::Play && (it.x - px).abs() < 11.0 && (it.y - py).abs() < 12.0 {
                it.dead = true;
                self.collect(it);
            }
        }
        items.retain(|i| !i.dead);
        items.append(&mut self.items);
        self.items = items;
    }

    // ------------------------------------------------------------ bullets
    fn eshot(&mut self, x: f32, y: f32, a: f32, s: f32, el: Elem) {
        self.eb.push(Bullet {
            x, y, vx: a.cos() * s, vy: a.sin() * s, r: 2.0, dmg: 2.0, el, life: 500, pierce: None,
            dead: false,
        });
    }
    fn bolt_impact(&mut self, b: &Bullet, en: &mut [Enemy]) {
        if b.el == Elem::Fire {
            self.boom(b.x, b.y, 6, Elem::Fire);
            for e in en.iter_mut() {
                if !e.dead && e.spawn <= 0 && dist(b.x, b.y, e.x, e.y) < 18.0 {
                    self.damage_enemy(e, 1.0, Elem::Fire);
                }
            }
        } else {
            self.spark(b.x, b.y, b.el.light());
        }
    }
    fn update_pbullets(&mut self) {
        let mut pb = std::mem::take(&mut self.pb);
        let mut en = std::mem::take(&mut self.enemies);
        for b in pb.iter_mut() {
            b.x += b.vx;
            b.y += b.vy;
            b.life -= 1;
            if b.el == Elem::Fire && self.frame % 3 == 0 {
                let c = self.rng.pick(&[rgb(0xfc9838), rgb(0xd82800), rgb(0xfce040)]);
                self.parts.push(Part { x: b.x, y: b.y, vx: 0.0, vy: -0.3, life: 10, max: 10, c, sz: 1, kind: PK::Dot });
            }
            if b.life <= 0 || b.x < -8.0 || b.x > WF + 8.0 || b.y < HUDF - 8.0 || b.y > HF + 8.0 {
                b.dead = true;
                continue;
            }
            if self.solid_at(b.x, b.y) {
                b.dead = true;
                self.bolt_impact(b, &mut en);
                continue;
            }
            for i in 0..en.len() {
                let e = &en[i];
                if e.dead || e.spawn > 0 || !hit(b.x, b.y, b.r * 2.0, b.r * 2.0, e.x, e.y, e.w, e.h) {
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
                match b.el {
                    Elem::Ice => e.slow = 90,
                    Elem::Earth => {
                        let l = b.vx.hypot(b.vy).max(0.01);
                        e.kbx = b.vx / l * 3.0;
                        e.kby = b.vy / l * 3.0;
                    }
                    _ => {}
                }
                en[i] = e;
                if b.dead {
                    break;
                }
            }
            if b.dead {
                if b.el == Elem::Fire {
                    self.bolt_impact(b, &mut en);
                }
                continue;
            }
            let bb = self.boss.as_ref().filter(|x| x.alive && !x.entering).map(|x| (x.x, x.y, x.w, x.h));
            if let Some((bx, by, bw, bh)) = bb {
                if hit(b.x, b.y, b.r * 2.0, b.r * 2.0, bx, by, bw, bh) {
                    let already = b.pierce.as_ref().map_or(false, |p| p.contains(&u32::MAX));
                    if !already {
                        match b.pierce.as_mut() {
                            Some(p) => p.push(u32::MAX),
                            None => b.dead = true,
                        }
                        self.damage_boss(b.dmg, b.el);
                        if let Some(bs) = self.boss.as_mut() {
                            if b.el == Elem::Ice {
                                bs.slow = bs.slow.max(40);
                            }
                        }
                        self.spark(b.x, b.y, b.el.light());
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
    fn update_ebullets(&mut self) {
        let (px, py) = (self.pl.x, self.pl.y);
        let mut eb = std::mem::take(&mut self.eb);
        for b in eb.iter_mut() {
            b.x += b.vx;
            b.y += b.vy;
            b.life -= 1;
            if b.life <= 0 || b.x < -8.0 || b.x > WF + 8.0 || b.y < HUDF - 8.0 || b.y > HF + 8.0 || self.solid_at(b.x, b.y) {
                b.dead = true;
                continue;
            }
            if self.mode == Mode::Play && self.pl.inv <= 0 && hit(b.x, b.y, 4.0, 4.0, px, py, 6.0, 6.0) {
                b.dead = true;
                self.hurt(b.dmg as i32);
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
        if e.frozen > 0 {
            e.frozen -= 1;
            return;
        }
        let f = if e.slow > 0 {
            e.slow -= 1;
            0.4
        } else {
            1.0
        };
        let (px, py) = (self.pl.x, self.pl.y);
        let d = dist(e.x, e.y, px, py);
        let a = ang(e.x, e.y, px, py);
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
                if e.lvl >= 1 && e.t % e.rate == 0 && d < 150.0 {
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
                if e.t % e.rate == 0 {
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
                if e.lvl >= 3 && e.t % 170 == 0 {
                    self.eshot(e.x, e.y, a, 1.4, Elem::Ice);
                }
            }
            EK::Golem => {
                e.vx = a.cos() * e.spd;
                e.vy = a.sin() * e.spd;
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, e.vx * f, e.vy * f);
            }
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
        e.x = e.x.clamp(10.0, WF - 10.0);
        e.y = e.y.clamp(HUDF + 10.0, HF - 10.0);
    }
    fn update_enemies(&mut self) {
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
            if !e.dead && e.spawn <= 0 && hit(e.x, e.y, e.w, e.h, p.x, p.y, p.w, p.h) {
                self.hurt(e.touch);
            }
        }
        en.retain(|e| !e.dead);
        en.append(&mut self.enemies);
        self.enemies = en;
    }

    // ------------------------------------------------------------ room objects
    fn room_objects(&mut self) {
        let ri = self.room;
        let (px, py) = (self.pl.x, self.pl.y);
        let near = (px - GATE_X).abs() < 12.0 && (py - GATE_Y).abs() < 12.0;
        let (gate, tank, cache, chest, shrine) = {
            let r = &self.rooms[ri];
            (r.gate, r.tank, r.cache, r.chest, r.shrine)
        };
        if gate > 0 {
            if near {
                if !self.s.cleared[gate] {
                    if gate == 1 || self.s.cleared[gate - 1] {
                        self.enter_gate(gate);
                        return;
                    }
                    if !self.gate_warned {
                        self.gate_warned = true;
                        if gate == 6 {
                            self.show_msg("THE DARK TOWER IS SEALED. GATHER ALL FIVE RUNES.");
                        } else {
                            self.show_msg(format!("THIS LAIR IS SEALED. CONQUER LAIR {} FIRST.", gate - 1));
                        }
                        self.sfx(Sfx::Deny);
                    }
                }
            } else {
                self.gate_warned = false;
            }
        }
        if near && tank && !self.s.tanks.contains(&ri) {
            self.s.tanks.push(ri);
            self.s.max_hp += 4;
            self.s.hp = self.s.max_hp;
            self.show_msg("HEART CONTAINER! MAXIMUM LIFE UP.");
            self.sfx(Sfx::Fanfare);
            self.save();
        }
        if near && cache && !self.s.caches.contains(&ri) {
            self.s.caches.push(ri);
            self.s.gold = (self.s.gold + 100).min(9999);
            self.show_msg("A DRAGON'S HOARD! +100 GOLD.");
            self.sfx(Sfx::Chest);
            self.save();
        }
        if let Some((cx, cy, content, val)) = chest {
            if !self.s.opened.contains(&ri) && (px - cx).abs() < 12.0 && (py - cy).abs() < 12.0 {
                self.s.opened.push(ri);
                self.open_chest(cx, cy, content, val);
                self.save();
            }
        }
        if let Some(se) = shrine {
            if near && self.s.el != se {
                self.set_element(Elem::from_idx(se));
            }
        }
        if ri == self.start {
            self.shop();
        }
    }
    fn open_chest(&mut self, cx: f32, cy: f32, content: u8, val: i32) {
        self.sfx(Sfx::Chest);
        let (ix, iy) = (cx, cy + 14.0);
        match content {
            CH_GOLD => {
                self.s.gold = (self.s.gold + val).min(9999);
                self.float(format!("+{} GOLD", val), cx - 28.0, cy - 16.0, rgb(0xfcbc3c));
            }
            CH_BREAD => self.add_item(IK::Bread, ix, iy, 0, Elem::Neutral),
            CH_POTION => self.add_item(IK::Potion, ix, iy, 0, Elem::Neutral),
            CH_GEM => self.add_item(IK::Gem, ix, iy, 0, Elem::Neutral),
            _ => self.add_item(IK::Meat, ix, iy, 0, Elem::Neutral),
        }
        self.float("TREASURE!", cx - 36.0, cy - 28.0, WHITE);
    }
    fn on_pedestal(&self) -> bool {
        self.in_lair == 0
            && self.room == self.start
            && (0..3).any(|i| (self.pl.x - Self::shop_x(i)).abs() < 10.0 && (self.pl.y - GATE_Y).abs() < 12.0)
    }
    fn shop_x(i: usize) -> f32 {
        80.0 + i as f32 * 48.0
    }
    fn shop(&mut self) {
        let prices = [15, 25, self.s.heart_price];
        for i in 0..3 {
            let x = Self::shop_x(i);
            let near = (self.pl.x - x).abs() < 10.0 && (self.pl.y - GATE_Y).abs() < 12.0;
            if !near {
                self.shop_armed[i] = true;
                continue;
            }
            if self.shop_armed[i] {
                self.shop_armed[i] = false;
                let what = ["A HEARTY ROAST", "A MANA POTION", "A HEART CONTAINER"][i];
                self.show_msg(format!("{} FOR {} GOLD. PRESS A TO BUY.", what, prices[i]));
                if let Some(m) = self.msg.as_mut() {
                    m.1 = 120;
                }
            }
            if !self.p(Btn::Fire) {
                continue;
            }
            if self.s.gold < prices[i] {
                self.show_msg("NOT ENOUGH GOLD, MAGE. SLAY MONSTERS AND OPEN CHESTS.");
                self.sfx(Sfx::Deny);
                continue;
            }
            self.s.gold -= prices[i];
            match i {
                0 => {
                    self.eat(50.0, 4, "ROAST");
                    self.show_msg("A HEARTY MEAL! HUNGER SATED.");
                }
                1 => {
                    self.s.mp = self.s.max_mp as f32;
                    self.show_msg("MANA FULLY RESTORED.");
                    self.sfx(Sfx::Heal);
                }
                _ => {
                    self.s.max_hp += 4;
                    self.s.hp = self.s.max_hp;
                    self.s.heart_price += 100;
                    self.show_msg("YOUR LIFE GROWS STRONGER.");
                    self.sfx(Sfx::Fanfare);
                }
            }
            self.save();
        }
    }

    // ------------------------------------------------------------ lairs & bosses
    fn enter_gate(&mut self, n: usize) {
        self.mode = Mode::LairIn;
        self.t = 0;
        self.gate_n = n;
        self.gate_room = self.room;
        self.eb.clear();
        self.pb.clear();
        self.play_song(None);
        self.sfx(Sfx::Gate);
    }
    fn update_lairin(&mut self) {
        self.t += 1;
        self.update_parts();
        if self.t >= 100 {
            let n = self.gate_n;
            self.start_lair(n);
        }
    }
    fn start_lair(&mut self, n: usize) {
        let th = if n == 6 { 5 } else { 4 };
        self.arena = Some(make_arena(&self.themes[th], (n - 1).min(3)));
        self.in_lair = n;
        self.mode = Mode::Play;
        self.paused = false;
        self.enemies.clear();
        self.pb.clear();
        self.eb.clear();
        self.items.clear();
        self.parts.clear();
        self.scroll = None;
        self.msg = None;
        self.pl = Player { x: 128.0, y: 200.0, w: 10.0, h: 12.0, fx: 0.0, fy: -1.0, dir: b'u', walk: 0, cd: 0, scd: 10, inv: 60, cast: 0 };
        let (name, el, shifting, hp, pats) = boss_def(n);
        let size = if n == 6 { 20 } else { 16 };
        let sprs = gen_boss(9001 + n as u32 * 53, size, size);
        let (sw, sh) = (sprs[0].img.w as f32, sprs[0].img.h as f32);
        let sc = 3;
        self.boss = Some(Boss {
            name, x: 128.0, y: 100.0, w: sw * sc as f32 * 0.7, h: sh * sc as f32 * 0.6, hp, max: hp, t: 0, t0: 0,
            pat: 0, pat_t: 0, flash: 0, tip: 0, slow: 0, alive: true, entering: true, gone: false, lv: n, el, shifting,
            sprs, sc, pats,
        });
        self.boss_dead = false;
        self.clear_t = 0;
        self.lair_intro = 150;
        self.flash = 16;
        self.play_song(Some(Song::Lair));
    }
    fn update_boss(&mut self) {
        let Some(mut b) = self.boss.take() else { return };
        self.upd_boss(&mut b);
        self.boss = Some(b);
    }
    fn upd_boss(&mut self, b: &mut Boss) {
        if self.lair_intro > 0 {
            self.lair_intro -= 1;
        }
        if !b.alive {
            return;
        }
        b.t += 1;
        if b.flash > 0 {
            b.flash -= 1;
        }
        if b.tip > 0 {
            b.tip -= 1;
        }
        if b.entering {
            if b.t >= 80 {
                b.entering = false;
                b.t0 = b.t;
            }
            return;
        }
        let slow = if b.slow > 0 {
            b.slow -= 1;
            true
        } else {
            false
        };
        let rage = b.hp < b.max * 0.4;
        let sp = if rage { 1.5 } else { 1.0 } * if slow { 0.5 } else { 1.0 };
        let bt = (b.t - b.t0) as f32;
        b.x = 128.0 + (bt * 0.011 * sp).sin() * if b.lv == 6 { 70.0 } else { 80.0 };
        b.y = 100.0 + (bt * 0.017).sin() * 34.0;
        b.pat_t += 1;
        if b.pat_t > 220 {
            b.pat_t = 0;
            b.pat = (b.pat + 1) % b.pats.len();
            if b.shifting {
                b.el = Elem::from_idx((b.el.idx() + 1) % 4);
                b.flash = 10;
                self.float(format!("{}!", b.el.name()), b.x - 24.0, b.y - 44.0, b.el.light());
                self.sfx(Sfx::Spell);
            }
        }
        let f = b.pat_t;
        let lv = b.lv as f32;
        let rate = if rage { 0.7 } else { 1.0 } * if slow { 1.6 } else { 1.0 };
        let every = |n: f32| f % ((n * rate).round().max(1.0) as i32) == 0;
        let (px, py) = (self.pl.x, self.pl.y);
        if f >= 20 {
            match b.pats[b.pat] {
                Pat::Aim => {
                    if every(28.0) {
                        let a = ang(b.x, b.y, px, py);
                        let n = if b.lv >= 3 { 5 } else { 3 };
                        for i in 0..n {
                            let da = (i as f32 - (n - 1) as f32 / 2.0) * 0.22;
                            self.eshot(b.x, b.y + 10.0, a + da, 1.7 + lv * 0.1, b.el);
                        }
                    }
                }
                Pat::Ring => {
                    if every(62.0) {
                        let n = 10 + b.lv * 2;
                        let o = self.rng.f();
                        for i in 0..n {
                            self.eshot(b.x, b.y, o + i as f32 * PI * 2.0 / n as f32, 1.2 + lv * 0.06, b.el);
                        }
                    }
                }
                Pat::Spiral => {
                    if every(6.0) {
                        for k in 0..2 {
                            self.eshot(b.x, b.y, b.t as f32 * 0.14 + k as f32 * PI, 1.4 + lv * 0.05, b.el);
                        }
                    }
                }
                Pat::Spiral2 => {
                    if every(5.0) {
                        for k in 0..3 {
                            self.eshot(b.x, b.y, -(b.t as f32) * 0.11 + k as f32 * 2.094, 1.6, b.el);
                        }
                    }
                }
                Pat::Summon => {
                    if f % 90 == 0 && self.enemies.len() < 6 {
                        for s in [-1.0f32, 1.0] {
                            let th = (b.lv - 1).min(3);
                            let k = if b.lv % 2 == 0 { EK::Bat } else { EK::Slime };
                            let mut m = self.make_enemy(k, b.x + s * 30.0, b.y + 20.0, th);
                            m.el = b.el;
                            m.spawn = 20;
                            self.enemies.push(m);
                        }
                    }
                    if f % 40 == 20 {
                        self.eshot(b.x, b.y + 10.0, ang(b.x, b.y, px, py), 2.1, b.el);
                    }
                }
                Pat::Rain => {
                    if every(9.0) {
                        let x = self.rng.range(24.0, 232.0);
                        let da = self.rng.range(-0.25, 0.25);
                        let s = 1.2 + self.rng.f() * 0.8;
                        self.eshot(x, HUDF + 18.0, PI / 2.0 + da, s, b.el);
                    }
                }
            }
        }
        let p = self.pl;
        if hit(b.x, b.y, b.w, b.h, p.x, p.y, p.w, p.h) {
            self.hurt(4);
        }
    }
    fn update_lair_state(&mut self) {
        if !self.boss_dead {
            return;
        }
        self.clear_t += 1;
        let (bx, by, el) = self.boss.as_ref().map_or((128.0, 100.0, Elem::Neutral), |b| (b.x, b.y, b.el));
        if self.clear_t < 110 && self.clear_t % 7 == 0 {
            let (ox, oy) = (self.rng.range(-26.0, 26.0), self.rng.range(-22.0, 22.0));
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
    fn finish_lair(&mut self) {
        let n = self.in_lair;
        self.s.cleared[n] = true;
        self.s.max_hp += 4;
        match n {
            1 => self.s.spell_lv = 2,
            2 => self.s.max_mp += 20,
            3 => self.s.max_hp += 8,
            4 => self.s.spell_lv = 3,
            5 => self.s.speed = 1.8,
            _ => {}
        }
        self.s.hp = self.s.max_hp;
        self.s.mp = self.s.max_mp as f32;
        self.s.food = 100.0;
        self.s.room = self.gate_room;
        self.save();
        self.t = 0;
        self.mode = if n == 6 { Mode::Victory } else { Mode::Reward };
        self.play_song(if n == 6 { Some(Song::Title) } else { None });
        self.sfx(Sfx::Fanfare);
    }
    fn update_reward(&mut self) {
        self.t += 1;
        self.update_stars(1.5);
        if self.t > 60 && (self.p(Btn::Start) || self.p(Btn::Fire)) {
            let r = self.gate_room;
            self.go_play(r, GATE_X, SPAWN_Y, false);
        }
    }
    fn update_dying(&mut self) {
        self.t += 1;
        self.update_parts();
        if self.t > 120 {
            self.mode = Mode::GameOver;
            self.t = 0;
            self.menu = 0;
        }
    }
    fn update_gameover(&mut self) {
        self.t += 1;
        if self.p(Btn::Up) || self.p(Btn::Down) {
            self.menu = 1 - self.menu;
            self.sfx(Sfx::Select);
        }
        if self.t > 30 && (self.p(Btn::Start) || self.p(Btn::Fire)) {
            self.sfx(Sfx::Select);
            self.s.hp = self.s.max_hp;
            self.s.food = self.s.food.max(60.0);
            self.s.gold /= 2;
            self.save();
            if self.menu == 0 {
                let r = if self.s.room < self.rooms.len() { self.s.room } else { self.start };
                self.go_play(r, GATE_X, SPAWN_Y, false);
            } else {
                self.mode = Mode::Title;
                self.menu = 0;
                self.play_song(Some(Song::Title));
            }
        }
    }
    fn update_victory(&mut self) {
        self.t += 1;
        self.update_stars(1.0);
        if self.t > 240 && (self.p(Btn::Start) || self.p(Btn::Fire)) {
            self.mode = Mode::Title;
            self.menu = 0;
        }
    }

    // ============================================================ drawing
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
            Mode::Play | Mode::Dying => {
                self.draw_play(scr);
                self.draw_hud(scr);
                if self.paused {
                    if self.in_lair > 0 {
                        scr.blend(0, HUD, W, H - HUD, BLACK, 0.6);
                        scr.text("PAUSED", 128, 120, WHITE, Align::Center, 16);
                    } else {
                        self.draw_map(scr);
                    }
                }
            }
            Mode::LairIn => {
                self.draw_play(scr);
                self.draw_lairin(scr);
                self.draw_hud(scr);
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

    fn draw_stars(&self, scr: &mut Screen) {
        for s in &self.stars {
            let c = if s.s > 1.2 { WHITE } else if s.s > 0.7 { rgb(0x747474) } else { rgb(0x3c3c7c) };
            scr.pset(s.x as i32, s.y as i32, c);
        }
    }
    fn gate_state(&self, n: usize) -> u8 {
        if self.s.cleared[n] {
            2
        } else if n == 1 || self.s.cleared[n - 1] {
            1
        } else {
            0
        }
    }
    fn draw_gate(&self, scr: &mut Screen, n: usize, x: i32, y: i32) {
        let st = self.gate_state(n);
        let stone = if n == 6 { rgb(0x602020) } else { rgb(0x5c5c5c) };
        scr.fill(x - 14, y - 14, 28, 28, rgb(0x101010));
        scr.fill(x - 13, y - 13, 26, 26, stone);
        scr.fill(x - 13, y - 13, 26, 2, rgb(0x9c9c9c));
        scr.fill(x - 8, y - 6, 16, 18, BLACK);
        scr.fill(x - 6, y - 9, 12, 3, BLACK);
        for i in 0..3 {
            scr.fill(x - 7 + i * 2, y + 2 + i * 3, 14 - i * 4, 1, rgb(0x303030));
        }
        let glow = match st {
            1 => {
                if (self.frame >> 3) & 1 == 1 { rgb(0xf878f8) } else { rgb(0x9818a8) }
            }
            2 => rgb(0x58d854),
            _ => rgb(0x404040),
        };
        scr.frame_rect(x - 14, y - 14, 28, 28, glow);
        if st == 0 {
            scr.line(x - 8, y - 6, x + 7, y + 11, rgb(0x8c8c8c));
            scr.line(x + 7, y - 6, x - 8, y + 11, rgb(0x8c8c8c));
        }
        let label = if n == 6 { "X".to_string() } else { n.to_string() };
        scr.text(&label, x + 1, y - 13, if st == 0 { rgb(0x747474) } else { WHITE }, Align::Center, 8);
    }
    fn draw_room_objs(&self, scr: &mut Screen, ri: usize, ox: i32, oy: i32) {
        let r = &self.rooms[ri];
        let (x, y) = (GATE_X as i32 + ox, GATE_Y as i32 + oy);
        let bob = if (self.frame >> 4) & 1 == 1 { 1.0 } else { 0.0 };
        if r.gate > 0 {
            self.draw_gate(scr, r.gate, x, y);
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
            let el = Elem::from_idx(se);
            scr.fill(x - 7, y + 4, 14, 6, rgb(0x747474));
            scr.fill(x - 7, y + 4, 14, 1, rgb(0xbcbcbc));
            scr.fill(x - 5, y + 10, 10, 2, rgb(0x404040));
            let oy2 = y - 4 + ((self.frame as f32 * 0.08).sin() * 2.0) as i32;
            scr.disc(x, oy2, 5, el.main());
            scr.disc(x - 1, oy2 - 1, 3, el.light());
            scr.pset(x - 2, oy2 - 2, WHITE);
            if self.frame % 20 < 10 {
                scr.pset(x + 6, oy2 - 5, WHITE);
            }
        }
        if ri == self.start && self.in_lair == 0 {
            let sage = &self.spr.mage_d[4];
            scr.spr(&sage.img, 128.0 + ox as f32, HUDF + 40.0 + oy as f32, false);
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
    fn draw_hero(&self, scr: &mut Screen, ox: f32, oy: f32) {
        let p = &self.pl;
        if p.inv > 0 && self.frame & 2 != 0 {
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
        if e.k == EK::Ghost && self.frame % 4 == 0 && e.flash == 0 {
            return;
        }
        let set = if e.k == EK::Generator {
            &self.spr.generator
        } else {
            &self.spr.enemies[e.k as usize][e.el.idx()]
        };
        let mut flip = false;
        let mut bob = 0.0;
        match e.k {
            EK::Slime => bob = ((e.t >> 4) & 1) as f32,
            EK::Bat | EK::Skeleton => flip = (e.t >> 3) & 1 == 1,
            EK::Ghost => bob = (e.t as f32 * 0.1).sin().round(),
            EK::Generator => bob = if e.t % 30 < 4 { -1.0 } else { 0.0 },
            _ => {}
        }
        let img = if e.flash > 0 {
            &set.white
        } else if e.frozen > 0 {
            &set.ice
        } else {
            &set.img
        };
        scr.spr(img, e.x, e.y + bob, flip);
        if e.slow > 0 && e.frozen == 0 && self.frame % 8 < 2 {
            scr.pset(e.x as i32 + 5, e.y as i32 - 6, rgb(0xa4e4fc));
        }
    }
    fn draw_bullets(&self, scr: &mut Screen) {
        for b in &self.pb {
            let (x, y) = (b.x.round() as i32, b.y.round() as i32);
            match b.el {
                Elem::Fire => {
                    scr.disc(x, y, 3, rgb(0xd82800));
                    scr.disc(x, y, 2, rgb(0xfc9838));
                    scr.fill(x, y - 1, 1, 2, rgb(0xfce040));
                }
                Elem::Ice => {
                    scr.fill(x - 1, y - 3, 3, 7, rgb(0xa4e4fc));
                    scr.fill(x - 3, y - 1, 7, 3, rgb(0xa4e4fc));
                    scr.fill(x, y - 1, 1, 3, WHITE);
                }
                Elem::Storm => {
                    let l = b.vx.hypot(b.vy).max(0.01);
                    let (ux, uy) = (b.vx / l, b.vy / l);
                    let (nx, ny) = (-uy, ux);
                    let mut prev = (x, y);
                    for k in 1..5 {
                        let j = if k % 2 == 0 { 2.0 } else { -2.0 };
                        let pt = ((b.x - ux * k as f32 * 3.0 + nx * j) as i32, (b.y - uy * k as f32 * 3.0 + ny * j) as i32);
                        scr.line(prev.0, prev.1, pt.0, pt.1, if k < 3 { WHITE } else { rgb(0xfce040) });
                        prev = pt;
                    }
                }
                _ => {
                    scr.disc(x, y, 4, rgb(0x3c2410));
                    scr.disc(x, y, 3, rgb(0x8c5020));
                    scr.fill(x - 1, y - 2, 2, 1, rgb(0x98d858));
                }
            }
        }
        let blink = (self.frame >> 2) & 1 == 1;
        for b in &self.eb {
            let (x, y) = (b.x.round() as i32, b.y.round() as i32);
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
                    scr.disc(x, y, 4, it.el.main());
                    scr.disc(x - 1, y - 1, 2, it.el.light());
                    scr.pset(x - 2, y - 2, WHITE);
                    continue;
                }
            };
            scr.spr(&s.img, it.x, it.y + if it.kind == IK::Coin { 0.0 } else { bob }, false);
        }
    }
    fn draw_parts(&self, scr: &mut Screen) {
        for p in &self.parts {
            match &p.kind {
                PK::Text(s) => scr.text(s, p.x as i32, p.y as i32, p.c, Align::Left, 8),
                PK::Ring => {
                    let r = (p.max - p.life) * 2 + 2;
                    scr.ring(p.x as i32, p.y as i32, r, if p.c != WHITE { p.c } else if p.life > 6 { WHITE } else { rgb(0xfcbc3c) });
                }
                PK::Line(x2, y2) => {
                    let (mx, my) = ((p.x + x2) / 2.0 + self.rng_jitter(p.life), (p.y + y2) / 2.0 - self.rng_jitter(p.life + 3));
                    scr.line(p.x as i32, p.y as i32, mx as i32, my as i32, if p.life % 2 == 0 { WHITE } else { p.c });
                    scr.line(mx as i32, my as i32, *x2 as i32, *y2 as i32, if p.life % 2 == 0 { WHITE } else { p.c });
                }
                PK::Dot => scr.fill(p.x.round() as i32, p.y.round() as i32, p.sz, p.sz, p.c),
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
            scr.blit(&self.rooms[sc.from].img, ox, HUD + oy, false, false);
            self.draw_room_objs(scr, sc.from, ox, oy);
            scr.blit(&self.rooms[sc.to].img, nx, HUD + ny, false, false);
            self.draw_room_objs(scr, sc.to, nx, ny);
            self.draw_hero(scr, nx as f32, ny as f32);
        } else {
            scr.blit(&self.cur_room().img, 0, HUD, false, false);
            if self.in_lair == 0 {
                self.draw_room_objs(scr, self.room, 0, 0);
            }
            self.draw_items(scr);
            for e in &self.enemies {
                self.draw_enemy(scr, e);
            }
            if let Some(b) = &self.boss {
                let hide = b.gone || (self.boss_dead && self.frame & 2 != 0) || (b.entering && (b.t >> 2) & 1 == 1);
                if !hide {
                    let s = &b.sprs[b.el.idx().min(3)];
                    let img = if b.flash > 0 {
                        &s.white
                    } else if b.slow > 0 && self.frame % 16 < 4 {
                        &s.ice
                    } else {
                        &s.img
                    };
                    let (w, h) = (img.w * b.sc, img.h * b.sc);
                    scr.blit_scaled(img, b.x as i32 - w / 2, b.y as i32 - h / 2, b.sc);
                }
            }
            if self.mode != Mode::Dying {
                self.draw_hero(scr, 0.0, 0.0);
            }
            self.draw_bullets(scr);
            self.draw_parts(scr);
        }
        scr.unclip();
        if self.in_lair > 0 {
            if let Some(b) = &self.boss {
                if b.alive && !b.entering {
                    scr.fill(0, H - 13, W, 13, BLACK);
                    scr.text(b.name, 6, H - 11, b.el.light(), Align::Left, 8);
                    let bw = 256 - 130 - 8;
                    scr.fill(130, H - 10, bw, 6, rgb(0x300000));
                    scr.fill(130, H - 10, (bw as f32 * b.hp / b.max).ceil() as i32, 6, rgb(0xf83800));
                }
                if self.lair_intro > 0 && (self.lair_intro >> 3) & 1 == 1 {
                    scr.fill(0, 150, W, 36, BLACK);
                    scr.text(b.name, 128, 156, b.el.light(), Align::Center, 16);
                    let hint = if b.shifting {
                        "IT SHIFTS ITS ELEMENT!".to_string()
                    } else {
                        format!("WEAK TO {}", b.el.opposite().name())
                    };
                    scr.text(&hint, 128, 175, WHITE, Align::Center, 8);
                }
            }
            if self.boss_dead && self.clear_t > 130 {
                scr.text("VICTORY!", 128, 110, rgb(0xb8f818), Align::Center, 16);
            }
        }
        self.draw_msg(scr);
    }
    fn draw_lairin(&self, scr: &mut Screen) {
        let t = self.t;
        scr.clip(0, HUD, W, H);
        for i in 0..6 {
            let r = (t * 3 + i * 22) % 150;
            let c = [rgb(0x9818a8), rgb(0x5c4880), rgb(0x201430)][i as usize % 3];
            scr.frame_rect(GATE_X as i32 - r, GATE_Y as i32 - r, r * 2, r * 2, c);
        }
        if t > 25 && (t >> 3) & 1 == 1 {
            scr.fill(40, 60, 176, 20, BLACK);
            let label = if self.gate_n == 6 { "THE DARK TOWER".to_string() } else { format!("MONSTER LAIR {}", self.gate_n) };
            scr.text(&label, 128, 66, WHITE, Align::Center, 8);
        }
        if t > 70 {
            scr.blend(0, HUD, W, H - HUD, BLACK, (t - 70) as f32 / 30.0);
        }
        scr.unclip();
    }
    fn draw_hud(&self, scr: &mut Screen) {
        scr.fill(0, 0, W, HUD, BLACK);
        scr.fill(0, HUD - 2, W, 1, rgb(0x5c4880));
        // Row 1: life, gold
        scr.text("HP", 4, 5, rgb(0xfc7460), Align::Left, 8);
        // Two HP per segment, stretched so a huge life total still fits before the gold counter.
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
        // Row 2: mana, food, element, runes
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
        let runes = (1..=5).filter(|&i| self.s.cleared[i]).count();
        scr.spr(&self.spr.gem.img, 206.0, 22.0, false);
        scr.text(&format!("{}/5", runes), 212, 19, rgb(0xf878f8), Align::Left, 8);
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
            if r.i == self.start {
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
        scr.text("S SHOP  RED HEART  BROWN CHEST", 128, y + 26, rgb(0x747474), Align::Center, 8);
    }
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
        scr.text("SELECT / M: MUTE", 128, 208, rgb(0x747474), Align::Center, 8);
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
            ["EXPLODING FIREBALLS", "SPELL: FLAME RING", "STRONG VS ICE FOES"],
            ["SHARDS THAT SLOW FOES", "SPELL: FROST NOVA", "STRONG VS FIRE FOES"],
            ["FAST PIERCING BOLTS", "SPELL: CHAIN BOLT", "STRONG VS EARTH FOES"],
            ["HEAVY KNOCKBACK ROCKS", "SPELL: QUAKE", "STRONG VS STORM FOES"],
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
        let n = self.gate_n;
        let (name, el, _, _, _) = boss_def(n);
        scr.text(name, 128, 50, el.light(), Align::Center, 16);
        scr.text("DEFEATED!", 128, 72, rgb(0xb8f818), Align::Center, 16);
        if self.t > 30 {
            scr.text(&format!("YOU CLAIM RUNE {} OF 5", n), 128, 110, rgb(0xf878f8), Align::Center, 8);
        }
        if self.t > 45 {
            scr.text(REWARDS[n], 128, 128, rgb(0xfcbc3c), Align::Center, 8);
        }
        if self.t > 55 {
            scr.text("MAXIMUM LIFE UP", 128, 144, rgb(0xfc7460), Align::Center, 8);
        }
        if self.t > 65 {
            let s = if n < 5 { format!("LAIR {} IS NOW OPEN", n + 1) } else { "THE DARK TOWER IS OPEN".to_string() };
            scr.text(&s, 128, 164, rgb(0x3cbcfc), Align::Center, 8);
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

    // ------------------------------------------------------------ debug hooks (snapshot mode)
    pub fn debug_no_save(&mut self) {
        self.no_save = true;
        self.has_save = false;
    }
    pub fn debug_force_new_game_menu(&mut self) {
        self.menu = self.menu_opts().iter().position(|o| *o == "NEW GAME").unwrap_or(0);
    }
    pub fn debug_lair(&mut self, n: usize) {
        if self.rooms.is_empty() {
            self.setup_world();
        }
        self.gate_room = self.rooms.iter().position(|r| r.gate == n).unwrap_or(self.start);
        self.s.max_hp = 999;
        self.s.hp = 999;
        self.gate_n = n;
        self.start_lair(n);
    }
    pub fn debug_boss_dx(&self) -> f32 {
        self.boss.as_ref().map_or(0.0, |b| b.x - self.pl.x)
    }
    pub fn debug_summary(&self) -> String {
        format!(
            "doors={:?} pl=({:.0},{:.0}) mode={:?} room={} lair={} hp={}/{} mp={:.0} food={:.1} gold={} el={} enemies={} boss_hp={:?}",
            self.rooms.get(self.room).map(|r| r.doors), self.pl.x, self.pl.y, self.mode, self.room, self.in_lair, self.s.hp, self.s.max_hp, self.s.mp, self.s.food, self.s.gold,
            self.el().name(), self.enemies.len(), self.boss.as_ref().map(|b| b.hp as i32)
        )
    }
}

const INTRO: [&str; 11] = [
    "THE DARK SORCERER HAS",
    "STOLEN THE LIGHT OF THE",
    "REALM AND SEALED IT IN",
    "HIS TOWER.",
    "",
    "FIVE MONSTERS GUARD THE",
    "RUNES THAT OPEN IT.",
    "MAGE, TAKE UP YOUR STAFF.",
    "LEARN THEIR WEAKNESSES.",
    "",
    "AND DON'T FORGET TO EAT.",
];
