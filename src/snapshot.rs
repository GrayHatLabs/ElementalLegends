//! Headless test modes (no window, no audio device):
//!
//! * `elementallegends --selftest [dir]` plays scripted scenarios through the real
//!   game loop (real input, real collisions) and checks the acceptance criteria.
//!   Exits non-zero on failure. With `dir`, also writes PNG screenshots.
//! * `elementallegends --snapshot <dir>` is the same run, always writing screenshots.
//!
//! Headless checks prove logic and rendering output, not how it looks or feels on
//! real hardware; see IMPLEMENTATION_STATUS.md for what still needs eyes-on testing.
use crate::game::{Btn, Game, Input, Mode};
use crate::gfx::{Screen, H, W};
use std::path::{Path, PathBuf};

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// Minimal PNG writer (stored/uncompressed deflate blocks).
fn write_png(path: &Path, px: &[u32], w: usize, h: usize) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            let p = px[y * w + x];
            raw.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }
    let mut z = vec![0x78, 0x01];
    for (i, chunk) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8);
        let n = chunk.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(chunk);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |ty: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = ty.to_vec();
        c.extend_from_slice(data);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    std::fs::write(path, out)
}

const T_ICE: u8 = 3;
const T_FLOOR: u8 = 0;
const T_STAIRS: u8 = 9;
const T_DECOR: u8 = 10;
const R_ENTRY: usize = 0;
const R_HUB: usize = 1;
const R_WEST: usize = 2;
const R_EAST: usize = 3;
const R_STAIRS: usize = 4;
const D_KEY: u8 = 1;
const D_DOOR: u8 = 2;
const D_EAST: u8 = 8;

fn tc(c: i32, r: i32) -> (f32, f32) {
    ((c * 16 + 8) as f32, (32 + r * 16 + 8) as f32)
}

struct T {
    g: Game,
    scr: Screen,
    input: Input,
    dir: Option<PathBuf>,
    fails: Vec<String>,
    passes: usize,
}

impl T {
    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.g.update(&self.input);
            self.input.clear_pressed();
        }
    }
    fn release(&mut self) {
        self.input.release_all();
    }
    fn tap(&mut self, b: Btn) {
        self.input.set_key(b, true);
        self.frames(1);
        self.input.set_key(b, false);
        self.frames(1);
    }
    fn shot(&mut self, name: &str) {
        let Some(dir) = self.dir.clone() else { return };
        self.g.draw(&mut self.scr);
        let p = dir.join(format!("{name}.png"));
        write_png(&p, &self.scr.px, W as usize, H as usize).expect("write png");
    }
    fn check(&mut self, ok: bool, what: &str) -> bool {
        if ok {
            self.passes += 1;
            println!("  PASS  {what}");
        } else {
            println!("  FAIL  {what}   [{}]", self.g.debug_summary());
            self.fails.push(what.to_string());
        }
        ok
    }
    /// Steer toward a point with the d-pad (axis at a time when blocked).
    fn walk_to(&mut self, tx: f32, ty: f32, max: usize) -> bool {
        for _ in 0..max {
            let (x, y) = self.g.debug_player();
            let (dx, dy) = (tx - x, ty - y);
            if dx.abs() < 2.0 && dy.abs() < 2.0 {
                self.release();
                return true;
            }
            self.input.set_key(Btn::Left, dx < -1.5);
            self.input.set_key(Btn::Right, dx > 1.5);
            self.input.set_key(Btn::Up, dy < -1.5);
            self.input.set_key(Btn::Down, dy > 1.5);
            self.frames(1);
            if self.g.debug_mode() != Mode::Play {
                break;
            }
        }
        self.release();
        false
    }
    fn hold_until(&mut self, b: Btn, max: usize, pred: impl Fn(&Game) -> bool) -> bool {
        self.input.set_key(b, true);
        for _ in 0..max {
            self.frames(1);
            if pred(&self.g) {
                self.input.set_key(b, false);
                return true;
            }
        }
        self.input.set_key(b, false);
        false
    }
    fn fire_from(&mut self, x: f32, y: f32, dir: u8) {
        self.g.debug_set_player(x, y, dir);
        self.frames(1);
        self.tap(Btn::Fire);
        self.frames(28);
    }
    fn cur(&self) -> usize {
        self.g.debug_dungeon().map_or(99, |d| d.1)
    }
    fn prog(&self) -> u8 {
        self.g.debug_dungeon().map_or(0, |d| d.2)
    }
    fn go_room(&mut self, b: Btn, target: usize) -> bool {
        self.hold_until(b, 400, move |g| g.debug_dungeon().map_or(false, |d| d.1 == target) && !g.debug_scrolling())
    }
    fn obj(&self, kind: &str) -> Option<(i32, i32, bool, bool)> {
        self.g.debug_objs().into_iter().find(|o| o.kind == kind).map(|o| (o.c, o.r, o.on, o.visible))
    }
}

