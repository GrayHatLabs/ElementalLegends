// ELEMENTAL LEGENDS - core game state, rules and flow.
//
// Submodules (all `impl Game` blocks over the same state):
//   status  - reusable burn / chill / freeze / stun effects
//   combat  - casting, projectiles, damage, enemy AI, particles
//   dungeon - dungeon rooms, keys, locked doors and puzzles
//   bosses  - the six fantasy bosses, their attacks and hazards
//   scenes  - monolith awakening, dungeon buildings, staircase and boss intro cinematics
//   draw    - world, HUD, map and menu rendering
//   debug   - hooks for the headless --snapshot / --selftest modes
use crate::audio::{Sfx, Song, Synth, SFX_COUNT};
use crate::gfx::*;
use crate::sprites::*;
use crate::world::*;
use sdl2::audio::AudioDevice;
use std::f32::consts::PI;

mod bosses;
mod combat;
mod debug;
mod draw;
mod dungeon;
mod minis;
mod scenes;
mod status;
mod village;

use bosses::Boss;
use dungeon::Dungeon;
use minis::enemy_mult;
use status::Status;

pub const GATE_X: f32 = 128.0;
pub const GATE_Y: f32 = (HUD + 6 * TS + 8) as f32;
/// Visible part of the world in logic units (320x208 screen pixels at 1.5x zoom).
pub const VIEW_W: f32 = SW as f32 / ZOOM;
pub const VIEW_H: f32 = (SH - HUD_PX) as f32 / ZOOM;
const WF: f32 = W as f32;
const HF: f32 = H as f32;
const HUDF: f32 = HUD as f32;
const SPAWN_Y: f32 = GATE_Y + 34.0;
/// Bump when the world layout changes; older saves are detected and ignored.
const SAVE_VERSION: u32 = 2;

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
fn tile_of(x: f32, y: f32) -> (i32, i32) {
    ((x / TS as f32).floor() as i32, ((y - HUDF) / TS as f32).floor() as i32)
}
fn tile_center(c: i32, r: i32) -> (f32, f32) {
    ((c * TS + 8) as f32, (HUD + r * TS + 8) as f32)
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
    /// Drink a carried mana potion.
    Potion,
    Start,
    Mute,
}

