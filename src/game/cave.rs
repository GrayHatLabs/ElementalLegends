//! Optional caves: small two- or three-room dungeons to clear for a treasure.
//!
//! ```text
//!   [TREASURE]          chest with the cave's reward (relic or power-up)
//!       | sealed until the challenge is beaten
//!   [CHALLENGE]         a sealed-door fight, or a freeze-a-monster-onto-the-plate puzzle
//!       |
//!   [MOUTH]             (three-room caves only) a few monsters; exit south to the overworld
//! ```
//! Caves are dungeon numbers LAIRS+1 ..= LAIRS+CAVES and reuse the dungeon rooms, objects,
//! progress flags (D_EAST = challenge beaten, D_KEY = treasure taken) and map.
//! Puzzle monsters that die are replaced, so the freeze puzzle can never soft-lock.
use super::dungeon::*;
use super::*;
use crate::levels::{Level, Loot};

/// Dungeon numbers 1..=LAIRS are the lairs; caves follow.
pub(super) const LAIRS: usize = 6;
pub(super) const C_SOLVED: u8 = D_EAST;
pub(super) const C_LOOTED: u8 = D_KEY;
/// Puzzle monsters are tough, so they get frozen rather than killed.
const PUZZLE_HP: f32 = 40.0;

/// What a cave's chest holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Reward {
    Bombs(i32),
    Elixirs(i32),
    Heart,
    ManaUp,
}

/// Per cave (1-based): three rooms?, challenge, freeze plates, reward.
fn cave_def(k: usize) -> (bool, Puz, usize, Reward) {
    match k {
        1 => (false, Puz::Combat, 0, Reward::Bombs(5)),
        2 => (true, Puz::FreezePlate, 1, Reward::Elixirs(1)),
        3 => (false, Puz::FreezePlate, 1, Reward::Heart),
        4 => (true, Puz::Combat, 0, Reward::ManaUp),
        5 => (true, Puz::FreezePlate, 2, Reward::Bombs(5)),
        6 => (false, Puz::Combat, 0, Reward::Heart),
        7 => (true, Puz::FreezePlate, 2, Reward::ManaUp),
        _ => (true, Puz::Combat, 0, Reward::Elixirs(2)),
    }
}
pub(super) fn is_cave(n: usize) -> bool {
    n > LAIRS
}
pub(super) fn cave_name(n: usize) -> &'static str {
    ["MOSSY HOLLOW", "WHISPERING GROTTO", "BONE CAVE", "ROOTED DEN", "FROSTED CAVERN", "SUNKEN BURROW", "ECHOING DEEP", "EMBER VAULT"]
        [(n - LAIRS - 1).min(CAVES - 1)]
}
/// Cave walls: the lair tileset that best matches each overworld region.
fn cave_theme(region: usize) -> usize {
    [4, 5, 4, 7][region.min(3)]
}
/// The room holding the challenge and the treasure room (dungeon room slots).
pub(super) fn cave_rooms(three: bool) -> (usize, usize) {
    if three {
        (R_HUB, R_STAIRS)
    } else {
        (R_ENTRY, R_HUB)
    }
}
/// Pressure plates in a freeze-puzzle room.
fn plate_spots(n: usize) -> &'static [(i32, i32)] {
    if n >= 2 {
        &[(4, 4), (11, 4)]
    } else {
        &[(8, 4)]
    }
}