pub fn run(dir: Option<&str>) -> i32 {
    let dir = dir.map(|d| {
        std::fs::create_dir_all(d).expect("create snapshot dir");
        PathBuf::from(d)
    });
    let mut t = T { g: Game::new(None), scr: Screen::new(), input: Input::default(), dir, fails: vec![], passes: 0 };
    t.g.debug_no_save();
    t.g.debug_seed(0xE1E7);

    // ---------------------------------------------------------------- title & monolith start
    println!("[start] monolith opening");
    t.frames(30);
    t.shot("01_title");
    t.g.debug_force_new_game_menu();
    t.tap(Btn::Start);
    t.frames(10);
    t.tap(Btn::Right);
    t.shot("02_choose");
    t.input.set_key(Btn::Left, true);
    t.frames(1);
    t.release();
    t.tap(Btn::Start); // Fire
    t.frames(60);
    t.shot("03_intro");
    t.tap(Btn::Start);
    t.frames(2);
    let (start, shop) = t.g.debug_rooms();
    t.check(t.g.debug_mode() == Mode::Awaken, "new game opens with the monolith awakening cinematic");
    t.check(t.g.debug_room() == start && t.g.debug_room_special(start) == 1, "the game starts in the monolith room");
    t.check(start != shop && t.g.debug_room_special(shop) == 2, "the shop is a different room from the start");
    for (f, name) in [(50, "04_awaken_pan"), (80, "05_awaken_runes"), (70, "06_awaken_beam")] {
        t.frames(f);
        t.shot(name);
    }
    t.frames(80);
    t.check(t.g.debug_mode() == Mode::Play, "control is restored after the opening");
    t.shot("07_monolith_room");
    t.g.debug_set_player(128.0, 150.0, b'u');
    t.hold_until(Btn::Up, 40, |g| g.debug_msg().map_or(false, |m| m.contains("RESTORES")));
    t.check(t.g.debug_msg().map_or(false, |m| m.contains("RESTORES")), "touching the monolith restores the mage");
    t.shot("08_monolith_touch");
    let f0 = t.g.debug_food();
    t.frames(720);
    t.check(t.g.debug_food() < f0 - 8.0, "food still drains over time");

    // Walk to the shop through the connecting doorway.
    let (sx, sy) = t.g.debug_room_xy(start);
    let (hx, hy) = t.g.debug_room_xy(shop);
    let (btn, door) = if hx > sx {
        (Btn::Right, (238.0, 136.0))
    } else if hx < sx {
        (Btn::Left, (18.0, 136.0))
    } else if hy < sy {
        (Btn::Up, (128.0, 44.0))
    } else {
        (Btn::Down, (128.0, 226.0))
    };
    t.g.debug_set_player(128.0, 150.0, b'd');
    t.walk_to(128.0, 136.0, 200);
    t.walk_to(door.0, door.1, 300);
    let reached = t.hold_until(btn, 200, move |g| g.debug_room() == shop && !g.debug_scrolling());
    t.check(reached, "the shop is reached by walking out of the monolith clearing");
    t.frames(10);
    t.shot("09_shop");
    t.g.debug_set_gold(100);
    t.g.debug_set_player(80.0, 150.0, b'u');
    t.walk_to(80.0, 136.0, 60);
    t.frames(2);
    t.tap(Btn::Fire);
    t.check(t.g.debug_gold() == 85, "shop purchasing still works (roast for 15 gold)");

    // Mana is a resource: slow regen, potions carried and drunk on demand.
    println!("[mana] regen and carried potions");
    t.g.debug_set_mp(0.0, 40);
    t.frames(600);
    let mp = t.g.debug_mp();
    t.check((4.0..=6.0).contains(&mp), &format!("mana regenerates at about 0.5 MP per second ({mp:.1} MP after 10 s)"));
    let p0 = t.g.debug_potions();
    t.g.debug_set_mp(10.0, 40);
    t.g.debug_set_player(128.0, 150.0, b'u');
    t.walk_to(128.0, 136.0, 60);
    t.frames(2);
    t.tap(Btn::Fire);
    t.check(t.g.debug_potions() == p0 + 1 && t.g.debug_gold() == 60, "buying a potion puts it in the pack (25 gold)");
    t.check(t.g.debug_mp() < 20.0, "buying a potion does not drink it");
    t.g.debug_set_player(128.0, 170.0, b'u');
    t.frames(2);
    t.tap(Btn::Potion);
    t.check(t.g.debug_mp() >= 39.5 && t.g.debug_potions() == p0, "the potion button drinks one and refills mana");
    t.tap(Btn::Potion);
    t.check(t.g.debug_potions() == p0, "drinking with full mana doesn't waste a potion");
    t.g.debug_set_potions(2);
    let (x, y) = t.g.debug_player();
    t.g.debug_add_potion_item(x, y);
    t.frames(2);
    t.check(t.g.debug_potions() == 3, "picking up a potion adds it to the pack");
    t.g.debug_set_potions(5);
    t.g.debug_set_mp(0.0, 40);
    t.g.debug_add_potion_item(x, y);
    t.frames(2);
    t.check(t.g.debug_potions() == 5 && t.g.debug_mp() >= 29.0, "with a full pack a found potion restores 30 MP instead");
    t.g.debug_set_gold(100);
    t.g.debug_set_player(128.0, 150.0, b'u');
    t.walk_to(128.0, 136.0, 60);
    t.frames(2);
    t.tap(Btn::Fire);
    t.check(t.g.debug_gold() == 100, "the shop won't sell a potion when the pack is full");
    t.shot("09b_potion_hud");

    // ---------------------------------------------------------------- fire: travelling bolt + burning DOT
    println!("[fire] firebolt and burning");
    t.g.debug_god();
    t.g.debug_set_element(0);
    t.g.debug_play_room(start, 40.0, 200.0);
    t.frames(5);
    let id = t.g.debug_spawn(2, 130.0, 200.0, 60.0, true); // skeleton: neutral, holds still
    t.g.debug_set_player(40.0, 200.0, b'r');
    t.tap(Btn::Fire);
    let mut xs = vec![];
    for i in 0..90 {
        if let Some(&(x, _)) = t.g.debug_player_bolts().first() {
            xs.push(x);
        }
        if i == 14 {
            t.shot("10_firebolt_flight");
        }
        t.frames(1);
        if t.g.debug_enemy(id).map_or(false, |e| e.burn > 0) {
            break;
        }
    }
    let travelled = xs.last().copied().unwrap_or(0.0) - xs.first().copied().unwrap_or(0.0);
    t.check(xs.len() >= 10 && travelled > 60.0, "firebolt visibly travels before hitting");
    let e = t.g.debug_enemy(id);
    t.check(e.as_ref().map_or(false, |e| e.burn > 0), "a firebolt hit ignites the enemy");
    let hp_hit = e.map_or(0.0, |e| e.hp);
    t.frames(20);
    t.shot("11_burning");
    t.check(t.g.debug_flame_parts() > 0, "burning enemies emit flames and embers");
    t.frames(100);
    let hp_later = t.g.debug_enemy(id).map_or(0.0, |e| e.hp);
    t.check(hp_later < hp_hit - 1.0, "burning deals damage over time with no further hits");
    t.frames(200);
    t.check(t.g.debug_enemy(id).map_or(false, |e| e.burn == 0), "the burn expires");
    t.shot("12_burn_faded");

    // ---------------------------------------------------------------- ice: slow, freeze, shatter
    println!("[ice] chill, freeze, shatter");
    t.g.debug_kill_enemies();
    t.frames(40);
    t.g.debug_set_element(1);
    let id = t.g.debug_spawn(2, 150.0, 200.0, 80.0, true);
    t.g.debug_set_player(60.0, 200.0, b'r');
    t.tap(Btn::Fire);
    t.hold_until(Btn::Mute, 60, move |g| g.debug_enemy(id).map_or(false, |e| e.chill > 0));
    t.input.release_all();
    let e = t.g.debug_enemy(id);
    t.check(e.as_ref().map_or(false, |e| e.chill > 0 && e.freeze == 0), "first ice hit chills (slows) the enemy");
    t.frames(6);
    t.shot("13_chilled");
    t.frames(14);
    t.tap(Btn::Fire);
    t.hold_until(Btn::Mute, 60, move |g| g.debug_enemy(id).map_or(false, |e| e.freeze > 0));
    t.input.release_all();
    t.check(t.g.debug_enemy(id).map_or(false, |e| e.freeze > 0), "second ice hit on a chilled enemy freezes it");
    t.frames(16);
    t.shot("14_frozen");
    let mut shards = 0;
    for _ in 0..200 {
        t.frames(1);
        if t.g.debug_enemy(id).map_or(true, |e| e.freeze == 0) {
            t.frames(3);
            shards = t.g.debug_shards();
            t.shot("15_shatter");
            break;
        }
    }
    t.check(shards > 6, "the freeze ends with an ice-shatter burst of falling fragments");

    // ---------------------------------------------------------------- bolt range
    println!("[range] bolt range vs power-ups and health");
    t.g.debug_kill_enemies();
    t.frames(40);
    let measure = |t: &mut T, lv: i32, hp: i32| -> f32 {
        t.g.debug_set_spell_lv(lv);
        t.g.debug_set_hp(hp, 40);
        t.g.debug_set_player(20.0, 150.0, b'r');
        t.frames(30);
        t.tap(Btn::Fire);
        let mut far = 0.0f32;
        for _ in 0..90 {
            for (x, _) in t.g.debug_player_bolts() {
                far = far.max(x - 20.0);
            }
            t.frames(1);
        }
        far
    };
    let short = measure(&mut t, 1, 40);
    let weak = measure(&mut t, 1, 4);
    let full = measure(&mut t, 3, 40);
    println!("  ranges: lv1 full hp {short:.0}px, lv1 low hp {weak:.0}px, lv3 full hp {full:.0}px");
    t.check(short < 130.0, "without power-ups a bolt does not cross the whole screen");
    t.check(weak < short * 0.7, "bolts get shorter as the mage's health drops");
    t.check(full > 200.0, "with the top magic power-up bolts reach across the screen");
    t.g.debug_set_spell_lv(1);
    t.g.debug_god();

    // ---------------------------------------------------------------- dungeons, puzzles, stairs, bosses
    for n in 1..=6 {
        run_dungeon(&mut t, n);
    }

    println!("\n{} passed, {} failed", t.passes, t.fails.len());
    for f in &t.fails {
        println!("  failed: {f}");
    }
    if t.fails.is_empty() {
        0
    } else {
        1
    }
}