#[derive(Clone, Copy, Default)]
pub struct Input {
    keys: [bool; 9],
    pad: [bool; 9],
    axis: [bool; 9],
    pressed: [bool; 9],
    /// Right analog stick (-1..1), for optional twin-stick aiming.
    aim: (f32, f32),
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
    pub fn set_aim_x(&mut self, v: f32) {
        self.aim.0 = v;
    }
    pub fn set_aim_y(&mut self, v: f32) {
        self.aim.1 = v;
    }
    /// Twin-stick aim direction (unit vector) when the right stick is pushed past the deadzone.
    pub fn aim(&self) -> Option<(f32, f32)> {
        let (x, y) = self.aim;
        let l = x.hypot(y);
        (l > 0.4).then(|| (x / l, y / l))
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
        Elem::Fire => (3.4, 2.0, 3.0, 16, 72),
        Elem::Ice => (4.4, 1.5, 3.0, 11, 60),
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
/// Mana regenerates slowly (0.5 MP per second), so potions matter.
const MP_REGEN: f32 = 0.5 / 60.0;
/// How many mana potions the mage can carry.
const MAX_POTIONS: i32 = 5;

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
    /// Walking this frame (drives the walk-cycle animation).
    moving: bool,
}
impl Player {
    fn at(x: f32, y: f32) -> Self {
        Player { x, y, w: 10.0, h: 12.0, fx: 0.0, fy: -1.0, dir: b'u', walk: 0, cd: 0, scd: 10, inv: 30, cast: 0, moving: false }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EK {
    Slime = 0,
    Bat,
    Skeleton,
    Imp,
    Ghost,
    Golem,
    Generator,
    // Overworld mini-bosses and their minions (see minis.rs).
    HoardDragon,
    Dryad,
    Treant,
    Zombie,
    GraveLord,
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
    st: Status,
    gen_id: u32,
    kbx: f32,
    kby: f32,
    tip: i32,
    /// Overworld mini-boss id (world::MINI_*), 0 for ordinary monsters.
    mini: u8,
    /// Mini-boss behaviour state (asleep / friendly / revealed / flying off...).
    mode: u8,
    timer: i32,
}
impl Enemy {
    /// Fully spawned in: can be hit. (Disguised or sleeping encounters use touch = 0 so they don't hurt.)
    fn active(&self) -> bool {
        self.spawn <= 0
    }
}

/// Enemy projectile looks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shot {
    Orb,
    Flame,
    Fireball,
    Boulder,
    Dark,
    Wave,
    /// A thrown apple (angry treant).
    Apple,
    /// Dryad thorns.
    Thorn,
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
    age: i32,
    style: Shot,
    pierce: Option<Vec<u32>>,
    dead: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum IK {
    Coin,
    Gem,
    Apple,
    Bread,
    Meat,
    Potion,
    Heart,
    Orb,
    /// Full food and full health (the angry treant's reward).
    GoldenApple,
    /// Cures poison; carried.
    Antidote,
}
struct Item {
    kind: IK,
    x: f32,
    y: f32,
    val: i32,
    el: Elem,
    life: i32,
    dead: bool,
    /// Special pickups: ITEM_HOARD coins belong to the Hoard Dragon's pile.
    tag: u8,
}
const ITEM_HOARD: u8 = 1;
/// Placed pickups (larders, the hoard) never expire or blink.
const PLACED_LIFE: i32 = i32::MAX / 2;

enum PK {
    Dot,
    Ring,
    Text(String),
    Line(f32, f32),
    /// Ice fragment falling under gravity.
    Shard,
    /// Expanding impact flash of an element with a max radius.
    Burst(Elem, f32),
    /// Radiating ice-crystal spikes.
    Crystal,
    /// Rising, cooling spark.
    Ember,
    /// Soft translucent glow of a radius.
    Glow(f32),
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
    /// Scrolling between dungeon rooms rather than overworld areas.
    dun: bool,
    /// Where the new area sits relative to the old one (logic units).
    nx: f32,
    ny: f32,
    /// Camera at the start (old area's frame) and end (new area's frame).
    cam0: (f32, f32),
    cam1: (f32, f32),
}

const REWARDS: [&str; 7] = [
    "",
    "SPELL POWER UP: LONGER BOLTS",
    "MANA CRYSTAL: MAX MP UP",
    "HEART CONTAINER",
    "SPELL POWER UP: TWIN BOLTS",
    "WINGED BOOTS: SPEED UP",
    "",
];

// ---------------------------------------------------------------- save data
#[derive(Clone, Debug, PartialEq)]
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
    /// Per-dungeon puzzle progress bit flags (see dungeon::D_*).
    dprog: [u8; 7],
    /// Mana potions carried.
    potions: i32,
    /// Overworld encounters discovered / finished (bit per world::MINI_* id).
    mini_seen: u32,
    mini_done: u32,
    /// Coins left on the Hoard Dragon's pile (-1 = untouched).
    hoard_left: i32,
    /// Zombies raised in the graveyard so far (the Grave Lord rises at 6).
    zombies: i32,
    /// Antidotes carried.
    antidotes: i32,
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
            dprog: [0; 7],
            potions: 1,
            mini_seen: 0,
            mini_done: 0,
            hoard_left: -1,
            zombies: 0,
            antidotes: 0,
        }
    }
    fn mini_done(&self, id: u8) -> bool {
        self.mini_done & (1 << id) != 0
    }
    fn to_text(&self) -> String {
        let b = |v: &[bool]| v.iter().map(|x| if *x { "1" } else { "0" }).collect::<Vec<_>>().join(",");
        let l = |v: &[usize]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        let dp = self.dprog.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        format!(
            "version={}\nmax_hp={}\nhp={}\nmax_mp={}\nmp={}\nfood={}\ngold={}\nel={}\nspell_lv={}\nspeed={}\ncleared={}\ntanks={}\ncaches={}\nopened={}\nvisited={}\nroom={}\ntime={}\nheart_price={}\ndprog={}\npotions={}\nmini_seen={}\nmini_done={}\nhoard_left={}\nzombies={}\nantidotes={}\n",
            SAVE_VERSION, self.max_hp, self.hp, self.max_mp, self.mp, self.food, self.gold, self.el, self.spell_lv, self.speed,
            b(&self.cleared), l(&self.tanks), l(&self.caches), l(&self.opened), l(&self.visited), self.room,
            self.time, self.heart_price, dp, self.potions, self.mini_seen, self.mini_done, self.hoard_left,
            self.zombies, self.antidotes
        )
    }
    fn from_text(txt: &str) -> Option<Self> {
        let mut s = Self::fresh();
        let mut ok = false;
        let mut version = 0;
        for line in txt.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim();
            let num = || v.parse::<f64>().unwrap_or(0.0);
            let list = || -> Vec<usize> { v.split(',').filter_map(|x| x.trim().parse().ok()).collect() };
            match k.trim() {
                "version" => version = num() as u32,
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
                "potions" => s.potions = (num() as i32).clamp(0, MAX_POTIONS),
                "mini_seen" => s.mini_seen = num() as u32,
                "mini_done" => s.mini_done = num() as u32,
                "hoard_left" => s.hoard_left = num() as i32,
                "zombies" => s.zombies = num() as i32,
                "antidotes" => s.antidotes = (num() as i32).clamp(0, 3),
                "dprog" => {
                    for (i, x) in v.split(',').enumerate().take(7) {
                        s.dprog[i] = x.trim().parse().unwrap_or(0);
                    }
                }
                _ => {}
            }
        }
        // Saves from before the SNES-scale world (different map layout) are ignored.
        if ok && s.max_hp > 0 && version == SAVE_VERSION {
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
    /// Opening cinematic at the monolith.
    Awaken,
    Play,
    /// Walking into a dungeon building.
    EnterDungeon,
    /// Walking down the staircase to a boss.
    Descend,
    /// Boss entrance animation.
    BossIntro,
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
    shop_room: usize,
    room: usize,
    /// Boss number while inside a boss arena (0 = none).
    in_lair: usize,
    arena: Option<Room>,
    dungeon: Option<Dungeon>,
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
    /// Fade-in from black after scene changes (frames remaining).
    fade: i32,
    paused: bool,
    boss: Option<Boss>,
    boss_dead: bool,
    clear_t: i32,
    gate_n: usize,
    gate_room: usize,
    gate_warned: bool,
    monolith_used: bool,
    /// Cinematic helpers: extra vertical pan used by the staircase descent.
    pan_y: f32,
    /// World camera: logic position shown at the top-left of the play field.
    cam: (f32, f32),
    sink: f32,
    walk_from: (f32, f32),
    stars: Vec<Star>,
    s: SaveData,
    rng: Rng,
    next_id: u32,
    spr: Sprites,
    /// Native-resolution art from the generated sheets (falls back to spr).
    art: crate::art::Art,
    themes: Vec<Theme>,
    audio: Option<AudioDevice<Synth>>,
    has_save: bool,
    last_sfx: [u64; SFX_COUNT],
    muted: bool,
    inp: Input,
    starve_t: i32,
    hungry_warned: bool,
    starving_warned: bool,
    shop_armed: [bool; 6],
    /// Player statuses: poison frames left (drains HP, green tint).
    poison: i32,
    /// Overworld encounter runtime state (see minis.rs).
    dryad_stage: u8,
    fruit: Vec<minis::Fruit>,
    graves_open: Vec<bool>,
    hazards: Vec<bosses::Hazard>,
    hoard_anger: f32,
    mini_hint: bool,
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
            shop_room: 0,
            room: 0,
            in_lair: 0,
            arena: None,
            dungeon: None,
            pl: Player::at(128.0, 160.0),
            enemies: vec![],
            pb: vec![],
            eb: vec![],
            items: vec![],
            parts: vec![],
            scroll: None,
            msg: None,
            flash: 0,
            shake: 0,
            fade: 0,
            paused: false,
            boss: None,
            boss_dead: false,
            clear_t: 0,
            gate_n: 0,
            gate_room: 0,
            gate_warned: false,
            monolith_used: false,
            pan_y: 0.0,
            cam: (0.0, HUDF),
            sink: 0.0,
            walk_from: (0.0, 0.0),
            stars,
            s: SaveData::fresh(),
            rng,
            next_id: 1,
            spr: Sprites::new(),
            art: crate::art::Art::load(),
            themes: build_themes(),
            audio,
            has_save: load_save().is_some(),
            last_sfx: [0; SFX_COUNT],
            muted: false,
            inp: Input::default(),
            starve_t: 0,
            hungry_warned: false,
            starving_warned: false,
            shop_armed: [true; 6],
            poison: 0,
            dryad_stage: 0,
            fruit: vec![],
            graves_open: vec![],
            hazards: vec![],
            hoard_anger: 0.0,
            mini_hint: false,
            no_save: false,
            quit: false,
        };
        g.attach_terrain_art();
        g.play_song(Some(Song::Title));
        g
    }

    /// Background music for where the mage is: village at the monolith and shop, a theme
    /// per overworld region, the dungeon theme below ground and the boss theme in lairs.
    fn area_song(&self) -> Song {
        if self.in_lair > 0 {
            return Song::Lair;
        }
        if self.dungeon.is_some() {
            return Song::Dungeon;
        }
        let r = &self.rooms[self.room];
        if r.special != SP_NONE {
            return Song::Village;
        }
        match r.theme {
            1 => Song::Crypt,
            2 => Song::Swamp,
            3 => Song::Volcano,
            _ => Song::Field,
        }
    }

    /// Give each theme its generated 24 px tilesets, if the art has them.
    fn attach_terrain_art(&mut self) {
        for (i, th) in self.themes.iter_mut().enumerate() {
            let Some(t) = self.art.terrain(i) else { continue };
            let ok = |w: &crate::art::Wang| w.size == HD_TS && w.tiles.len() == 16;
            th.hd_wall = t.wall.as_ref().filter(|w| ok(w)).map(|w| w.tiles.clone());
            th.hd_water = t.water.as_ref().filter(|w| ok(w)).map(|w| w.tiles.clone());
            th.hd_deco = t.deco.clone();
        }
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
        let (rooms, start, shop) = gen_world(&self.themes);
        self.rooms = rooms;
        self.start = start;
        self.shop_room = shop;
        for &i in &self.s.visited {
            if let Some(r) = self.rooms.get_mut(i) {
                r.visited = true;
            }
        }
        self.setup_minis();
    }
    fn el(&self) -> Elem {
        Elem::from_idx(self.s.el)
    }
    fn overworld(&self) -> bool {
        self.in_lair == 0 && self.dungeon.is_none()
    }
    fn cur_room(&self) -> &Room {
        if self.in_lair > 0 {
            if let Some(a) = &self.arena {
                return a;
            }
        }
        if let Some(d) = &self.dungeon {
            return &d.rooms[d.cur];
        }
        &self.rooms[self.room]
    }
    fn tile_at(&self, c: i32, r: i32) -> u8 {
        let room = self.cur_room();
        if c < 0 || c >= room.cols() as i32 || r < 0 || r >= room.rows() as i32 {
            return T_FLOOR;
        }
        room.tiles[r as usize][c as usize]
    }
    /// Right and bottom logic edges of the current area (left is 0, top is HUD).
    fn room_wf(&self) -> f32 {
        self.cur_room().wf()
    }
    fn room_hf(&self) -> f32 {
        self.cur_room().hf()
    }

    // ------------------------------------------------------------ camera
    /// Camera position that centres (x, y) inside `room`, clamped to its edges.
    fn cam_target(room: &Room, x: f32, y: f32) -> (f32, f32) {
        let (vw, vh) = (VIEW_W, VIEW_H);
        let cx = if room.wf() <= vw { (room.wf() - vw) / 2.0 } else { (x - vw / 2.0).clamp(0.0, room.wf() - vw) };
        let cy = if room.hf() - HUDF <= vh {
            HUDF + (room.hf() - HUDF - vh) / 2.0
        } else {
            let mut cy = (y - vh / 2.0).clamp(HUDF, room.hf() - vh);
            // Tall set pieces (lair buildings, the monolith, the cottage) stand in the top half
            // of their area: keep them framed while the mage is still on screen.
            if room.gate > 0 || room.special == SP_MONOLITH || room.special == SP_SHOP {
                cy = cy.min(HUDF).max(y - vh + 28.0).clamp(HUDF, room.hf() - vh);
            }
            cy
        };
        (cx, cy)
    }
    /// Follow the mage. `snap` jumps straight there (room entry, teleports).
    fn follow_cam(&mut self, snap: bool) {
        let (tx, mut ty) = Self::cam_target(self.cur_room(), self.pl.x, self.pl.y);
        // Boss fights frame both combatants (the mage always stays on screen).
        let focus = self.boss.as_ref().map(|b| b.y - b.h * 0.5).or_else(|| {
            self.enemies.iter().find(|e| e.mini > 0 && !e.dead && e.k != EK::Zombie).map(|e| e.y - e.h * 0.5 - 12.0)
        });
        if let Some(fy) = focus {
            let room = self.cur_room();
            let (_, by) = Self::cam_target(room, self.pl.x, (fy + self.pl.y) * 0.5 + 8.0);
            let lo = (self.pl.y - VIEW_H + 28.0).max(HUDF);
            ty = by.max(lo).min((self.pl.y - 20.0).max(lo));
        }
        if snap {
            self.cam = (tx, ty);
        } else {
            // Tight SNES-style follow with a touch of smoothing.
            self.cam.0 += (tx - self.cam.0) * 0.3;
            self.cam.1 += (ty - self.cam.1) * 0.3;
            if (tx - self.cam.0).abs() < 0.05 {
                self.cam.0 = tx;
            }
            if (ty - self.cam.1).abs() < 0.05 {
                self.cam.1 = ty;
            }
        }
    }

    // ------------------------------------------------------------ collision
    fn solid_at(&self, x: f32, y: f32) -> bool {
        let (c, r) = tile_of(x, y);
        solid_tile(self.tile_at(c, r)) || self.obj_blocks(c, r)
    }
    fn shot_blocked(&self, x: f32, y: f32) -> bool {
        let (c, r) = tile_of(x, y);
        shot_solid(self.tile_at(c, r)) || self.obj_blocks(c, r)
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
    /// If something solid appeared on top of the mage (a portcullis slamming shut in the
    /// doorway, a block sliding in), push them out toward the middle of the room.
    fn unstick_player(&mut self) {
        let mut p = self.pl;
        if !self.box_solid(p.x, p.y, p.w, p.h) {
            return;
        }
        let a = ang(p.x, p.y, GATE_X, GATE_Y);
        let (dx, dy) = (a.cos(), a.sin());
        for _ in 0..64 {
            p.x += dx;
            p.y += dy;
            if !self.box_solid(p.x, p.y, p.w, p.h) {
                // A little extra so the mage lands clearly inside, not grazing the bars.
                for _ in 0..4 {
                    if self.box_solid(p.x + dx, p.y + dy, p.w, p.h) {
                        break;
                    }
                    p.x += dx;
                    p.y += dy;
                }
                break;
            }
        }
        self.pl.x = p.x;
        self.pl.y = p.y;
    }
    /// Player movement with corner-sliding so doorways are easy to slip into.
    fn move_player(&self, p: &mut Player, dx: f32, dy: f32) -> (bool, bool) {
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
        (hx, hy)
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
            Mode::Awaken => self.update_awaken(),
            Mode::Play => self.update_play(),
            Mode::EnterDungeon => self.update_enter_dungeon(),
            Mode::Descend => self.update_descend(),
            Mode::BossIntro => self.update_boss_intro(),
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
        // The camera follows the mage whenever the world is on screen (not mid-slide).
        let world = matches!(
            self.mode,
            Mode::Play | Mode::Dying | Mode::EnterDungeon | Mode::Descend | Mode::BossIntro | Mode::Awaken
        );
        if world && self.scroll.is_none() && !self.rooms.is_empty() {
            self.follow_cam(false);
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
            self.begin_awaken();
        }
    }

    // ------------------------------------------------------------ overworld
    fn clear_entities(&mut self) {
        self.enemies.clear();
        self.pb.clear();
        self.eb.clear();
        self.items.clear();
        self.parts.clear();
    }
    fn go_play(&mut self, room: usize, x: f32, y: f32, spawn: bool) {
        self.mode = Mode::Play;
        self.paused = false;
        self.in_lair = 0;
        self.arena = None;
        self.dungeon = None;
        self.boss = None;
        self.boss_dead = false;
        self.room = room;
        self.scroll = None;
        self.pan_y = 0.0;
        self.pl = Player::at(x, y);
        self.clear_entities();
        self.msg = None;
        self.shop_armed = [true; 6];
        self.enter_room(spawn);
        self.follow_cam(true);
        let song = self.area_song();
        self.play_song(Some(song));
    }
    fn enter_room(&mut self, spawn: bool) {
        let r = self.room;
        self.rooms[r].visited = true;
        self.s.room = r;
        self.gate_warned = false;
        self.monolith_used = false;
        if spawn {
            self.spawn_room_enemies();
        }
        // Encounters appear whenever their screen is entered, even without regular spawns.
        self.enter_mini_room();
        if self.mode == Mode::Play {
            let song = self.area_song();
            self.play_song(Some(song));
        }
        if r == self.shop_room {
            self.show_msg("WELCOME TO THE VILLAGE! STAND ON AN ITEM AND PRESS A TO BUY. THE INN AND NOTICE BOARD ARE DOWN THE PATH.");
            if let Some(m) = self.msg.as_mut() {
                m.1 = 170;
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
            EK::HoardDragon => (9999.0, 0.0, 34.0, 20.0, 3),
            EK::Dryad => (36.0, 0.7, 10.0, 14.0, 3),
            EK::Treant => (50.0, 0.3, 22.0, 26.0, 4),
            EK::Zombie => (10.0, 0.35, 10.0, 13.0, 3),
            EK::GraveLord => (70.0, 0.45, 16.0, 22.0, 4),
        };
        let th = theme.min(3);
        let per = match k {
            EK::Golem => 2.0,
            EK::Generator => 3.0,
            EK::Dryad | EK::Treant | EK::GraveLord => 8.0,
            EK::Zombie => 2.0,
            EK::HoardDragon => 0.0,
            _ => 1.0,
        };
        let hp = hp + th as f32 * per;
        let el = match k {
            EK::Slime => REGION[th],
            EK::Bat => Elem::Storm,
            EK::Skeleton | EK::Generator | EK::Zombie | EK::GraveLord | EK::HoardDragon | EK::Dryad => Elem::Neutral,
            EK::Imp => Elem::Fire,
            EK::Ghost => Elem::Ice,
            EK::Golem | EK::Treant => Elem::Earth,
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
            spawn: 30, rate, lvl: th as i32, dead: false, st: Status::default(), gen_id: 0, kbx: 0.0, kby: 0.0,
            tip: 0, mini: 0, mode: 0, timer: 0,
        }
    }
    fn free_spot(&mut self, min_d: f32, special: bool) -> Option<(f32, f32)> {
        let (cols, rows, (mx, my)) = {
            let r = self.cur_room();
            (r.cols() as i32, r.rows() as i32, r.center())
        };
        for _ in 0..40 {
            let c = self.rng.irange(1, cols - 2);
            let r = self.rng.irange(1, rows - 2);
            if self.tile_at(c, r) != T_FLOOR || self.obj_blocks(c, r) {
                continue;
            }
            let (x, y) = tile_center(c, r);
            if dist(x, y, self.pl.x, self.pl.y) < min_d {
                continue;
            }
            if special && (x - mx).abs() < 24.0 && (y - my).abs() < 24.0 {
                continue;
            }
            return Some((x, y));
        }
        None
    }
    fn spawn_room_enemies(&mut self) {
        if !self.overworld() {
            return;
        }
        let (th, special, gate, sp, cells) = {
            let r = &self.rooms[self.room];
            (r.theme, r.gate > 0 || r.tank || r.cache || r.shrine.is_some(), r.gate > 0, r.special, (r.cw * r.ch) as i32)
        };
        if sp != SP_NONE {
            return;
        }
        // Wilderness areas are four screens big and hold proportionally bigger hordes.
        let per_cell = 2 + th as i32 + self.rng.irange(0, 1) - gate as i32;
        let n = if cells > 1 { per_cell * cells * 3 / 4 } else { per_cell };
        self.spawn_pack(n, th, special);
        let generators = if cells > 1 { 2 } else { 1 };
        for _ in 0..generators {
            if self.rng.f() < 0.3 + th as f32 * 0.1 {
                if let Some((x, y)) = self.free_spot(90.0, special) {
                    let mut g = self.make_enemy(EK::Generator, x, y, th);
                    g.spawn = 0;
                    self.enemies.push(g);
                }
            }
        }
    }
    fn spawn_pack(&mut self, n: i32, th: usize, special: bool) {
        for _ in 0..n {
            if let Some((x, y)) = self.free_spot(80.0, special) {
                let k = self.rng.pick(POOLS[th.min(3)]);
                let e = self.make_enemy(k, x, y, th);
                self.enemies.push(e);
            }
        }
    }
    /// Walk off the edge of an overworld area through doorway side `d`.
    fn start_scroll(&mut self, d: usize) {
        let r = self.room;
        let along = if d < 2 { self.pl.x } else { self.pl.y };
        let Some(link) = self.rooms[r].link_at(d, along) else {
            self.keep_inside();
            return;
        };
        self.begin_scroll(d, r, link.to, false);
    }
    /// Keep the mage within the current area (no doorway here).
    fn keep_inside(&mut self) {
        let (w, h) = (self.room_wf(), self.room_hf());
        self.pl.x = self.pl.x.clamp(12.0, w - 12.0);
        self.pl.y = self.pl.y.clamp(HUDF + 12.0, h - 12.0);
    }
    /// SNES-style slide from area `from` to area `to` (both in the overworld or both
    /// dungeon rooms). The new area is laid out next to the old one using their grid
    /// positions, the mage is placed just inside the doorway, and the camera glides
    /// from the old view to the new one.
    pub(super) fn begin_scroll(&mut self, d: usize, from: usize, to: usize, dun: bool) {
        let (fr, tr) = match (&self.dungeon, dun) {
            (Some(dg), true) => (&dg.rooms[from], &dg.rooms[to]),
            _ => (&self.rooms[from], &self.rooms[to]),
        };
        let nx = (tr.x as f32 - fr.x as f32) * CELL_W;
        let ny = (tr.y as f32 - fr.y as f32) * CELL_H;
        let (mut x, mut y) = (self.pl.x - nx, self.pl.y - ny);
        match d {
            0 => y = tr.hf() - 10.0,
            1 => y = HUDF + 10.0,
            2 => x = 10.0,
            _ => x = tr.wf() - 10.0,
        }
        x = x.clamp(10.0, tr.wf() - 10.0);
        y = y.clamp(HUDF + 10.0, tr.hf() - 10.0);
        let cam1 = Self::cam_target(tr, x, y);
        let cam0 = self.cam;
        self.scroll = Some(Scroll { d, t: 0.0, from, to, dun, nx, ny, cam0, cam1 });
        self.clear_entities();
        self.pl.x = x;
        self.pl.y = y;
    }

    fn update_needs(&mut self) {
        self.tick_player_status();
        if self.mode != Mode::Play {
            return;
        }
        self.s.food = (self.s.food - 1.0 / 72.0).max(0.0);
        self.s.mp = (self.s.mp + MP_REGEN).min(self.s.max_mp as f32);
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
        if self.fade > 0 {
            self.fade -= 1;
        }
        if let Some(sc) = self.scroll.as_mut() {
            sc.t += 1.0 / 36.0;
            if sc.t >= 1.0 {
                let (to, dun, cam1) = (sc.to, sc.dun, sc.cam1);
                self.scroll = None;
                if dun {
                    if let Some(d) = self.dungeon.as_mut() {
                        d.cur = to;
                    }
                    self.enter_droom();
                } else {
                    self.room = to;
                    self.enter_room(true);
                }
                self.cam = cam1;
            }
            return;
        }
        self.update_needs();
        if self.mode != Mode::Play {
            return;
        }
        self.unstick_player();
        let mut p = self.pl;
        if p.inv > 0 {
            p.inv -= 1;
        }
        if p.cast > 0 {
            p.cast -= 1;
        }
        let mut dx = (self.held(Btn::Right) as i32 - self.held(Btn::Left) as i32) as f32;
        let mut dy = (self.held(Btn::Down) as i32 - self.held(Btn::Up) as i32) as f32;
        let (ix, iy) = (dx, dy);
        p.moving = dx != 0.0 || dy != 0.0;
        let mut blocked = (false, false);
        if dx != 0.0 || dy != 0.0 {
            let l = dx.hypot(dy);
            dx /= l;
            dy /= l;
            // Holding CAST locks facing so the mage can strafe (Gauntlet style). With
            // twin-stick aiming the right stick decides the facing instead.
            if !self.held(Btn::Fire) && self.inp.aim().is_none() {
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
            blocked = self.move_player(&mut p, dx * sp, dy * sp);
        }
        // Twin-stick aiming (right stick, e.g. RG35XX Pro): face and cast where it points.
        if let Some((ax, ay)) = self.inp.aim() {
            p.fx = ax;
            p.fy = ay;
            p.dir = if ax.abs() >= ay.abs() {
                if ax > 0.0 { b'r' } else { b'l' }
            } else if ay > 0.0 {
                b'd'
            } else {
                b'u'
            };
        }
        self.pl = p;
        if self.in_lair == 0 {
            let (rw, rh) = (self.room_wf(), self.room_hf());
            let exit = if p.x < 4.0 {
                Some(3)
            } else if p.x > rw - 4.0 {
                Some(2)
            } else if p.y < HUDF + 4.0 {
                Some(0)
            } else if p.y > rh - 4.0 {
                Some(1)
            } else {
                None
            };
            if let Some(d) = exit {
                if self.dungeon.is_some() {
                    self.dungeon_exit(d);
                } else {
                    self.start_scroll(d);
                }
                return;
            }
        }
        if self.dungeon.is_some() && self.in_lair == 0 {
            self.dungeon_player(ix, iy, blocked);
            if self.mode != Mode::Play {
                return;
            }
        }
        if self.overworld() {
            self.overworld_bump(ix, iy, blocked);
        }
        if self.pl.cd > 0 {
            self.pl.cd -= 1;
        }
        if self.pl.scd > 0 {
            self.pl.scd -= 1;
        }
        let aiming = self.inp.aim().is_some();
        if (self.held(Btn::Fire) || aiming) && self.pl.cd <= 0 && !self.boss_dead && !self.on_pedestal() {
            self.cast_bolt();
        }
        if self.held(Btn::Sub) && self.pl.scd <= 0 && !self.boss_dead {
            self.cast_spell();
        }
        if self.p(Btn::Potion) {
            // The same button cures poison first when you carry an antidote.
            if self.poison > 0 && self.s.antidotes > 0 {
                self.use_antidote();
            } else {
                self.drink_potion();
            }
        }
        self.update_pbullets();
        self.update_enemies();
        self.update_boss();
        self.update_minis();
        self.update_ebullets();
        self.update_items();
        self.update_parts();
        if self.mode == Mode::Play {
            if self.overworld() {
                self.room_objects();
            } else if self.in_lair == 0 {
                self.dungeon_update();
            } else {
                self.update_lair_state();
            }
        }
        if let Some(m) = self.msg.as_mut() {
            m.1 -= 1;
            if m.1 <= 0 {
                self.msg = None;
            }
        }
        self.dungeon_flush();
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
        self.items.push(Item { kind, x, y, val, el, life: 600, dead: false, tag: 0 });
    }
    fn drop_from(&mut self, e: &Enemy) {
        // Encounter bosses hand out their own rewards (minis.rs).
        if e.mini != 0 {
            return;
        }
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
        } else if r < 0.49 {
            self.add_item(IK::Antidote, e.x, e.y, 0, Elem::Neutral);
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
    /// Drink a carried potion: refills mana completely.
    fn drink_potion(&mut self) {
        let (x, y) = (self.pl.x, self.pl.y);
        if self.s.potions <= 0 {
            self.float("NO POTIONS", x - 40.0, y - 18.0, rgb(0x747474));
            self.sfx(Sfx::Deny);
            return;
        }
        if self.s.mp >= self.s.max_mp as f32 - 0.5 {
            self.float("MANA IS FULL", x - 48.0, y - 18.0, rgb(0x3cbcfc));
            return;
        }
        self.s.potions -= 1;
        self.s.mp = self.s.max_mp as f32;
        self.float("MANA RESTORED", x - 52.0, y - 18.0, rgb(0x3cbcfc));
        self.part(x, y, 0.0, 0.0, 16, rgb(0x3cbcfc), 1, PK::Glow(14.0));
        for i in 0..8 {
            let ox = (i as f32 - 3.5) * 2.0;
            self.part(x + ox, y + 4.0, 0.0, -0.6 - (i % 3) as f32 * 0.2, 24, rgb(0xa4e4fc), 1, PK::Dot);
        }
        self.sfx(Sfx::Heal);
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
                if it.tag == ITEM_HOARD {
                    self.hoard_coin_taken();
                }
            }
            IK::GoldenApple => {
                self.s.food = 100.0;
                self.s.hp = self.s.max_hp;
                self.float("GOLDEN APPLE!", x - 52.0, y - 18.0, rgb(0xfce040));
                self.part(x, y, 0.0, 0.0, 18, rgb(0xfce040), 1, PK::Glow(16.0));
                self.sfx(Sfx::Fanfare);
            }
            IK::Antidote => {
                if self.s.antidotes < 3 {
                    self.s.antidotes += 1;
                    self.float(format!("ANTIDOTE {}/3", self.s.antidotes), x - 44.0, y - 18.0, rgb(0x58d854));
                } else if self.poison > 0 {
                    self.cure_poison();
                }
                self.sfx(Sfx::Pickup);
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
                if self.s.potions < MAX_POTIONS {
                    self.s.potions += 1;
                    self.float(format!("POTION {}/{}", self.s.potions, MAX_POTIONS), x - 40.0, y - 18.0, rgb(0x3cbcfc));
                } else {
                    self.s.mp = (self.s.mp + 30.0).min(self.s.max_mp as f32);
                    self.float("+30 MP", x - 24.0, y - 18.0, rgb(0x3cbcfc));
                }
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

    // ------------------------------------------------------------ overworld objects
    fn gate_state(&self, n: usize) -> u8 {
        if self.s.cleared[n] {
            2
        } else if n == 1 || self.s.cleared[n - 1] {
            1
        } else {
            0
        }
    }
    fn room_objects(&mut self) {
        let ri = self.room;
        let (px, py) = (self.pl.x, self.pl.y);
        let (gx, gy) = self.rooms[ri].center();
        let near = (px - gx).abs() < 12.0 && (py - gy).abs() < 12.0;
        let (gate, tank, cache, chest, shrine, special) = {
            let r = &self.rooms[ri];
            (r.gate, r.tank, r.cache, r.chest, r.shrine, r.special)
        };
        if gate > 0 {
            // The building's doorway sits just below its footprint.
            let at_door = (px - GATE_X).abs() < 12.0 && py < GATE_Y + 8.0;
            if at_door {
                if self.gate_state(gate) == 1 {
                    self.begin_enter_dungeon(gate);
                    return;
                }
                if !self.gate_warned {
                    self.gate_warned = true;
                    if self.s.cleared[gate] {
                        self.show_msg("THIS PLACE IS QUIET NOW. ITS GUARDIAN HAS FALLEN.");
                    } else if gate == 6 {
                        self.show_msg("THE DARK TOWER IS SEALED. GATHER ALL FIVE RUNES.");
                    } else {
                        self.show_msg(format!("A MAGIC SEAL BARS THE WAY. CONQUER LAIR {} FIRST.", gate - 1));
                    }
                    self.sfx(Sfx::Deny);
                }
            } else {
                self.gate_warned = false;
            }
        }
        if special == SP_MONOLITH {
            let touching = (px - 128.0).abs() < 18.0 && py < 146.0;
            if touching && !self.monolith_used {
                self.monolith_used = true;
                self.s.hp = self.s.max_hp;
                self.s.mp = self.s.max_mp as f32;
                let runes = (1..=5).filter(|&i| self.s.cleared[i]).count();
                self.show_msg(format!(
                    "THE MONOLITH'S LIGHT RESTORES YOU. RUNES AWAKENED: {} OF 5. SEEK THE LAIRS BEYOND THE VILLAGE.",
                    runes
                ));
                self.flash = 6;
                self.sfx(Sfx::Heal);
                self.save();
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
        if ri == self.shop_room {
            self.shop();
            self.village();
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
        self.village_talk_spot()
            || (self.overworld()
                && self.room == self.shop_room
                && (0..4).any(|i| (self.pl.x - Self::shop_x(i)).abs() < 10.0 && (self.pl.y - GATE_Y).abs() < 12.0))
    }
    /// Shop pedestals: roast, mana potion, antidote, heart container.
    fn shop_x(i: usize) -> f32 {
        56.0 + i as f32 * 48.0
    }
    fn shop_prices(&self) -> [i32; 4] {
        [15, 25, 20, self.s.heart_price]
    }
    fn shop(&mut self) {
        let prices = self.shop_prices();
        for i in 0..4 {
            let x = Self::shop_x(i);
            let near = (self.pl.x - x).abs() < 10.0 && (self.pl.y - GATE_Y).abs() < 12.0;
            if !near {
                self.shop_armed[i] = true;
                continue;
            }
            if self.shop_armed[i] {
                self.shop_armed[i] = false;
                let what = ["A HEARTY ROAST", "A MANA POTION TO CARRY", "AN ANTIDOTE FOR POISON", "A HEART CONTAINER"][i];
                self.show_msg(format!("{} FOR {} GOLD. PRESS A TO BUY.", what, prices[i]));
                if let Some(m) = self.msg.as_mut() {
                    m.1 = 120;
                }
            }
            if !self.p(Btn::Fire) {
                continue;
            }
            if i == 1 && self.s.potions >= MAX_POTIONS {
                self.show_msg("YOUR PACK CAN'T HOLD ANY MORE POTIONS.");
                self.sfx(Sfx::Deny);
                continue;
            }
            if i == 2 && self.s.antidotes >= 3 && self.poison <= 0 {
                self.show_msg("YOU CAN ONLY CARRY THREE ANTIDOTES.");
                self.sfx(Sfx::Deny);
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
                    self.s.potions += 1;
                    self.show_msg(format!("A MANA POTION FOR YOUR PACK ({}/{}). PRESS Y OR C TO DRINK IT.", self.s.potions, MAX_POTIONS));
                    self.sfx(Sfx::Pickup);
                }
                2 => {
                    if self.poison > 0 && self.s.antidotes >= 3 {
                        self.cure_poison();
                    } else {
                        self.s.antidotes += 1;
                        self.show_msg(format!("AN ANTIDOTE FOR YOUR PACK ({}/3). IF POISONED, PRESS Y OR C TO DRINK.", self.s.antidotes));
                    }
                    self.sfx(Sfx::Pickup);
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

    // ------------------------------------------------------------ progression
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
            self.fade = 30;
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
}

const INTRO: [&str; 11] = [
    "THE DARK SORCERER HAS",
    "STOLEN THE LIGHT OF THE",
    "REALM AND SEALED IT IN",
    "HIS TOWER.",
    "",
    "FIVE MONSTERS GUARD THE",
    "RUNES THAT OPEN IT.",
    "MAGE, THE OLD MONOLITH",
    "CALLS YOU. WAKE UP.",
    "",
    "AND DON'T FORGET TO EAT.",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_round_trip_keeps_everything() {
        let mut s = SaveData::fresh();
        s.gold = 1234;
        s.cleared[2] = true;
        s.tanks = vec![3, 9];
        s.dprog[1] = 0b1011;
        s.el = 2;
        s.potions = 4;
        s.mini_seen = 0b11110;
        s.mini_done = 0b00100;
        s.hoard_left = 7;
        s.zombies = 3;
        s.antidotes = 2;
        let back = SaveData::from_text(&s.to_text()).expect("parse");
        assert_eq!(back, s);
    }

    #[test]
    fn saves_from_the_old_world_layout_are_ignored() {
        let old = "max_hp=24\nhp=20\ngold=77\ncleared=0,1,0,0,0,0,0\nroom=31\n";
        assert!(SaveData::from_text(old).is_none(), "pre-SNES saves must not load into the new map");
        assert!(SaveData::from_text("version=1\nmax_hp=24\n").is_none());
    }

    #[test]
    fn saves_without_dungeon_progress_still_load() {
        let old = "version=2\nmax_hp=24\nhp=20\ngold=77\ncleared=0,1,0,0,0,0,0\nroom=31\n";
        let s = SaveData::from_text(old).expect("parse");
        assert_eq!(s.gold, 77);
        assert!(s.cleared[1]);
        assert_eq!(s.dprog, [0; 7]);
        // Saves from before the overworld encounters: nothing discovered or finished yet.
        assert_eq!((s.mini_seen, s.mini_done, s.hoard_left, s.zombies, s.antidotes), (0, 0, -1, 0, 0));
    }

    #[test]
    fn elemental_multipliers() {
        assert_eq!(mult(Elem::Fire, Elem::Ice), 2.0);
        assert_eq!(mult(Elem::Fire, Elem::Fire), 0.5);
        assert_eq!(mult(Elem::Storm, Elem::Earth), 2.0);
        assert_eq!(mult(Elem::Storm, Elem::Neutral), 1.0);
    }
}