pub(super) fn build_cave(n: usize, region: usize, themes: &[Theme], prog: u8, level: Option<&Level>) -> Dungeon {
    let (mut three, mut puz, plates, _) = cave_def(n - LAIRS);
    // A level file decides the room count (a "mouth" room makes three) and the challenge.
    let lrooms = level.map(|l| &l.rooms).filter(|r| !r.is_empty());
    let mut no_challenge = false;
    if let Some(rs) = lrooms {
        three = rs.contains_key("mouth");
        match rs.get("challenge").and_then(|c| c.puzzle.as_deref()) {
            Some("none") => no_challenge = true,
            Some(p) => puz = puz_from(p).unwrap_or(puz),
            None => {}
        }
    }
    let (chal, treasure) = cave_rooms(three);
    let theme = level.and_then(|l| l.theme).filter(|&t| t < themes.len()).unwrap_or(cave_theme(region));
    let slot_of = |i: usize| -> Option<&str> {
        if i == chal {
            Some("challenge")
        } else if i == treasure {
            Some("treasure")
        } else if i == R_ENTRY && three {
            Some("mouth")
        } else {
            None
        }
    };
    let def = |i: usize| slot_of(i).and_then(|sl| lrooms.and_then(|r| r.get(sl)));
    let solved = prog & C_SOLVED != 0 || no_challenge;
    let mut has = vec![false; 7];
    has[R_ENTRY] = true;
    has[R_HUB] = true;
    has[R_STAIRS] = three;
    let mut rooms = Vec::new();
    let mut objs = Vec::new();
    for i in 0..7 {
        let mut r = Room::new(i, GRID[i].0 as usize, GRID[i].1 as usize, 9100 + n as u32 * 10 + i as u32);
        for d in 0..4 {
            r.doors[d] = has[i] && neighbor(i, d, &has).is_some();
        }
        if i == R_ENTRY {
            r.doors[1] = true; // way back out
        }
        r.theme = region.min(3);
        r.visited = i == R_ENTRY;
        r.frame();
        let mut o: Vec<Obj> = Vec::new();
        if has[i] {
            if i == chal {
                // The way on to the treasure stays sealed until the challenge is beaten.
                if !solved {
                    r.set_gap(0, T_SEAL);
                }
                match puz {
                    Puz::FreezePlate => {
                        for &(c, y) in plate_spots(plates) {
                            r.tiles[y as usize][c as usize] = T_PLATE;
                        }
                        for &(x, y) in &[(2, 2), (13, 2), (2, 10), (13, 10)] {
                            r.tiles[y][x] = T_WALL;
                        }
                        o.push(Obj::new(OK::Shrine(Elem::Ice), 3, 8));
                    }
                    _ => {
                        for &(x, y) in &[(5, 4), (10, 4), (5, 8), (10, 8)] {
                            r.tiles[y][x] = T_WALL;
                        }
                    }
                }
            } else if i == treasure {
                for &(x, y) in &[(4, 4), (11, 4), (4, 8), (11, 8)] {
                    r.tiles[y][x] = T_WALL;
                }
                let mut ch = Obj::new(OK::Chest, 8, 5);
                ch.on = prog & C_LOOTED != 0;
                o.push(ch);
            } else {
                // The mouth of a three-room cave: loose rocks.
                for &(x, y) in &[(3, 3), (12, 4), (4, 9), (11, 9), (2, 6)] {
                    r.tiles[y][x] = T_WALL;
                }
            }
            if let Some(d) = def(i) {
                apply_room_def(&mut r, &mut o, d, false, false, prog & C_LOOTED != 0);
                if i == chal && !solved {
                    r.set_gap(0, T_SEAL);
                }
            }
        }
        render(&mut r, &themes[theme]);
        rooms.push(r);
        objs.push(o);
    }
    let mut pz = vec![None; 7];
    if !no_challenge {
        pz[chal] = Some(puz);
    }
    Dungeon {
        n, rooms, objs, puz: pz, has, larder: vec![vec![]; 7], hub_combat: false, cur: R_ENTRY, dirty: false, sealed: false,
        push_t: 0, crack_hits: vec![], seen: vec![false; 7], warned: false, cave: true, theme, keep: None,
        spawns: (0..7).map(|i| def(i).map_or(vec![], |d| d.enemies.clone())).collect(),
        random: (0..7).map(|i| def(i).and_then(|d| d.random_enemies.or(if d.enemies.is_empty() { None } else { Some(0) }))).collect(),
        chal,
        treasure,
    }
}

impl Game {
    fn cave_k(&self) -> usize {
        self.dungeon.as_ref().map_or(1, |d| d.n - LAIRS)
    }
    pub(super) fn in_cave(&self) -> bool {
        self.dungeon.as_ref().map_or(false, |d| d.cave)
    }
    /// Cave k (1-based) has had its treasure taken.
    pub(super) fn cave_cleared(&self, k: usize) -> bool {
        self.s.dprog.get(LAIRS + k).map_or(false, |p| p & C_LOOTED != 0)
    }