fn run_dungeon(t: &mut T, n: usize) {
    println!("[dungeon {n}]");
    t.release();
    t.g.debug_clear_to(n);
    t.g.debug_god();
    let gate = t.g.debug_gate_room(n);
    t.g.debug_play_room(gate, 128.0, 176.0);
    t.frames(4);
    t.check(t.g.debug_tile(7, 4) == T_DECOR, &format!("dungeon {n}: entrance building occupies its footprint"));
    t.shot(&format!("20_building_{n}"));
    let entered = t.hold_until(Btn::Up, 120, |g| g.debug_mode() == Mode::EnterDungeon);
    t.check(entered, &format!("dungeon {n}: walking into the doorway starts the entry transition"));
    t.frames(30);
    t.shot(&format!("21_entering_{n}"));
    t.frames(40);
    t.check(t.cur() == R_ENTRY && t.g.debug_mode() == Mode::Play, &format!("dungeon {n}: inside the entrance hall"));
    t.shot(&format!("22_dungeon_entry_{n}"));
    let (hub_combat, west, east) = t.g.debug_puzzles().unwrap();
    println!("  layout: hub combat={hub_combat} west={west} east={east}");
    food_room(t, n);
    t.walk_to(128.0, 60.0, 200);
    { let ok = t.go_room(Btn::Up, R_HUB); t.check(ok, &format!("dungeon {n}: entrance -> hall")); }
    t.frames(4);
    if hub_combat {
        t.check(t.g.debug_dungeon_sealed(), &format!("dungeon {n}: hall seals its doors for a combat challenge"));
        t.check(!t.g.debug_player_stuck(), &format!("dungeon {n}: the mage is not trapped in the sealed hall door"));
        t.shot(&format!("23_hall_sealed_{n}"));
        t.frames(30);
        t.g.debug_kill_enemies();
        t.frames(4);
        t.check(!t.g.debug_dungeon_sealed(), &format!("dungeon {n}: defeating the monsters reopens the hall"));
    }
    t.g.debug_kill_enemies();
    // Locked door before the key.
    t.g.debug_set_player(128.0, 70.0, b'u');
    t.hold_until(Btn::Up, 40, |_| false);
    t.check(t.prog() & D_DOOR == 0 && t.cur() == R_HUB, &format!("dungeon {n}: the great door is locked without a key"));

    // West chamber: the key.
    t.g.debug_set_player(128.0, 136.0, b'l');
    { let ok = t.go_room(Btn::Left, R_WEST); t.check(ok, &format!("dungeon {n}: hall -> west chamber")); }
    t.frames(4);
    t.shot(&format!("24_west_{n}_{west}"));
    solve_room(t, n, false, &west);
    t.check(t.g.debug_dungeon().map_or(0, |d| d.3) == 1 && t.prog() & D_KEY != 0, &format!("dungeon {n}: {west} puzzle yields the key"));
    t.shot(&format!("25_west_solved_{n}"));

    // East chamber: the staircase seal.
    t.g.debug_set_player(200.0, 136.0, b'r');
    { let ok = t.go_room(Btn::Right, R_HUB); t.check(ok, &format!("dungeon {n}: west -> hall")); }
    t.g.debug_kill_enemies();
    t.g.debug_set_player(128.0, 136.0, b'r');
    { let ok = t.go_room(Btn::Right, R_EAST); t.check(ok, &format!("dungeon {n}: hall -> east chamber")); }
    t.frames(4);
    t.shot(&format!("26_east_{n}_{east}"));
    solve_room(t, n, true, &east);
    t.check(t.prog() & D_EAST != 0, &format!("dungeon {n}: {east} puzzle breaks the staircase seal"));
    t.shot(&format!("27_east_solved_{n}"));

    // Unlock the great door and reach the stairs.
    t.g.debug_set_player(56.0, 136.0, b'l');
    { let ok = t.go_room(Btn::Left, R_HUB); t.check(ok, &format!("dungeon {n}: east -> hall")); }
    t.g.debug_kill_enemies();
    t.g.debug_set_player(128.0, 136.0, b'u');
    t.walk_to(128.0, 60.0, 200);
    let opened = t.hold_until(Btn::Up, 120, |g| g.debug_dungeon().map_or(false, |d| d.2 & D_DOOR != 0));
    t.check(opened, &format!("dungeon {n}: the key unlocks the great door"));
    { let ok = t.go_room(Btn::Up, R_STAIRS); t.check(ok, &format!("dungeon {n}: hall -> staircase room")); }
    t.frames(4);
    t.check(t.g.debug_tile(7, 3) == T_STAIRS, &format!("dungeon {n}: the staircase is unsealed"));
    t.shot(&format!("28_stairs_room_{n}"));

    // Staircase cinematic into the boss arena.
    t.walk_to(128.0, 150.0, 200);
    let desc = t.hold_until(Btn::Up, 200, |g| g.debug_mode() == Mode::Descend);
    t.check(desc, &format!("dungeon {n}: stepping onto the stairs starts the descent"));
    let before = t.g.debug_player();
    t.input.set_key(Btn::Left, true); // input must not interrupt the cinematic
    t.frames(40);
    t.release();
    t.shot(&format!("29_descend_a_{n}"));
    let (px, _) = t.g.debug_player();
    t.check((px - 128.0).abs() < 2.0 && (before.0 - 128.0).abs() < 20.0, &format!("dungeon {n}: the mage walks down the stairs on their own"));
    t.frames(60);
    t.shot(&format!("30_descend_b_{n}"));
    let intro = t.hold_until(Btn::Mute, 120, |g| g.debug_mode() == Mode::BossIntro);
    t.input.release_all();
    t.check(intro, &format!("dungeon {n}: the descent leads to the boss entrance"));
    t.frames(60);
    t.shot(&format!("31_boss_intro_{n}"));
    let play = t.hold_until(Btn::Mute, 150, |g| g.debug_mode() == Mode::Play);
    t.input.release_all();
    t.check(play, &format!("dungeon {n}: control returns when the boss fight begins"));
    boss_fight(t, n);
}

