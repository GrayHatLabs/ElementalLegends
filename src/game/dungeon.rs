//! Dungeons: a small cross of rooms behind each lair building.
//!
//! ```text
//!            [STAIRS]      boss staircase, sealed until the EAST puzzle is solved
//!               | (locked door: needs the key)
//!   [WEST] -- [HUB] -- [EAST]
//!               |
//!           [ENTRANCE]  -- exit south to the overworld
//! ```
//! WEST's puzzle yields the key, EAST's breaks the staircase seal. Every puzzle
//! kind is reusable, and element puzzles ship with an orb shrine of the needed
//! element so any mage can solve them. Push-block rooms reset when re-entered
//! unsolved, so nothing can soft-lock.
use super::*;

pub(super) const D_KEY: u8 = 1;
pub(super) const D_DOOR: u8 = 2;
pub(super) const D_WEST: u8 = 4;
pub(super) const D_EAST: u8 = 8;
pub(super) const D_HUB: u8 = 16;
pub(super) const D_CRACK_W: u8 = 32;
pub(super) const D_CRACK_E: u8 = 64;

pub(super) const R_ENTRY: usize = 0;
pub(super) const R_HUB: usize = 1;
pub(super) const R_WEST: usize = 2;
pub(super) const R_EAST: usize = 3;
pub(super) const R_STAIRS: usize = 4;
/// Feast hall (dungeon 1 only), west of the entrance.
pub(super) const R_FEAST: usize = 5;
/// Hidden pantry (dungeons 2-6), behind a cracked wall east of the entrance.
pub(super) const R_PANTRY: usize = 6;
pub(super) const D_PANTRY: u8 = 128;
const GRID: [(i32, i32); 7] = [(1, 2), (1, 1), (0, 1), (2, 1), (1, 0), (0, 2), (2, 2)];
/// Food laid out in each food room: (kind, col, row).
fn larder_stock(room: usize) -> Vec<(IK, i32, i32)> {
    match room {
        R_FEAST => vec![
            (IK::Meat, 4, 6),
            (IK::Bread, 6, 6),
            (IK::Meat, 8, 6),
            (IK::Bread, 10, 6),
            (IK::Meat, 12, 6),
            (IK::Apple, 4, 10),
            (IK::Apple, 7, 10),
            (IK::Potion, 9, 10),
            (IK::Apple, 12, 10),
        ],
        R_PANTRY => vec![(IK::Meat, 6, 5), (IK::Bread, 9, 5), (IK::Bread, 6, 8), (IK::Apple, 9, 8), (IK::Apple, 8, 7)],
        _ => vec![],
    }
}
/// Placed food never expires or blinks.
const LARDER_LIFE: i32 = i32::MAX / 2;
/// Staircase tiles in the stairs room (top-left corner; 2x2).
pub(super) const STAIRS_C: i32 = 7;
pub(super) const STAIRS_R: i32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Puz {
    /// Defeat every monster while the doors are sealed.
    Combat,
    /// Light four braziers with fire magic.
    Torches,
    /// Push stone blocks onto pressure plates.
    Plates,
    /// Freeze a river with ice magic to cross it.
    IceBridge,
    /// Break a cracked wall to find a hidden alcove.
    Hidden,
}