    /// Room entry inside a cave: spawns, seals and hints.
    pub(super) fn enter_cave_room(&mut self) {
        let prog = self.dprog();
        let k = self.cave_k();
        let Some(d) = self.dungeon.as_mut() else { return };
        let (chal, treasure) = (d.chal, d.treasure);
        let puz = d.puz[chal];
        let solved = prog & C_SOLVED != 0 || puz.is_none();
        let cur = d.cur;
        let first = !d.seen[cur];
        d.seen[cur] = true;
        d.rooms[cur].visited = true;
        d.warned = false;
        let th = d.rooms[cur].theme;
        let fight = cur == chal && puz == Some(Puz::Combat) && !solved;
        if fight {
            for dd in 0..4 {
                if d.rooms[cur].doors[dd] {
                    d.rooms[cur].set_gap(dd, T_SEAL);
                }
            }
            d.sealed = true;
            d.dirty = true;
        }
        self.dungeon_flush();
        let listed = self.spawn_listed(cur, fight);
        if fight {
            if !listed {
                self.spawn_pack(4 + k as i32 / 2, th, false);
            }
            self.show_msg("THE CAVE RUMBLES SHUT BEHIND YOU! DEFEAT EVERY MONSTER.");
            self.sfx(Sfx::Rumble);
            self.shake = 10;
        } else if cur == chal && puz == Some(Puz::FreezePlate) && !solved {
            if !listed {
                self.spawn_puzzle_foes();
            }
            if first {
                self.show_msg("A HEAVY PLATE... TOO HEAVY FOR YOU ALONE. FREEZE A MONSTER AND PUSH IT ON!");
            }
        } else if cur == treasure {
            if first && prog & C_LOOTED == 0 {
                self.show_msg("A TREASURE CHEST GLITTERS IN THE DARK.");
            }
        } else {
            if !listed {
                self.spawn_pack(if cur == chal { 1 } else { 2 }, th, false);
            }
            if first && cur == R_ENTRY {
                self.show_msg(format!("{}. CLEAR THE CAVE AND CLAIM ITS TREASURE.", self.dname(k + LAIRS)));
            }
        }
    }
    /// Two slow, tough monsters to freeze and shove onto the plates.
    fn spawn_puzzle_foes(&mut self) {
        let th = self.dungeon.as_ref().map_or(0, |d| d.rooms[d.cur].theme);
        let have = self.enemies.iter().filter(|e| !e.dead).count();
        for (i, &(x, y)) in [(88.0, 150.0), (168.0, 150.0)].iter().enumerate().skip(have) {
            let kind = if i == 0 { EK::Slime } else { EK::Golem };
            let mut e = self.make_enemy(kind, x, y, th);
            e.hp = PUZZLE_HP;
            e.spd *= 0.6;
            self.enemies.push(e);
        }
    }

    /// Player interactions in a cave: shrines, the chest, and pushing frozen monsters.
    pub(super) fn cave_player(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        self.push_frozen(ix, iy, blocked);
        let (px, py) = (self.pl.x, self.pl.y);
        let touched: Vec<(usize, OK, bool)> = self.dungeon.as_ref().map_or(vec![], |d| {
            d.objs[d.cur]
                .iter()
                .enumerate()
                .filter(|(_, o)| o.visible)
                .filter(|(_, o)| {
                    let (ox, oy) = tile_center(o.c, o.r);
                    (ox - px).abs() < 12.0 && (oy - py).abs() < 12.0
                })
                .map(|(i, o)| (i, o.k, o.on))
                .collect()
        });
        for (i, k, on) in touched {
            match k {
                OK::Chest if !on => self.open_cave_chest(i),
                OK::Shrine(el) if self.el() != el => self.set_element(el),
                _ => {}
            }
        }
    }
    /// Walking into a frozen monster shoves it one tile, like a push block.
    fn push_frozen(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        let axis = if ix != 0.0 && iy == 0.0 { Some((ix.signum(), 0.0)) } else if iy != 0.0 && ix == 0.0 { Some((0.0, iy.signum())) } else { None };
        let _ = blocked;
        let Some((dx, dy)) = axis else {
            if let Some(d) = self.dungeon.as_mut() {
                d.push_t = 0;
            }
            return;
        };
        let (px, py) = (self.pl.x + dx * 12.0, self.pl.y + dy * 12.0);
        let Some(i) = self.enemies.iter().position(|e| {
            !e.dead && e.st.frozen() && (e.x - px).abs() < e.w / 2.0 + 4.0 && (e.y - py).abs() < e.h / 2.0 + 4.0 && e.kbx.abs() + e.kby.abs() < 0.1
        }) else {
            return;
        };
        let Some(d) = self.dungeon.as_mut() else { return };
        d.push_t += 1;
        if d.push_t < 12 {
            return;
        }
        d.push_t = 0;
        // Knockback carries it about one tile (4 / (1 - 0.75) = 16 units).
        let e = &mut self.enemies[i];
        e.kbx = dx * 4.0;
        e.kby = dy * 4.0;
        self.sfx(Sfx::Push);
    }
    fn open_cave_chest(&mut self, i: usize) {
        let (x, y) = {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
            tile_center(d.objs[cur][i].c, d.objs[cur][i].r)
        };
        let k = self.cave_k();
        let (_, _, _, reward) = cave_def(k);
        let loot = self.dungeon.as_ref().map_or(Loot::Default, |d| d.objs[d.cur][i].loot);
        self.set_dprog(C_LOOTED);
        let custom = !matches!(loot, Loot::Default | Loot::Key | Loot::Page);
        let msg = if custom { self.give_loot(loot) } else { match reward {
            Reward::Bombs(n) => {
                self.s.bombs = (self.s.bombs + n).min(bag::MAX_BOMBS);
                format!("{} BOMBS FOR YOUR BAG! SELECT PICKS THEM, Y DROPS ONE.", n)
            }
            Reward::Elixirs(n) => {
                self.s.elixirs = (self.s.elixirs + n).min(bag::MAX_ELIXIRS);
                if n == 1 { "AN ELIXIR! SAVE IT FOR A HARD FIGHT.".to_string() } else { format!("{} ELIXIRS! SAVE THEM FOR HARD FIGHTS.", n) }
            }
            Reward::Heart => {
                self.s.max_hp += 4;
                self.s.hp = self.s.max_hp;
                "A HEART CONTAINER! MAXIMUM LIFE UP.".to_string()
            }
            Reward::ManaUp => {
                self.s.max_mp += 10;
                self.s.mp = self.s.max_mp as f32;
                "A MANA CRYSTAL! MAXIMUM MAGIC UP.".to_string()
            }
        } };
        let page = match self.take_page(k) {
            Some(n) => format!(" AND A LOST SPELLBOOK PAGE, {} OF 5!", n),
            None => String::new(),
        };
        self.float("TREASURE!", x - 36.0, y - 22.0, WHITE);
        self.show_msg(format!("{}{} THE CAVE IS CLEARED.", msg, page));
        self.part(x, y, 0.0, 0.0, 16, rgb(0xfcbc3c), 1, PK::Glow(14.0));
        self.sfx(Sfx::Fanfare);
        self.save();
    }