fn solve_room(t: &mut T, n: usize, east: bool, puz: &str) {
    t.g.debug_kill_enemies();
    t.frames(2);
    let dir_btn = if east { Btn::Right } else { Btn::Left };
    let face = if east { b'r' } else { b'l' };
    match puz {
        "Combat" => {
            let (px, _) = t.g.debug_player();
            let inside = if east { px > 20.0 } else { px < 236.0 };
            t.check(!t.g.debug_player_stuck() && inside, &format!("dungeon {n}: the slamming door pushes the mage into the room, not into the bars"));            t.check(t.g.debug_dungeon_sealed() || t.g.debug_enemy_count() == 0, &format!("dungeon {n}: combat chamber sealed"));
            t.frames(20);
            t.g.debug_kill_enemies();
            t.frames(4);
        }
        "Torches" => {
            let (x, y) = tc(8, 6);
            t.g.debug_set_player(x, y + 6.0, b'u');
            t.hold_until(Btn::Up, 20, |g| g.debug_element() == 0);
            t.check(t.g.debug_element() == 0, &format!("dungeon {n}: the fire shrine grants fire for the braziers"));
            for &(c, r) in &[(3, 3), (12, 3), (3, 9), (12, 9)] {
                let (x, y) = tc(c, r);
                if r < 6 {
                    t.fire_from(x, y + 32.0, b'u');
                } else {
                    t.fire_from(x, y - 32.0, b'd');
                }
            }
            t.frames(4);
        }
        "Plates" => {
            // Block A: up the column, then one push onto the plate.
            let pushes: [((i32, i32), Btn, (i32, i32), (i32, i32), Btn, (i32, i32)); 2] =
                [((5, 9), Btn::Up, (5, 3), (6, 3), Btn::Left, (4, 3)), ((10, 9), Btn::Up, (10, 3), (9, 3), Btn::Right, (11, 3))];
            for (stand1, b1, goal1, stand2, b2, goal2) in pushes {
                let (x, y) = tc(stand1.0, stand1.1);
                t.g.debug_set_player(x, y, b'u');
                let g1 = goal1;
                let moved = t.hold_until(b1, 400, move |g| g.debug_objs().iter().any(|o| o.kind == "block" && (o.c, o.r) == g1));
                t.frames(10);
                let (x, y) = tc(stand2.0, stand2.1);
                t.g.debug_set_player(x, y, b'u');
                let g2 = goal2;
                let placed = t.hold_until(b2, 200, move |g| g.debug_objs().iter().any(|o| o.kind == "block" && (o.c, o.r) == g2));
                t.frames(10);
                t.check(moved && placed, &format!("dungeon {n}: a stone block is pushed onto a pressure plate"));
            }
            t.frames(4);
        }
        "IceBridge" => {
            let shrine = if east { tc(4, 9) } else { tc(11, 9) };
            t.g.debug_set_player(shrine.0, shrine.1 + 6.0, b'u');
            t.hold_until(Btn::Up, 20, |g| g.debug_element() == 1);
            t.check(t.g.debug_element() == 1, &format!("dungeon {n}: the ice shrine grants ice for the river"));
            let from = if east { tc(7, 6) } else { tc(8, 6) };
            t.fire_from(from.0, from.1, face);
            let cols = if east { [9, 10] } else { [5, 6] };
            t.check(cols.iter().all(|&c| t.g.debug_tile(c, 6) == T_ICE), &format!("dungeon {n}: an ice bolt freezes a path across the water"));
            t.shot(&format!("32_ice_bridge_{n}"));
            t.g.debug_set_player(from.0, from.1, face);
            t.hold_until(dir_btn, 200, move |g| {
                let o = g.debug_objs();
                o.iter().any(|o| (o.kind == "chest" || o.kind == "lever") && o.on)
            });
        }
        "Hidden" => {
            let (c, _) = if east { (11, 6) } else { (4, 6) };
            let from = if east { tc(8, 6) } else { tc(7, 6) };
            for _ in 0..4 {
                if t.g.debug_tile(c, 6) == T_FLOOR {
                    break;
                }
                t.fire_from(from.0, from.1, face);
            }
            t.check(t.g.debug_tile(c, 6) == T_FLOOR, &format!("dungeon {n}: bolts break the cracked wall open"));
            t.shot(&format!("33_hidden_passage_{n}"));
            t.g.debug_set_player(from.0, from.1, face);
            let got = t.hold_until(dir_btn, 200, move |g| {
                let o = g.debug_objs();
                o.iter().any(|o| (o.kind == "chest" || o.kind == "lever") && o.on)
            });
            if !got {
                let objs: Vec<String> = t.g.debug_objs().iter().map(|o| format!("{}@{},{} on={}", o.kind, o.c, o.r, o.on)).collect();
                println!("  diag: player {:?} objs {:?} tiles row6 {:?}", t.g.debug_player(), objs, (0..16).map(|c| t.g.debug_tile(c, 6)).collect::<Vec<_>>());
            }
        }
        _ => {}
    }
    // West chambers reveal / hold the key chest: go and open it.
    if !east {
        if let Some((c, r, on, visible)) = t.obj("chest") {
            t.check(visible, &format!("dungeon {n}: the key chest is available"));
            if !on {
                let (x, y) = tc(c, r);
                t.g.debug_set_player(x, y + 18.0, b'u');
                t.walk_to(x, y, 80);
                t.frames(2);
            }
        }
    }
}