/// (hub is a combat room, west puzzle, east puzzle)
pub(super) fn layout(n: usize) -> (bool, Puz, Puz) {
    match n {
        1 => (false, Puz::Combat, Puz::Torches),
        2 => (false, Puz::Plates, Puz::Hidden),
        3 => (true, Puz::IceBridge, Puz::Plates),
        4 => (true, Puz::Hidden, Puz::Torches),
        5 => (false, Puz::Torches, Puz::IceBridge),
        _ => (true, Puz::IceBridge, Puz::Hidden),
    }
}
pub(super) fn dungeon_name(n: usize) -> &'static str {
    ["", "OVERGROWN SHRINE", "UNDERGROUND CRYPT", "RUINED CASTLE", "DRAGON FORTRESS", "FORGOTTEN SANCTUARY", "DARK TOWER"]
        [n.min(6)]
}
pub(super) fn dungeon_theme(n: usize) -> usize {
    3 + n.clamp(1, 6)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum OK {
    Torch,
    Block,
    Lever,
    Chest,
    Shrine(Elem),
}

#[derive(Clone, Debug)]
pub(super) struct Obj {
    pub k: OK,
    pub c: i32,
    pub r: i32,
    /// Torch lit / lever pulled / chest opened / block locked on its plate.
    pub on: bool,
    pub visible: bool,
    /// Push-slide animation frames remaining and direction.
    pub slide: i32,
    pub sdx: i32,
    pub sdy: i32,
    pub home: (i32, i32),
}
impl Obj {
    fn new(k: OK, c: i32, r: i32) -> Self {
        Obj { k, c, r, on: false, visible: true, slide: 0, sdx: 0, sdy: 0, home: (c, r) }
    }
    pub fn solid(&self) -> bool {
        self.visible && matches!(self.k, OK::Torch | OK::Block | OK::Lever)
    }
    /// Draw position (includes the push slide).
    pub fn pos(&self) -> (f32, f32) {
        let (x, y) = tile_center(self.c, self.r);
        (x - (self.sdx * self.slide * 2) as f32, y - (self.sdy * self.slide * 2) as f32)
    }
}

pub(super) struct Dungeon {
    pub n: usize,
    pub rooms: Vec<Room>,
    pub objs: Vec<Vec<Obj>>,
    pub puz: [Option<Puz>; 7],
    /// Which of the seven room slots this dungeon uses.
    pub has: [bool; 7],
    /// Food left in the feast hall / pantry, kept while you come and go.
    pub larder: Vec<Vec<(IK, f32, f32)>>,
    pub hub_combat: bool,
    pub cur: usize,
    pub dirty: bool,
    pub sealed: bool,
    pub push_t: i32,
    /// (room, col, row, hits) for cracked walls being chipped at.
    pub crack_hits: Vec<(usize, i32, i32, i32)>,
    pub seen: [bool; 7],
    pub warned: bool,
}

fn neighbor(i: usize, d: usize, has: &[bool; 7]) -> Option<usize> {
    let (x, y) = (GRID[i].0 + DIRS[d].0, GRID[i].1 + DIRS[d].1);
    let food = |r: usize| r == R_FEAST || r == R_PANTRY;
    GRID.iter()
        .enumerate()
        .position(|(j, &g)| has[j] && g == (x, y))
        // Food rooms open only onto the entrance hall, never into puzzle chambers.
        .filter(|&j| !(food(i) || food(j)) || i == R_ENTRY || j == R_ENTRY)
}
/// Mirror a column for the east chamber (templates are written for the west one).
fn mc(c: i32, east: bool) -> i32 {
    if east {
        RC as i32 - 1 - c
    } else {
        c
    }
}

pub(super) fn build_dungeon(n: usize, themes: &[Theme], prog: u8) -> Dungeon {
    let (hub_combat, west, east) = layout(n);
    let mut rooms = Vec::new();
    let mut objs = Vec::new();
    let mut has = [true; 7];
    has[R_FEAST] = n == 1;
    has[R_PANTRY] = n >= 2;
    for i in 0..7 {
        let mut r = Room::new(i, GRID[i].0 as usize, GRID[i].1 as usize, 7000 + n as u32 * 10 + i as u32);
        for d in 0..4 {
            r.doors[d] = has[i] && neighbor(i, d, &has).is_some();
        }
        if i == R_ENTRY {
            r.doors[1] = true; // way back out
        }
        r.theme = (n - 1).min(3);
        r.visited = i == R_ENTRY;
        r.frame();
        let mut o: Vec<Obj> = Vec::new();
        match i {
            R_ENTRY => {
                for &(x, y) in &[(4, 3), (11, 3), (4, 9), (11, 9)] {
                    r.tiles[y][x] = T_WALL;
                }
                // The pantry hides behind a brittle stretch of the east wall.
                if has[R_PANTRY] && prog & D_PANTRY == 0 {
                    r.set_gap(2, T_CRACK);
                }
            }
            R_FEAST => {
                // Two long banquet tables.
                for y in [4, 8] {
                    for x in 4..=11 {
                        r.tiles[y][x] = T_DECOR;
                    }
                }
            }
            R_PANTRY => {
                for &(x, y) in &[(2, 2), (3, 2), (12, 2), (13, 2), (2, 10), (13, 10), (12, 10)] {
                    r.tiles[y][x] = T_DECOR;
                }
            }
            R_HUB => {
                for &(x, y) in &[(3, 3), (12, 3), (3, 9), (12, 9)] {
                    r.tiles[y][x] = T_WALL;
                }
                if prog & D_DOOR == 0 {
                    r.set_gap(0, T_LOCK);
                }
            }
            R_WEST | R_EAST => {
                let side_east = i == R_EAST;
                let p = if side_east { east } else { west };
                let solved = prog & if side_east { D_EAST } else { D_WEST } != 0;
                build_puzzle(&mut r, &mut o, p, side_east, solved, prog);
            }
            R_STAIRS => {
                for &(x, y) in &[(5, 2), (10, 2)] {
                    r.tiles[y][x] = T_WALL;
                }
                let t = if prog & D_EAST != 0 { T_STAIRS } else { T_BARRIER };
                for dy in 0..2 {
                    for dx in 0..2 {
                        r.tiles[(STAIRS_R + dy) as usize][(STAIRS_C + dx) as usize] = t;
                    }
                }
                for &(x, y) in &[(5, 5), (10, 5)] {
                    let mut t = Obj::new(OK::Torch, x, y);
                    t.on = true;
                    o.push(t);
                }
            }
            _ => {}
        }
        render(&mut r, &themes[dungeon_theme(n)]);
        rooms.push(r);
        objs.push(o);
    }
    let mut puz = [None; 7];
    puz[R_WEST] = Some(west);
    puz[R_EAST] = Some(east);
    // Food rooms are freshly stocked each time you enter the dungeon.
    let larder = (0..7)
        .map(|i| {
            if !has[i] {
                return vec![];
            }
            larder_stock(i)
                .into_iter()
                .map(|(k, c, r)| {
                    let (x, y) = tile_center(c, r);
                    (k, x, y)
                })
                .collect()
        })
        .collect();
    Dungeon {
        n, rooms, objs, puz, has, larder, hub_combat, cur: R_ENTRY, dirty: false, sealed: false, push_t: 0,
        crack_hits: vec![], seen: [false; 7], warned: false,
    }
}

/// Lay out one puzzle chamber. `east` mirrors the west-chamber template.
fn build_puzzle(r: &mut Room, o: &mut Vec<Obj>, p: Puz, east: bool, solved: bool, prog: u8) {
    // The reward: west chambers hold the key chest, east chambers break the stairs seal
    // (directly, or through a lever when the puzzle is about reaching a spot).
    let key_taken = prog & D_KEY != 0;
    let reward = |o: &mut Vec<Obj>, c: i32, row: i32, hidden_until_solved: bool| {
        if east {
            if matches!(p, Puz::Hidden | Puz::IceBridge) {
                let mut l = Obj::new(OK::Lever, c, row);
                l.on = solved;
                o.push(l);
            }
        } else {
            let mut ch = Obj::new(OK::Chest, c, row);
            ch.visible = solved || !hidden_until_solved;
            ch.on = key_taken;
            o.push(ch);
        }
    };
    match p {
        Puz::Combat => {
            for &(x, y) in &[(5, 4), (10, 4), (5, 8), (10, 8)] {
                r.tiles[y][x] = T_WALL;
            }
            reward(o, 8, 6, true);
        }
        Puz::Torches => {
            for &(x, y) in &[(3, 3), (12, 3), (3, 9), (12, 9)] {
                let mut t = Obj::new(OK::Torch, x, y);
                t.on = solved;
                o.push(t);
            }
            o.push(Obj::new(OK::Shrine(Elem::Fire), 8, 6));
            reward(o, 7, 10, true);
        }
        Puz::Plates => {
            let plates = [(4, 3), (11, 3)];
            for &(x, y) in &plates {
                r.tiles[y as usize][x as usize] = T_PLATE;
            }
            for (i, &(x, y)) in [(5, 8), (10, 8)].iter().enumerate() {
                let mut b = Obj::new(OK::Block, x, y);
                if solved {
                    b.c = plates[i].0;
                    b.r = plates[i].1;
                    b.on = true;
                }
                o.push(b);
            }
            reward(o, 7, 10, true);
        }
        Puz::IceBridge => {
            for y in 1..RR - 1 {
                for x in [5, 6] {
                    r.tiles[y][mc(x, east) as usize] = if solved { T_ICE } else { T_WATER };
                }
            }
            o.push(Obj::new(OK::Shrine(Elem::Ice), mc(11, east), 9));
            reward(o, mc(2, east), 6, false);
        }
        Puz::Hidden => {
            for y in 3..=9 {
                r.tiles[y][mc(4, east) as usize] = T_WALL;
            }
            for x in 1..=4 {
                r.tiles[3][mc(x, east) as usize] = T_WALL;
                r.tiles[9][mc(x, east) as usize] = T_WALL;
            }
            let broken = prog & if east { D_CRACK_E } else { D_CRACK_W } != 0;
            r.tiles[6][mc(4, east) as usize] = if broken { T_FLOOR } else { T_CRACK };
            reward(o, mc(2, east), 6, false);
        }
    }
}

impl Game {
    fn dprog(&self) -> u8 {
        self.dungeon.as_ref().map_or(0, |d| self.s.dprog[d.n])
    }
    fn set_dprog(&mut self, flag: u8) {
        if let Some(n) = self.dungeon.as_ref().map(|d| d.n) {
            self.s.dprog[n] |= flag;
            self.save();
        }
    }
    pub(super) fn dungeon_keys(&self) -> i32 {
        let p = self.dprog();
        (p & D_KEY != 0 && p & D_DOOR == 0) as i32
    }

    /// Called after the entrance cinematic: build the dungeon and step inside.
    pub(super) fn start_dungeon(&mut self, n: usize) {
        let d = build_dungeon(n, &self.themes, self.s.dprog[n]);
        self.dungeon = Some(d);
        self.in_lair = 0;
        self.arena = None;
        self.boss = None;
        self.boss_dead = false;
        self.scroll = None;
        self.mode = Mode::Play;
        self.paused = false;
        self.pl = Player::at(128.0, HF - 22.0);
        self.fade = 30;
        self.cam_y = 0.0;
        self.enter_droom();
        self.show_msg(format!("{}. FIND THE KEY AND BREAK THE SEAL ON THE STAIRS.", dungeon_name(n)));
        self.play_song(Some(Song::Field));
    }

    /// Room entry: spawns, combat seals, block resets and hints.
    pub(super) fn enter_droom(&mut self) {
        self.clear_entities();
        let prog = self.dprog();
        let Some(d) = self.dungeon.as_mut() else { return };
        let cur = d.cur;
        let first = !d.seen[cur];
        d.seen[cur] = true;
        d.rooms[cur].visited = true;
        d.push_t = 0;
        d.warned = false;
        let puz = d.puz[cur];
        let side_flag = if cur == R_EAST { D_EAST } else { D_WEST };
        let solved = prog & side_flag != 0;
        // Unsolved block puzzles reset so a stuck block can never soft-lock the player.
        if puz == Some(Puz::Plates) && !solved {
            for b in d.objs[cur].iter_mut().filter(|o| o.k == OK::Block) {
                (b.c, b.r) = b.home;
                b.slide = 0;
            }
        }
        let combat = (puz == Some(Puz::Combat) && !solved) || (cur == R_HUB && d.hub_combat && prog & D_HUB == 0);
        let th = d.rooms[cur].theme;
        if combat {
            for dd in 0..4 {
                if d.rooms[cur].doors[dd] {
                    d.rooms[cur].set_gap(dd, T_SEAL);
                }
            }
            d.sealed = true;
            d.dirty = true;
        }
        let normal = match cur {
            R_ENTRY => 1,
            R_HUB => 2,
            R_STAIRS | R_FEAST | R_PANTRY => 0,
            _ => 2,
        };
        let food: Vec<(IK, f32, f32)> = d.larder[cur].clone();
        self.dungeon_flush();
        for (kind, x, y) in food {
            self.items.push(Item { kind, x, y, val: 0, el: Elem::Neutral, life: LARDER_LIFE, dead: false, tag: 0 });
        }
        if combat {
            self.spawn_pack(5, th, false);
            self.show_msg("THE DOORS SLAM SHUT! DEFEAT EVERY MONSTER.");
            self.sfx(Sfx::Rumble);
            self.shake = 10;
        } else {
            self.spawn_pack(normal, th, false);
            if first {
                let hint = match (cur, puz) {
                    (R_STAIRS, _) if prog & D_EAST == 0 => Some("A MAGIC SEAL GUARDS THE STAIRS. SOLVE THE EAST CHAMBER TO BREAK IT."),
                    (R_STAIRS, _) => Some("THE STAIRS DESCEND INTO DARKNESS..."),
                    (R_FEAST, _) => Some("A FEAST HALL! THE OLD KEEPERS LEFT A MEAL FOR WEARY TRAVELLERS."),
                    (R_PANTRY, _) => Some("A HIDDEN PANTRY, STILL STOCKED WITH FOOD!"),
                    (_, Some(Puz::Torches)) if !solved => Some("FOUR COLD BRAZIERS. PERHAPS FIRE MAGIC WILL WAKE THEM."),
                    (_, Some(Puz::Plates)) if !solved => Some("PRESSURE PLATES... PUSH THE STONE BLOCKS ONTO THEM. LEAVE THE ROOM TO RESET."),
                    (_, Some(Puz::IceBridge)) if !solved => Some("DEEP WATER BLOCKS THE WAY. ICE MAGIC COULD BRIDGE IT."),
                    (_, Some(Puz::Hidden)) if !solved => Some("THE WALLS HERE LOOK OLD AND BRITTLE..."),
                    _ => None,
                };
                if let Some(h) = hint {
                    self.show_msg(h);
                }
            }
        }
    }

    /// Remember what's left on the tables before leaving a food room.
    fn stash_larder(&mut self) {
        let left: Vec<(IK, f32, f32)> =
            self.items.iter().filter(|i| !i.dead && i.life > LARDER_LIFE / 2).map(|i| (i.kind, i.x, i.y)).collect();
        if let Some(d) = self.dungeon.as_mut() {
            if matches!(d.cur, R_FEAST | R_PANTRY) {
                let cur = d.cur;
                d.larder[cur] = left;
            }
        }
    }

    pub(super) fn dungeon_exit(&mut self, dir: usize) {
        self.stash_larder();
        let Some(d) = self.dungeon.as_ref() else { return };
        if d.cur == R_ENTRY && dir == 1 {
            let r = self.gate_room;
            self.go_play(r, GATE_X, SPAWN_Y, false);
            self.fade = 24;
            return;
        }
        let Some(to) = neighbor(d.cur, dir, &d.has) else {
            self.pl.x = self.pl.x.clamp(12.0, WF - 12.0);
            self.pl.y = self.pl.y.clamp(HUDF + 12.0, HF - 12.0);
            return;
        };
        let from = d.cur;
        self.scroll = Some(Scroll { d: dir, t: 0.0, from, to, dun: true });
        self.clear_entities();
        self.place_after_scroll(dir);
    }

    pub(super) fn obj_blocks(&self, c: i32, r: i32) -> bool {
        if self.in_lair > 0 {
            return false;
        }
        self.dungeon.as_ref().map_or(false, |d| d.objs[d.cur].iter().any(|o| o.solid() && o.c == c && o.r == r))
    }

    fn set_tile(&mut self, c: i32, r: i32, t: u8) {
        if let Some(d) = self.dungeon.as_mut() {
            let cur = d.cur;
            d.rooms[cur].tiles[r as usize][c as usize] = t;
            d.dirty = true;
        }
    }
    pub(super) fn dungeon_flush(&mut self) {
        let themes = &self.themes;
        if let Some(d) = self.dungeon.as_mut() {
            if d.dirty {
                let cur = d.cur;
                render(&mut d.rooms[cur], &themes[dungeon_theme(d.n)]);
                d.dirty = false;
            }
        }
    }

    // ------------------------------------------------------------ elemental interactions
    pub(super) fn freeze_water(&mut self, c: i32, r: i32) {
        if self.dungeon.is_none() || self.in_lair > 0 || self.tile_at(c, r) != T_WATER {
            return;
        }
        self.set_tile(c, r, T_ICE);
        let (x, y) = tile_center(c, r);
        self.part(x, y, 0.0, 0.0, 12, rgb(0xa4e4fc), 1, PK::Crystal);
        self.sfx(Sfx::Freeze);
    }
    pub(super) fn frost_water_near(&mut self, x: f32, y: f32, rad: f32) {
        for r in 1..RR as i32 - 1 {
            for c in 1..RC as i32 - 1 {
                let (tx, ty) = tile_center(c, r);
                if dist(x, y, tx, ty) < rad {
                    self.freeze_water(c, r);
                }
            }
        }
    }
    pub(super) fn quake_cracks(&mut self) {
        if self.dungeon.is_none() || self.in_lair > 0 {
            return;
        }
        for r in 0..RR as i32 {
            for c in 0..RC as i32 {
                if self.tile_at(c, r) == T_CRACK {
                    self.break_crack(c, r);
                }
            }
        }
    }
    fn break_crack(&mut self, c: i32, r: i32) {
        let cur = self.dungeon.as_ref().map_or(0, |d| d.cur);
        if cur == R_ENTRY {
            // The pantry wall: the whole doorway crumbles at once.
            for rr in 5..=7 {
                self.set_tile(RC as i32 - 1, rr, T_FLOOR);
            }
            let (x, y) = tile_center(RC as i32 - 1, 6);
            for _ in 0..24 {
                let (vx, vy) = (self.rng.range(-2.2, 0.4), self.rng.range(-2.6, -0.4));
                let col = self.rng.pick(&[rgb(0x686878), rgb(0xa0a0b0), rgb(0x3c3c48)]);
                self.part(x, y, vx, vy, 30, col, 3, PK::Shard);
            }
            self.sfx(Sfx::Rumble);
            self.shake = 10;
            self.set_dprog(D_PANTRY);
            self.show_msg("THE CRACKED WALL CRUMBLES... A HIDDEN PANTRY LIES BEYOND!");
            return;
        }
        self.set_tile(c, r, T_FLOOR);
        let (x, y) = tile_center(c, r);
        for _ in 0..14 {
            let (vx, vy) = (self.rng.range(-1.8, 1.8), self.rng.range(-2.4, -0.4));
            let col = self.rng.pick(&[rgb(0x686878), rgb(0xa0a0b0), rgb(0x3c3c48)]);
            self.part(x, y, vx, vy, 28, col, 3, PK::Shard);
        }
        self.part(x, y, 0.0, 0.0, 12, WHITE, 1, PK::Burst(Elem::Earth, 12.0));
        self.sfx(Sfx::Rumble);
        self.shake = 8;
        self.set_dprog(if cur == R_EAST { D_CRACK_E } else { D_CRACK_W });
        self.show_msg("THE WALL CRUMBLES, REVEALING A HIDDEN PASSAGE!");
    }
    /// A player bolt struck a solid tile or dungeon object.
    pub(super) fn dungeon_bolt_hit(&mut self, c: i32, r: i32, el: Elem) {
        if self.dungeon.is_none() || self.in_lair > 0 {
            return;
        }
        if self.tile_at(c, r) == T_CRACK {
            let hits = {
                let d = self.dungeon.as_mut().unwrap();
                let cur = d.cur;
                match d.crack_hits.iter_mut().find(|h| h.0 == cur && h.1 == c && h.2 == r) {
                    Some(h) => {
                        h.3 += 1;
                        h.3
                    }
                    None => {
                        d.crack_hits.push((cur, c, r, 1));
                        1
                    }
                }
            };
            let (x, y) = tile_center(c, r);
            self.spark(x, y, rgb(0xa0a0b0));
            self.sfx(Sfx::Hit);
            if hits >= 3 || el == Elem::Earth && hits >= 2 {
                self.break_crack(c, r);
            }
            return;
        }
        let idx = {
            let d = self.dungeon.as_ref().unwrap();
            d.objs[d.cur].iter().position(|o| o.solid() && o.c == c && o.r == r)
        };
        let Some(i) = idx else { return };
        let (k, on) = {
            let d = self.dungeon.as_ref().unwrap();
            (d.objs[d.cur][i].k, d.objs[d.cur][i].on)
        };
        match k {
            OK::Torch if el == Elem::Fire && !on => {
                self.light_torch(i);
            }
            OK::Torch if !on => {
                let (x, y) = tile_center(c, r);
                self.float("NEEDS FIRE", x - 40.0, y - 20.0, rgb(0xfc9838));
            }
            // Levers must be pulled by hand, so the far bank of a river can't be reached by a shot.
            _ => {}
        }
    }
    fn light_torch(&mut self, i: usize) {
        let (x, y) = {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
            let o = &d.objs[cur][i];
            tile_center(o.c, o.r)
        };
        self.embers(x, y - 6.0, 12, 1.2);
        self.part(x, y - 6.0, 0.0, 0.0, 14, rgb(0xfc9838), 1, PK::Burst(Elem::Fire, 12.0));
        self.sfx(Sfx::Ignite);
    }
    fn pull_lever(&mut self, i: usize) {
        let (x, y, cur) = {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
            let o = &d.objs[cur][i];
            let (x, y) = tile_center(o.c, o.r);
            (x, y, cur)
        };
        self.spark(x, y, WHITE);
        self.sfx(Sfx::Click);
        if cur == R_EAST {
            self.solve_side(true);
        }
    }

    /// Reward for a solved chamber.
    fn solve_side(&mut self, east: bool) {
        let flag = if east { D_EAST } else { D_WEST };
        if self.dprog() & flag != 0 {
            return;
        }
        self.set_dprog(flag);
        if east {
            let themes = &self.themes;
            if let Some(d) = self.dungeon.as_mut() {
                let sr = &mut d.rooms[R_STAIRS];
                for dy in 0..2 {
                    for dx in 0..2 {
                        sr.tiles[(STAIRS_R + dy) as usize][(STAIRS_C + dx) as usize] = T_STAIRS;
                    }
                }
                render(sr, &themes[dungeon_theme(d.n)]);
            }
            self.show_msg("SOMEWHERE NEARBY, A MAGIC SEAL SHATTERS...");
            self.sfx(Sfx::Rumble);
            self.shake = 14;
        } else {
            let pos = self.dungeon.as_mut().and_then(|d| {
                let cur = d.cur;
                d.objs[cur].iter_mut().find(|o| o.k == OK::Chest).map(|ch| {
                    let was = ch.visible;
                    ch.visible = true;
                    (was, tile_center(ch.c, ch.r))
                })
            });
            if let Some((was, (x, y))) = pos {
                if !was {
                    self.show_msg("A TREASURE CHEST APPEARS!");
                    self.part(x, y, 0.0, 0.0, 16, rgb(0xfcbc3c), 1, PK::Glow(12.0));
                    self.boom(x, y, 16, Elem::Neutral);
                }
            }
            self.sfx(Sfx::Chest);
        }
    }

    // ------------------------------------------------------------ per-frame logic
    /// Player-driven interactions: pushing blocks, doors, stairs, chests, shrines, levers.
    pub(super) fn dungeon_player(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        self.try_push(ix, iy, blocked);
        let (px, py) = (self.pl.x, self.pl.y);
        let cur = self.dungeon.as_ref().map_or(0, |d| d.cur);
        // Locked hub door.
        if cur == R_HUB && self.tile_at(7, 0) == T_LOCK && py < HUDF + 26.0 && (px - 128.0).abs() < 34.0 {
            if self.dungeon_keys() > 0 {
                if let Some(d) = self.dungeon.as_mut() {
                    d.rooms[R_HUB].set_gap(0, T_FLOOR);
                    d.dirty = true;
                }
                self.set_dprog(D_DOOR);
                self.show_msg("THE KEY TURNS. THE DOOR SWINGS OPEN!");
                self.sfx(Sfx::Unlock);
                self.part(128.0, HUDF + 8.0, 0.0, 0.0, 14, rgb(0xfcbc3c), 1, PK::Glow(16.0));
            } else if !self.dungeon.as_ref().map_or(true, |d| d.warned) {
                if let Some(d) = self.dungeon.as_mut() {
                    d.warned = true;
                }
                self.show_msg("THE DOOR IS LOCKED. SEARCH THE SIDE CHAMBERS FOR A KEY.");
                self.sfx(Sfx::Deny);
            }
        }
        // Staircase to the boss.
        if cur == R_STAIRS {
            let (sx, sy) = tile_center(STAIRS_C, STAIRS_R);
            let (cx, cy) = (sx + 8.0, sy + 8.0);
            let near = (px - cx).abs() < 18.0 && py < cy + 20.0;
            if near {
                if self.tile_at(STAIRS_C, STAIRS_R) == T_STAIRS {
                    self.begin_descend();
                    return;
                } else if !self.dungeon.as_ref().map_or(true, |d| d.warned) {
                    if let Some(d) = self.dungeon.as_mut() {
                        d.warned = true;
                    }
                    self.show_msg("THE SEAL HOLDS FAST. SOLVE THE PUZZLE IN THE EAST CHAMBER.");
                    self.sfx(Sfx::Deny);
                }
            }
        }
        // Touch objects.
        let touched: Vec<(usize, OK, bool)> = self.dungeon.as_ref().map_or(vec![], |d| {
            d.objs[d.cur]
                .iter()
                .enumerate()
                .filter(|(_, o)| o.visible)
                .filter(|(_, o)| {
                    let (ox, oy) = tile_center(o.c, o.r);
                    // Solid objects stop the mage ~14px from their centre, so reach a bit further.
                    let reach = if o.solid() { 17.0 } else { 11.0 };
                    (ox - px).abs() < reach && (oy - py).abs() < reach
                })
                .map(|(i, o)| (i, o.k, o.on))
                .collect()
        });
        for (i, k, on) in touched {
            match k {
                OK::Chest if !on => self.open_key_chest(i),
                OK::Shrine(el) if self.el() != el => self.set_element(el),
                OK::Lever if !on => self.pull_lever(i),
                _ => {}
            }
        }
    }
    fn open_key_chest(&mut self, i: usize) {
        let (x, y) = {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
            tile_center(d.objs[cur][i].c, d.objs[cur][i].r)
        };
        self.set_dprog(D_KEY | D_WEST);
        self.float("KEY!", x - 16.0, y - 20.0, rgb(0xfcbc3c));
        self.show_msg("YOU FOUND THE DUNGEON KEY! IT OPENS THE GREAT DOOR IN THE HALL.");
        self.part(x, y, 0.0, 0.0, 16, rgb(0xfcbc3c), 1, PK::Glow(14.0));
        self.sfx(Sfx::Fanfare);
    }
    fn try_push(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        let axis = if ix != 0.0 && iy == 0.0 && blocked.0 {
            Some((ix as i32, 0))
        } else if iy != 0.0 && ix == 0.0 && blocked.1 {
            Some((0, iy as i32))
        } else {
            None
        };
        let Some((dx, dy)) = axis else {
            if let Some(d) = self.dungeon.as_mut() {
                d.push_t = 0;
            }
            return;
        };
        let probe = (self.pl.x + dx as f32 * (self.pl.w / 2.0 + 3.0), self.pl.y + dy as f32 * (self.pl.h / 2.0 + 3.0));
        let (c, r) = tile_of(probe.0, probe.1);
        let Some(d) = self.dungeon.as_ref() else { return };
        let cur = d.cur;
        let Some(bi) = d.objs[cur].iter().position(|o| o.k == OK::Block && o.c == c && o.r == r && o.slide == 0) else {
            return;
        };
        if d.objs[cur][bi].on {
            return; // locked in place on a solved plate
        }
        let (tc, tr) = (c + dx, r + dy);
        let target = self.tile_at(tc, tr);
        let free = matches!(target, T_FLOOR | T_PLATE | T_ICE)
            && tc > 0
            && tr > 0
            && tc < RC as i32 - 1
            && tr < RR as i32 - 1
            && !d.objs[cur].iter().any(|o| o.visible && o.c == tc && o.r == tr)
            && !self.enemies.iter().any(|e| tile_of(e.x, e.y) == (tc, tr));
        let d = self.dungeon.as_mut().unwrap();
        d.push_t += 1;
        if d.push_t < 14 || !free {
            return;
        }
        d.push_t = 0;
        let b = &mut d.objs[cur][bi];
        b.c = tc;
        b.r = tr;
        b.slide = 8;
        b.sdx = dx;
        b.sdy = dy;
        self.sfx(Sfx::Push);
    }
    /// Puzzle checks and animations each frame.
    pub(super) fn dungeon_update(&mut self) {
        let prog = self.dprog();
        let Some(d) = self.dungeon.as_mut() else { return };
        let cur = d.cur;
        for o in d.objs[cur].iter_mut() {
            if o.slide > 0 {
                o.slide -= 1;
            }
        }
        // Combat rooms open when every monster is gone.
        if d.sealed && self.enemies.iter().all(|e| e.dead) {
            d.sealed = false;
            for dd in 0..4 {
                if d.rooms[cur].doors[dd] {
                    d.rooms[cur].set_gap(dd, T_FLOOR);
                }
            }
            if cur == R_HUB && prog & D_DOOR == 0 {
                d.rooms[cur].set_gap(0, T_LOCK);
            }
            d.dirty = true;
            let puz = d.puz[cur];
            self.show_msg("SILENCE FALLS. THE DOORS GRIND OPEN.");
            self.sfx(Sfx::Unlock);
            if cur == R_HUB && puz.is_none() {
                self.set_dprog(D_HUB);
            } else {
                self.solve_side(cur == R_EAST);
            }
            return;
        }
        let side_flag = if cur == R_EAST { D_EAST } else { D_WEST };
        if prog & side_flag != 0 {
            return;
        }
        match d.puz[cur] {
            Some(Puz::Torches) => {
                if d.objs[cur].iter().filter(|o| o.k == OK::Torch).all(|o| o.on) {
                    self.solve_side(cur == R_EAST);
                }
            }
            Some(Puz::Plates) => {
                let room = &d.rooms[cur];
                let blocks: Vec<(i32, i32, i32)> =
                    d.objs[cur].iter().filter(|o| o.k == OK::Block).map(|o| (o.c, o.r, o.slide)).collect();
                let all = blocks.iter().all(|&(c, r, s)| s == 0 && room.tiles[r as usize][c as usize] == T_PLATE);
                if all {
                    for o in d.objs[cur].iter_mut().filter(|o| o.k == OK::Block) {
                        o.on = true;
                    }
                    self.solve_side(cur == R_EAST);
                }
            }
            _ => {}
        }
    }
}