    /// Per-frame cave logic: fights end, plates press, puzzle monsters come back.
    pub(super) fn cave_update(&mut self) {
        let prog = self.dprog();
        let Some(d) = self.dungeon.as_mut() else { return };
        let (chal, puz) = (d.chal, d.puz[d.chal]);
        let cur = d.cur;
        if d.sealed && self.enemies.iter().all(|e| e.dead) {
            d.sealed = false;
            for dd in 0..4 {
                if d.rooms[cur].doors[dd] {
                    d.rooms[cur].set_gap(dd, T_FLOOR);
                }
            }
            d.dirty = true;
            self.cave_solved("SILENCE FALLS. THE WAY DEEPER GRINDS OPEN.");
            return;
        }
        if cur != chal || puz != Some(Puz::FreezePlate) || prog & C_SOLVED != 0 {
            return;
        }
        let mut spots: Vec<(i32, i32)> = vec![];
        for (y, row) in d.rooms[cur].tiles.iter().enumerate() {
            for (x, &t) in row.iter().enumerate() {
                if t == T_PLATE {
                    spots.push((x as i32, y as i32));
                }
            }
        }
        let pressed = spots
            .iter()
            .filter(|&&(c, r)| self.enemies.iter().any(|e| !e.dead && e.st.frozen() && tile_of(e.x, e.y) == (c, r)))
            .count();
        if pressed == spots.len() {
            self.cave_solved("THE PLATE SINKS WITH A HEAVY CLUNK. THE WAY DEEPER GRINDS OPEN.");
            return;
        }
        // Never leave the room without something to freeze.
        if self.frame % 120 == 0 && self.enemies.iter().filter(|e| !e.dead).count() < 2 {
            let custom = self.dungeon.as_ref().map_or(false, |d| !d.spawns[cur].is_empty());
            if custom {
                if self.enemies.iter().all(|e| e.dead) {
                    self.spawn_listed(cur, false);
                }
            } else {
                self.spawn_puzzle_foes();
            }
            self.float("SOMETHING CRAWLS OUT OF THE ROCKS...", 20.0, HUDF + 20.0, rgb(0xbcbcbc));
        }
    }
    fn cave_solved(&mut self, msg: &str) {
        self.set_dprog(C_SOLVED);
        if let Some(d) = self.dungeon.as_mut() {
            let cur = d.cur;
            d.rooms[cur].set_gap(0, T_FLOOR);
            d.dirty = true;
        }
        self.dungeon_flush();
        self.show_msg(msg);
        self.sfx(Sfx::Unlock);
        self.shake = 10;
    }
}