fn boss_fight(t: &mut T, n: usize) {
    t.g.debug_god();
    let mut attacks: Vec<String> = vec![];
    let mut max_phase = 1;
    let mut saw_hazard = false;
    let mut saw_shots = false;
    let mut strafe_kept = true;
    let mut frame_ms: Vec<f64> = Vec::with_capacity(2400);
    t.g.debug_set_player(128.0, 200.0, b'u');
    t.input.set_key(Btn::Fire, true);
    let limit = 2400;
    for f in 0..limit {
        // Storm bolts: no status effects, so the fight isn't slowed by freezes.
        // Use a neutral element if the boss is weak to storm, so its whole moveset gets shown.
        let el = if t.g.debug_boss_weak() == Some(2) { 0 } else { 2 };
        t.g.debug_set_element(el);
        if let Some(b) = t.g.debug_boss() {
            if b.fighting && !attacks.contains(&b.atk) {
                attacks.push(b.atk.clone());
            }
            max_phase = max_phase.max(b.phase);
            saw_hazard |= b.hazards > 0;
            let (px, _) = t.g.debug_player();
            t.input.set_key(Btn::Left, b.x < px - 3.0);
            t.input.set_key(Btn::Right, b.x > px + 3.0);
        }
        saw_shots |= t.g.debug_enemy_bullets() > 0;
        let t0 = std::time::Instant::now();
        t.frames(1);
        t.g.draw(&mut t.scr);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        frame_ms.push(ms);
        if t.g.debug_dir() != b'u' && t.g.debug_mode() == Mode::Play {
            strafe_kept = false;
        }
        match f {
            150 => t.shot(&format!("34_boss_{n}_a")),
            400 => {
                t.shot(&format!("35_boss_{n}_b"));
                t.g.debug_boss_hp(0.45);
            }
            800 => {
                t.shot(&format!("36_boss_{n}_phase2"));
                if n == 6 {
                    t.g.debug_boss_hp(0.3);
                }
            }
            1300 => {
                t.shot(&format!("37_boss_{n}_c"));
                t.g.debug_boss_hp(0.02);
            }
            _ => {}
        }
        if matches!(t.g.debug_mode(), Mode::Reward | Mode::Victory) {
            break;
        }
        if t.g.debug_boss().map_or(false, |b| !b.alive) && f % 60 == 0 {
            t.shot(&format!("38_boss_{n}_defeat"));
        }
    }
    t.release();
    let name = t.g.debug_boss().map_or("?", |b| b.name);
    println!("  boss {name}: attacks {:?}, max phase {max_phase}", attacks);
    if !frame_ms.is_empty() {
        let avg = frame_ms.iter().sum::<f64>() / frame_ms.len() as f64;
        let mut sorted = frame_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p99 = sorted[(sorted.len() * 99 / 100).min(sorted.len() - 1)];
        println!("  perf: update+draw avg {avg:.2} ms, p99 {p99:.2} ms per frame on this machine");
        t.check(p99 < 4.0, &format!("boss {n}: frame cost stays low (p99 {p99:.2} ms, budget 16.7 ms)"));
    }
    t.check(attacks.len() >= 3, &format!("boss {n} ({name}) uses at least three distinct attacks"));
    t.check(max_phase >= 2, &format!("boss {n} ({name}) enters a second phase"));
    t.check(saw_hazard || saw_shots, &format!("boss {n} ({name}) creates hazards or projectiles"));
    t.check(strafe_kept, &format!("boss {n}: holding cast keeps the mage's facing while strafing"));
    t.hold_until(Btn::Mute, 400, |g| matches!(g.debug_mode(), Mode::Reward | Mode::Victory));
    t.input.release_all();
    let done = matches!(t.g.debug_mode(), Mode::Reward | Mode::Victory);
    t.check(done && t.g.debug_cleared(n), &format!("boss {n} ({name}) defeat animation leads to the reward"));
    t.shot(&format!("39_reward_{n}"));
    if t.g.debug_mode() == Mode::Reward {
        t.frames(70);
        t.tap(Btn::Start);
        t.frames(5);
    }
}

