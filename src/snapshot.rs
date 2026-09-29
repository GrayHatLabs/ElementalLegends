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
    t.g.debug_set_player(56.0, 150.0, b'u');
    t.walk_to(56.0, 136.0, 60);
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
    t.g.debug_set_player(104.0, 150.0, b'u');
    t.walk_to(104.0, 136.0, 60);
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
    t.g.debug_set_player(104.0, 150.0, b'u');
    t.walk_to(104.0, 136.0, 60);
    t.frames(2);
    t.tap(Btn::Fire);
    t.check(t.g.debug_gold() == 100, "the shop won't sell a potion when the pack is full");
    t.shot("09b_potion_hud");
    // Antidotes: bought and carried, cure poison with the potion button.
    t.g.debug_set_player(152.0, 150.0, b'u');
    t.walk_to(152.0, 136.0, 60);
    t.frames(2);
    t.tap(Btn::Fire);
    t.check(t.g.debug_antidotes() == 1 && t.g.debug_gold() == 80, "the shop sells an antidote (20 gold) that goes in the pack");
    t.g.debug_set_player(128.0, 176.0, b'u');
    let hp_before = t.g.debug_hp();
    t.g.debug_set_poison(400);
    t.frames(80);
    t.check(t.g.debug_hp() < hp_before, "poison drains health over time");
    t.shot("09c_poisoned");
    t.tap(Btn::Potion);
    t.check(t.g.debug_poison() == 0 && t.g.debug_antidotes() == 0, "with poison and an antidote, the potion button cures it");
    t.g.debug_set_poison(100);
    t.frames(120);
    t.check(t.g.debug_poison() == 0, "poison wears off on its own");

    // ---------------------------------------------------------------- fire: travelling bolt + burning DOT
    println!("[fire] firebolt and burning");
    t.g.debug_god();
    t.g.debug_set_element(0);
    t.g.debug_play_room(start, 40.0, 200.0);
    t.frames(5);
    let id = t.g.debug_spawn(2, 105.0, 200.0, 60.0, true); // skeleton: neutral, holds still
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
    t.check(xs.len() >= 10 && travelled > 40.0, "firebolt visibly travels before hitting");
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
    let id = t.g.debug_spawn(2, 120.0, 200.0, 80.0, true);
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
    t.check(short < 95.0, "without power-ups a bolt only flies about five tiles");
    t.check(weak < short * 0.7, "bolts get shorter as the mage's health drops");
    t.check(full > 160.0 && full < 210.0, "the top magic power-up extends bolts to about twelve tiles");
    let mut shots = vec![];
    for lv in 1..=3 {
        t.g.debug_set_spell_lv(lv);
        t.g.debug_set_player(20.0, 150.0, b'r');
        t.frames(40);
        t.tap(Btn::Fire);
        shots.push(t.g.debug_player_bolts().len());
        t.frames(60);
    }
    t.check(shots == vec![1, 1, 2], &format!("single bolt until the end stages, twin bolts at magic level 3 (got {shots:?})"));
    t.g.debug_set_spell_lv(1);
    t.g.debug_god();

    // ---------------------------------------------------------------- overworld encounters
    encounters(&mut t);

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
    t.g.debug_set_player(128.0, 176.0, b'u');
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