/// Dungeon 1's feast hall (west of the entrance) and the hidden pantries (east, behind cracks).
fn food_room(t: &mut T, n: usize) {
    const R_FEAST: usize = 5;
    const R_PANTRY: usize = 6;
    t.g.debug_kill_enemies();
    let (btn, room, back) = if n == 1 { (Btn::Left, R_FEAST, Btn::Right) } else { (Btn::Right, R_PANTRY, Btn::Left) };
    if n >= 2 {
        t.check(t.g.debug_tile(15, 6) == 4, &format!("dungeon {n}: a cracked wall hides the pantry doorway"));
        for _ in 0..4 {
            if t.g.debug_tile(15, 6) == T_FLOOR {
                break;
            }
            let (x, y) = tc(12, 6);
            t.fire_from(x, y, b'r');
        }
        t.check((5..=7).all(|r| t.g.debug_tile(15, r) == T_FLOOR), &format!("dungeon {n}: breaking the crack opens the whole pantry doorway"));
    }
    t.g.debug_set_player(128.0, 136.0, if n == 1 { b'l' } else { b'r' });
    let ok = t.go_room(btn, room);
    t.check(ok, &format!("dungeon {n}: reached the {}", if n == 1 { "feast hall" } else { "hidden pantry" }));
    t.frames(4);
    let stock = t.g.debug_larder_items();
    t.check(stock >= if n == 1 { 8 } else { 4 }, &format!("dungeon {n}: the food room is stocked ({stock} items)"));
    t.check(t.g.debug_enemy_count() == 0, &format!("dungeon {n}: the food room is a safe room"));
    t.check(t.g.debug_tile(7, 0) == 1 && t.g.debug_tile(7, 12) == 1, &format!("dungeon {n}: the food room only connects to the entrance hall"));
    t.shot(&format!("{}_{n}", if n == 1 { "40_feast_hall" } else { "41_pantry" }));
    // Eat one thing, leave, come back: the rest should still be there.
    t.g.debug_set_food(10.0);
    let (fx, fy) = if n == 1 { tc(4, 6) } else { tc(6, 5) };
    t.g.debug_set_player(fx + 16.0, fy, b'l');
    t.walk_to(fx, fy, 60);
    t.frames(2);
    t.check(t.g.debug_food() > 30.0, &format!("dungeon {n}: eating the food fills the food bar"));
    let left = t.g.debug_larder_items();
    // Leave along a row with no food on it.
    let (exit_x, exit_y) = if n == 1 { (200.0, 120.0) } else { (56.0, 136.0) };
    t.g.debug_set_player(exit_x, exit_y, b'u');
    let ok = t.go_room(back, R_ENTRY);
    t.check(ok, &format!("dungeon {n}: back to the entrance"));
    t.g.debug_set_player(128.0, 136.0, b'u');
    t.go_room(btn, room);
    t.frames(2);
    t.check(t.g.debug_larder_items() == left, &format!("dungeon {n}: uneaten food is still there when you return"));
    t.g.debug_set_player(exit_x, exit_y, b'u');
    t.go_room(back, R_ENTRY);
    t.g.debug_god();
    t.g.debug_set_player(128.0, 150.0, b'u');
}