/// The four overworld encounters: Hoard Dragon, Deceiving Dryad, fruit trees + treant, graveyard.
fn encounters(t: &mut T) {
    let mob_hp = |t: &T, id: u32| t.g.debug_mobs().into_iter().find(|m| m.id == id).map_or(0.0, |m| m.hp);

    // ---------------------------------------------------------------- hoard dragon
    println!("[encounter] hoard dragon");
    t.release();
    t.g.debug_god();
    t.g.debug_set_element(2);
    let Some(room) = t.g.debug_mini_room(1) else {
        t.check(false, "a hoard dragon screen exists");
        return;
    };
    t.g.debug_play_room(room, 128.0, 200.0);
    t.frames(5);
    let coins = t.g.debug_items().iter().filter(|i| i.3).count();
    let asleep = t.g.debug_mob("HoardDragon").map_or(false, |d| d.mode == 0);
    t.check(asleep && coins == 24, &format!("the hoard dragon sleeps on its gold ({coins} coins)"));
    t.check(t.g.debug_mini_seen(1), "the hoard screen is marked as discovered");
    t.shot("50_hoard_asleep");
    t.g.debug_set_player(128.0, 170.0, b'u');
    t.tap(Btn::Fire);
    t.frames(40);
    let woke = t.g.debug_mob("HoardDragon").map_or(false, |d| d.mode != 0 && d.hp >= 9999.0);
    t.check(woke, "shooting the dragon wakes it, but it can't be hurt");
    let anger0 = t.g.debug_hoard_anger();
    let mut shots = false;
    for _ in 0..240 {
        t.frames(1);
        shots |= t.g.debug_enemy_bullets() > 0;
    }
    t.check(shots, "the awake dragon fights back with swipes and fire breath");
    t.shot("51_hoard_awake");
    let gold0 = t.g.debug_gold();
    let pile: Vec<(f32, f32)> = t.g.debug_items().iter().filter(|i| i.3).map(|i| (i.1, i.2)).collect();
    for (k, &(x, y)) in pile.iter().enumerate() {
        t.g.debug_set_player(x, y, b'u');
        t.frames(2);
        if k == pile.len() / 2 {
            t.check(t.g.debug_hoard_anger() > anger0, "taking its gold makes the dragon angrier");
            t.shot("52_hoard_looting");
        }
    }
    t.check(t.g.debug_hoard_left() == 0 && t.g.debug_gold() > gold0 + 100, "the hoard's gold is the reward");
    t.check(t.g.debug_mini_done(1), "with the gold gone the encounter is finished (saved)");
    t.frames(40);
    t.shot("53_hoard_flies_off");
    t.frames(120);
    t.check(t.g.debug_mob("HoardDragon").is_none(), "the dragon flies away for good");
    t.g.debug_play_room(room, 128.0, 200.0);
    t.frames(5);
    t.check(t.g.debug_mob("HoardDragon").is_none() && t.g.debug_items().iter().all(|i| !i.3), "revisiting: no dragon and no gold left");

    // ---------------------------------------------------------------- deceiving dryad
    println!("[encounter] deceiving dryad");
    let Some(a) = t.g.debug_mini_room(2) else {
        t.check(false, "a dryad screen exists");
        return;
    };
    let (b, dir) = t.g.debug_pool_room().unwrap();
    t.g.debug_play_room(a, 128.0, 200.0);
    t.frames(5);
    let d = t.g.debug_mob("Dryad");
    t.check(d.as_ref().map_or(false, |d| d.mode == 0), "a friendly 'villager' waits on the dryad's screen");
    t.shot("54_dryad_disguised");
    // Attacking her early reveals her (fresh world afterwards so the lure can be tested too).
    if let Some(d) = d {
        t.g.debug_set_player(d.x, d.y + 30.0, b'u');
        t.tap(Btn::Fire);
        t.frames(30);
    }
    t.check(t.g.debug_mob("Dryad").map_or(false, |d| d.mode == 1), "attacking the villager early reveals the dryad");
    t.g.debug_new_world(2);
    t.g.debug_god();
    t.g.debug_play_room(a, 128.0, 200.0);
    t.frames(5);
    let mut wilted = 0;
    for _ in 0..700 {
        let Some(d) = t.g.debug_mob("Dryad") else { break };
        let (px, py) = t.g.debug_player();
        let (dx, dy) = (d.x - px, d.y - py);
        let far = dx.hypot(dy) > 30.0;
        t.input.set_key(Btn::Left, far && dx < -2.0);
        t.input.set_key(Btn::Right, far && dx > 2.0);
        t.input.set_key(Btn::Up, far && dy < -2.0);
        t.input.set_key(Btn::Down, far && dy > 2.0);
        t.frames(1);
        wilted = wilted.max(t.g.debug_wilt_marks());
    }
    t.release();
    t.check(wilted > 0, "flowers wilt where the 'villager' walks (a tell)");
    t.check(t.g.debug_dryad_stage() == 1, "she leads the player off toward the next screen");
    let entry = match dir {
        0 => (128.0, 222.0),
        1 => (128.0, 44.0),
        2 => (18.0, 136.0),
        _ => (238.0, 136.0),
    };
    t.g.debug_play_room(b, entry.0, entry.1);
    t.frames(5);
    t.check(t.g.debug_mob("Dryad").map_or(false, |d| d.mode == 0), "she waits beyond her pool on the next screen");
    t.shot("55_dryad_beckons");
    t.walk_to(128.0, 124.0, 300);
    t.frames(3);
    t.check(t.g.debug_poison() > 0, "stepping into the pool poisons the player");
    t.check(t.g.debug_mob("Dryad").map_or(false, |d| d.mode == 1), "then the dryad reveals herself and attacks");
    t.shot("56_dryad_revealed");
    let mut fought = false;
    for _ in 0..200 {
        t.frames(1);
        fought |= t.g.debug_enemy_bullets() > 0 || t.g.debug_hazards() > 0;
    }
    t.check(fought, "the revealed dryad throws thorns and lashes with vines");
    if let Some(d) = t.g.debug_mob("Dryad") {
        let h0 = d.hp;
        t.g.debug_hit(d.id, 3.0, 0);
        t.check((h0 - mob_hp(t, d.id) - 6.0).abs() < 0.01, "the dryad is weak to fire (double damage)");
        let mh0 = t.g.debug_max().0;
        t.g.debug_hit(d.id, 999.0, 0);
        t.frames(3);
        t.check(t.g.debug_mini_done(2) && t.g.debug_max().0 == mh0 + 4, "defeating the dryad gives a heart container");
    }

    // ---------------------------------------------------------------- fruit trees + angry treant
    println!("[encounter] fruit trees and the angry treant");
    t.g.debug_god();
    t.g.debug_set_element(2);
    let treant_room = t.g.debug_mini_room(3).unwrap();
    let plain = t.g.debug_fruit_rooms().into_iter().find(|&r| r != treant_room).unwrap();
    t.g.debug_play_room(plain, 128.0, 200.0);
    t.g.debug_kill_enemies();
    t.frames(40);
    let apples = |t: &T| t.g.debug_items().iter().filter(|i| i.0 == "Apple").count();
    let trees = t.g.debug_trees();
    t.check(!trees.is_empty(), "fruit trees grow on some screens");
    let (c, r, _, _, _) = trees[0];
    let (x, y) = tc(c, r);
    let a0 = apples(t);
    t.fire_from(x, y + 30.0, b'u');
    t.check(apples(t) == a0 + 1, "shooting a fruit tree knocks down an apple");
    t.g.debug_set_player(x + 18.0, y, b'l');
    t.hold_until(Btn::Left, 34, |_| false);
    let (_, _, left, regrow, _) = t.g.debug_trees()[0];
    t.check(left == 0 && regrow > 0 && apples(t) >= a0 + 3, "bumping the tree shakes loose more apples until it's bare");
    let bare = apples(t);
    t.fire_from(x, y + 30.0, b'u');
    t.check(apples(t) == bare, "a bare tree drops nothing until it regrows");
    t.g.debug_regrow_now();
    t.frames(2);
    t.check(t.g.debug_trees()[0].2 == 3, "the tree restocks after a few minutes");
    t.shot("57_fruit_trees");
    t.g.debug_play_room(treant_room, 128.0, 200.0);
    t.g.debug_kill_enemies();
    t.frames(40);
    t.check(t.g.debug_mini_seen(3), "the treant's screen is marked as discovered");
    if let Some(tr) = t.g.debug_trees().into_iter().find(|t| t.4) {
        let (x, y) = tc(tr.0, tr.1);
        // Shoot from whichever side has open ground.
        if tr.0 < 8 {
            t.fire_from(x + 28.0, y, b'l');
        } else {
            t.fire_from(x - 28.0, y, b'r');
        }
        t.frames(40);
        t.check(t.g.debug_mob("Treant").is_some() && t.g.debug_tile(tr.0, tr.1) == T_FLOOR, "one 'fruit tree' is a disguised treant that wakes when shaken");
        let mut fought = false;
        for _ in 0..300 {
            t.frames(1);
            fought |= t.g.debug_hazards() > 0 || t.g.debug_enemy_bullets() > 0;
        }
        t.check(fought, "the angry treant attacks with roots, thrown apples and slams");
        t.shot("58_angry_treant");
        if let Some(m) = t.g.debug_mob("Treant") {
            let h0 = m.hp;
            t.g.debug_hit(m.id, 2.0, 2);
            t.check((h0 - mob_hp(t, m.id) - 4.0).abs() < 0.01, "the treant is weak to storm");
            if let Some(m2) = t.g.debug_mob("Treant") {
                let (px, py) = if m2.x < 128.0 { (m2.x + 80.0, m2.y) } else { (m2.x - 80.0, m2.y) };
                t.g.debug_set_player(px, py, b'd');
            }
            t.g.debug_hit(m.id, 999.0, 2);
            t.frames(3);
        }
        let ga = t.g.debug_items().into_iter().find(|i| i.0 == "GoldenApple");
        t.check(ga.is_some() && t.g.debug_mini_done(3), "the treant leaves a golden apple");
        if let Some(g) = ga {
            t.g.debug_set_food(10.0);
            t.g.debug_set_hp(5, 40);
            t.g.debug_set_player(g.1, g.2, b'u');
            t.frames(2);
            t.check(t.g.debug_food() >= 99.9 && t.g.debug_hp() == 40, "the golden apple restores all food and health");
        }
    } else {
        t.check(false, "the treant screen has a disguised tree");
    }

    // ---------------------------------------------------------------- graveyard
    println!("[encounter] graveyard");
    t.g.debug_god();
    let gr = t.g.debug_mini_room(4).unwrap();
    t.g.debug_play_room(gr, 128.0, 200.0);
    t.frames(3);
    let graves = t.g.debug_graves();
    t.check(graves.len() >= 6, &format!("tombstones stand in the graveyard ({})", graves.len()));
    t.shot("59_graveyard");
    let below = |g: (i32, i32)| {
        let (x, y) = tc(g.0, g.1);
        (x, y + 26.0)
    };
    t.g.debug_set_element(0);
    let (sx, sy) = below(graves[4]);
    t.fire_from(sx, sy, b'u');
    t.check(t.g.debug_zombies() == 0 && t.g.debug_mob("Zombie").is_none(), "fire doesn't disturb the dead");
    t.g.debug_set_element(2);
    t.fire_from(sx, sy, b'u');
    t.frames(45);
    t.check(t.g.debug_zombies() == 1 && t.g.debug_mob("Zombie").is_some(), "a storm bolt raises a zombie from its grave");
    t.check(t.g.debug_items().iter().filter(|i| i.0 == "Coin").count() >= 2, "opened graves give up gold");
    if let Some(z) = t.g.debug_mob("Zombie") {
        let h0 = z.hp;
        t.g.debug_hit(z.id, 2.0, 0);
        t.check((h0 - mob_hp(t, z.id) - 4.0).abs() < 0.01, "zombies are weak to fire");
    }
    t.g.debug_kill_enemies();
    t.g.debug_set_player(128.0, 136.0, b'u');
    t.frames(2);
    t.tap(Btn::Sub);
    t.frames(45);
    t.check(t.g.debug_zombies() >= 3, &format!("Chain Bolt raises several zombies at once ({} raised)", t.g.debug_zombies()));
    for &g in graves.iter() {
        if t.g.debug_mob("GraveLord").is_some() {
            break;
        }
        t.g.debug_kill_enemies();
        let (x, y) = below(g);
        t.fire_from(x, y, b'u');
    }
    t.frames(70);
    t.check(t.g.debug_mob("GraveLord").is_some(), "after six zombies the Grave Lord rises");
    t.shot("60_grave_lord");
    let mut fought = false;
    for _ in 0..300 {
        t.frames(1);
        fought |= t.g.debug_enemy_bullets() > 0 || t.g.debug_mobs().iter().filter(|m| m.kind == "Zombie").count() > 0;
    }
    t.check(fought, "the Grave Lord summons the dead and hurls dark orbs");
    if let Some(l) = t.g.debug_mob("GraveLord") {
        let h0 = l.hp;
        t.g.debug_hit(l.id, 2.0, 0);
        t.check((h0 - mob_hp(t, l.id) - 4.0).abs() < 0.01, "the Grave Lord is weak to fire");
        let mp0 = t.g.debug_max().1;
        t.g.debug_hit(l.id, 999.0, 0);
        t.frames(3);
        t.check(t.g.debug_mini_done(4) && t.g.debug_max().1 == mp0 + 20, "defeating the Grave Lord gives gold and his amulet (max MP up)");
    }
    t.g.debug_play_room(gr, 128.0, 200.0);
    t.frames(3);
    t.fire_from(sx, sy, b'u');
    t.frames(45);
    t.check(t.g.debug_mob("Zombie").is_some() && t.g.debug_mob("GraveLord").is_none(), "afterwards, lightning only raises ordinary zombies");

    // ---------------------------------------------------------------- map markers
    // (The dryad test reset the world, so drop by the hoard screen again first.)
    t.g.debug_play_room(room, 128.0, 200.0);
    t.frames(3);
    t.g.debug_kill_enemies();
    t.frames(2);
    t.tap(Btn::Start);
    t.frames(2);
    let all_seen = (1..=4).all(|i| t.g.debug_mini_seen(i));
    t.check(t.g.debug_paused() && all_seen, "every encounter shows on the map once discovered");
    t.shot("61_map_markers");
    t.tap(Btn::Start);
    t.release();
}
