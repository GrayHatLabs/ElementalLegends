// Overworld generation: a 6x6 grid of Zelda-style screens connected by a
// randomized maze, with dungeon buildings, treasure, shrines and dead-end
// secrets. Also the shared tile set used by dungeon rooms and boss arenas.
use crate::gfx::*;
use crate::sprites::Theme;
use std::collections::VecDeque;

pub const HUD: i32 = 32;
pub const TS: i32 = 16;
pub const RC: usize = 16;
pub const RR: usize = 13;
pub const WW: usize = 6;
pub const WH: usize = 6;
pub const START_X: usize = 2;
pub const START_Y: usize = 5;
pub const WORLD_SEED: u32 = 1988;

// ---------------------------------------------------------------- tiles
pub const T_FLOOR: u8 = 0;
pub const T_WALL: u8 = 1;
/// Blocks walking, but bolts fly over it. Ice bolts freeze it into T_ICE.
pub const T_WATER: u8 = 2;
pub const T_ICE: u8 = 3;
/// A brittle wall hiding a passage; breaks after a few hits.
pub const T_CRACK: u8 = 4;
/// Locked door, opened with a dungeon key.
pub const T_LOCK: u8 = 5;
/// Portcullis that seals a room during a combat challenge.
pub const T_SEAL: u8 = 6;
/// Magic seal over the boss staircase.
pub const T_BARRIER: u8 = 7;
pub const T_PLATE: u8 = 8;
pub const T_STAIRS: u8 = 9;
/// Solid, drawn as floor: the footprint of big sprites (monolith, buildings, standing stones).
pub const T_DECOR: u8 = 10;

/// Blocks the player and walking enemies.
pub fn solid_tile(t: u8) -> bool {
    matches!(t, T_WALL | T_WATER | T_CRACK | T_LOCK | T_SEAL | T_BARRIER | T_DECOR)
}
/// Stops projectiles.
pub fn shot_solid(t: u8) -> bool {
    matches!(t, T_WALL | T_CRACK | T_LOCK | T_SEAL | T_BARRIER | T_DECOR)
}

/// Seeded PRNG (mulberry32) so the world is identical every playthrough.
pub struct Mul(pub u32);
impl Mul {
    pub fn f(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64) / 4294967296.0
    }
}

/// (dx, dy, opposite direction index) for n, s, e, w.
pub const DIRS: [(i32, i32, usize); 4] = [(0, -1, 1), (0, 1, 0), (1, 0, 3), (-1, 0, 2)];

/// Chest contents codes.
pub const CH_GOLD: u8 = 0;
pub const CH_BREAD: u8 = 1;
pub const CH_POTION: u8 = 2;
pub const CH_GEM: u8 = 3;
pub const CH_MEAT: u8 = 4;

/// Room purpose.
pub const SP_NONE: u8 = 0;
pub const SP_MONOLITH: u8 = 1;
pub const SP_SHOP: u8 = 2;

pub struct Room {
    pub i: usize,
    pub x: usize,
    pub y: usize,
    pub doors: [bool; 4],
    pub visited: bool,
    pub gate: usize,
    pub tank: bool,
    pub cache: bool,
    pub dist: i32,
    pub theme: usize,
    pub seed: u32,
    pub special: u8,
    pub tiles: [[u8; RC]; RR],
    pub img: Sprite,
    /// (x, y, contents, gold value)
    pub chest: Option<(f32, f32, u8, i32)>,
    /// Element index of an orb shrine in the centre of the room.
    pub shrine: Option<usize>,
}

impl Room {
    pub fn new(i: usize, x: usize, y: usize, seed: u32) -> Self {
        Room {
            i, x, y, doors: [false; 4], visited: false, gate: 0, tank: false, cache: false, dist: -1, theme: 0, seed,
            special: SP_NONE, tiles: [[0; RC]; RR], img: Sprite::new(1, 1), chest: None, shrine: None,
        }
    }
    /// Border walls with the standard door gaps for each open side.
    pub fn frame(&mut self) {
        for y in 0..RR {
            for x in 0..RC {
                self.tiles[y][x] = if x == 0 || y == 0 || x == RC - 1 || y == RR - 1 { T_WALL } else { T_FLOOR };
            }
        }
        for d in 0..4 {
            if self.doors[d] {
                self.set_gap(d, T_FLOOR);
            }
        }
    }
    /// Fill one door gap (n, s, e, w) with a tile.
    pub fn set_gap(&mut self, d: usize, t: u8) {
        match d {
            0 => (6..=9).for_each(|x| self.tiles[0][x] = t),
            1 => (6..=9).for_each(|x| self.tiles[RR - 1][x] = t),
            2 => (5..=7).for_each(|y| self.tiles[y][RC - 1] = t),
            _ => (5..=7).for_each(|y| self.tiles[y][0] = t),
        }
    }
}

const QPATS: [&[(usize, usize)]; 8] = [
    &[],
    &[(2, 2), (3, 2), (2, 3), (3, 3)],
    &[(2, 2), (3, 2), (4, 2), (2, 3), (2, 4)],
    &[(1, 3), (2, 3), (3, 3), (4, 3)],
    &[(4, 1), (4, 2), (4, 3)],
    &[(2, 2), (4, 2), (2, 4), (4, 4)],
    &[(3, 3)],
    &[(1, 1), (2, 1), (1, 2)],
];

fn at(x: i32, y: i32) -> Option<usize> {
    if x >= 0 && y >= 0 && (x as usize) < WW && (y as usize) < WH {
        Some(y as usize * WW + x as usize)
    } else {
        None
    }
}

/// Returns (rooms, start room, shop room).
pub fn gen_world(themes: &[Theme]) -> (Vec<Room>, usize, usize) {
    let mut rng = Mul(WORLD_SEED);
    let mut rooms = Vec::new();
    for y in 0..WH {
        for x in 0..WW {
            let seed = (rng.f() * 1e9) as u32;
            rooms.push(Room::new(y * WW + x, x, y, seed));
        }
    }
    let start = START_Y * WW + START_X;
    let mut seen = vec![false; rooms.len()];
    seen[start] = true;
    let mut stack = vec![start];
    while let Some(&c) = stack.last() {
        let (cx, cy) = (rooms[c].x as i32, rooms[c].y as i32);
        let opts: Vec<usize> =
            (0..4).filter(|&d| at(cx + DIRS[d].0, cy + DIRS[d].1).map_or(false, |n| !seen[n])).collect();
        if opts.is_empty() {
            stack.pop();
            continue;
        }
        let d = opts[(rng.f() * opts.len() as f64) as usize];
        let n = at(cx + DIRS[d].0, cy + DIRS[d].1).unwrap();
        rooms[c].doors[d] = true;
        rooms[n].doors[DIRS[d].2] = true;
        seen[n] = true;
        stack.push(n);
    }
    for _ in 0..6 {
        let c = (rng.f() * rooms.len() as f64) as usize;
        let d = (rng.f() * 4.0) as usize;
        if let Some(n) = at(rooms[c].x as i32 + DIRS[d].0, rooms[c].y as i32 + DIRS[d].1) {
            rooms[c].doors[d] = true;
            rooms[n].doors[DIRS[d].2] = true;
        }
    }
    rooms[start].dist = 0;
    let mut q = VecDeque::from([start]);
    while let Some(c) = q.pop_front() {
        for d in 0..4 {
            if !rooms[c].doors[d] {
                continue;
            }
            let n = at(rooms[c].x as i32 + DIRS[d].0, rooms[c].y as i32 + DIRS[d].1).unwrap();
            if rooms[n].dist < 0 {
                rooms[n].dist = rooms[c].dist + 1;
                q.push_back(n);
            }
        }
    }
    let max_d = rooms.iter().map(|r| r.dist).max().unwrap_or(1);
    for r in rooms.iter_mut() {
        r.theme = ((r.dist * 4) / (max_d + 1)).min(3) as usize;
    }
    // The village shop sits one screen away from the monolith, along the first open path.
    let shop = [0usize, 2, 3, 1]
        .iter()
        .filter(|&&d| rooms[start].doors[d])
        .find_map(|&d| at(rooms[start].x as i32 + DIRS[d].0, rooms[start].y as i32 + DIRS[d].1))
        .unwrap_or(start);
    rooms[start].special = SP_MONOLITH;
    if shop != start {
        rooms[shop].special = SP_SHOP;
    }
    let mut order: Vec<usize> = (0..rooms.len()).filter(|&i| i != start && i != shop).collect();
    order.sort_by(|&a, &b| rooms[a].dist.cmp(&rooms[b].dist).then(rooms[a].seed.cmp(&rooms[b].seed)));
    let l = order.len();
    rooms[order[l - 1]].gate = 6;
    for (i, f) in [0.06, 0.24, 0.42, 0.6, 0.8].iter().enumerate() {
        let mut k = (f * (l - 1) as f64) as usize;
        while k < l - 1 && rooms[order[k]].gate > 0 {
            k += 1;
        }
        rooms[order[k]].gate = i + 1;
    }
    let dead: Vec<usize> = order
        .iter()
        .copied()
        .filter(|&i| rooms[i].gate == 0 && rooms[i].doors.iter().filter(|&&d| d).count() == 1)
        .collect();
    let mut tanks = 0;
    for (j, &i) in dead.iter().enumerate() {
        if tanks < 3 && j % 2 == 0 {
            rooms[i].tank = true;
            tanks += 1;
        } else {
            rooms[i].cache = true;
        }
    }
    for r in rooms.iter_mut() {
        build_room(r, themes);
    }
    (rooms, start, shop)
}

fn build_room(r: &mut Room, themes: &[Theme]) {
    let mut rng = Mul(r.seed);
    r.frame();
    match r.special {
        SP_MONOLITH => {
            // The monolith itself and a ring of standing stones.
            for y in 3..=5 {
                for x in 7..=8 {
                    r.tiles[y][x] = T_DECOR;
                }
            }
            for &(x, y) in &[(4, 3), (11, 3), (4, 9), (11, 9)] {
                r.tiles[y][x] = T_DECOR;
            }
        }
        SP_SHOP => {
            // The merchant's cottage along the north wall.
            for y in 1..=2 {
                for x in 6..=9 {
                    r.tiles[y][x] = T_DECOR;
                }
            }
        }
        _ => {
            let pat = QPATS[(rng.f() * QPATS.len() as f64) as usize];
            let diag = rng.f() < 0.3;
            for &(c, y) in pat.iter() {
                let (mc, my) = (RC - 1 - c, RR - 1 - y);
                r.tiles[y][c] = T_WALL;
                r.tiles[my][mc] = T_WALL;
                if !diag {
                    r.tiles[y][mc] = T_WALL;
                    r.tiles[my][c] = T_WALL;
                }
            }
            if r.gate > 0 {
                // Dungeon building footprint; the doorway is just below it at the room centre.
                for y in 2..=5 {
                    for x in 6..=9 {
                        r.tiles[y][x] = T_DECOR;
                    }
                }
            }
            if rng.f() < 0.35 {
                for _ in 0..30 {
                    let c = 1 + (rng.f() * 14.0) as usize;
                    let y = 1 + (rng.f() * 11.0) as usize;
                    if r.tiles[y][c] != T_FLOOR || ((c as f32 - 7.5).abs() < 3.0 && (y as f32 - 6.0).abs() < 2.5) {
                        continue;
                    }
                    let v = rng.f();
                    let (content, val) = if v < 0.55 {
                        (CH_GOLD, 20 + (rng.f() * 40.0) as i32)
                    } else if v < 0.72 {
                        (CH_BREAD, 0)
                    } else if v < 0.84 {
                        (CH_POTION, 0)
                    } else if v < 0.94 {
                        (CH_GEM, 0)
                    } else {
                        (CH_MEAT, 0)
                    };
                    r.chest = Some(((c as i32 * TS + 8) as f32, (HUD + y as i32 * TS + 8) as f32, content, val));
                    break;
                }
            }
            if r.gate == 0 && !r.tank && !r.cache && rng.f() < 0.14 {
                r.shrine = Some((rng.f() * 4.0) as usize);
            }
        }
    }
    render(r, &themes[r.theme]);
}

/// Re-draw a room's static image from its tiles (call after tiles change).
pub fn render(r: &mut Room, th: &Theme) {
    let mut img = Sprite::new(W, RR as i32 * TS);
    // Stable per-tile hash so each room keeps the same tile variants.
    let seed = r.seed ^ (r.i as u32).wrapping_mul(0x9e37_79b9);
    let hash = |x: usize, y: usize| -> u32 {
        let mut h = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663) ^ seed;
        h ^= h >> 13;
        h = h.wrapping_mul(0x5bd1_e995);
        h ^ (h >> 15)
    };
    let floor_at = |x: usize, y: usize| -> &Sprite {
        let v = match hash(x, y) % 10 {
            0..=3 => 0,
            4..=7 => 1,
            8 => 2,
            _ => 3,
        };
        &th.floors[v.min(th.floors.len() - 1)]
    };
    let wall_at = |x: usize, y: usize| -> &Sprite { &th.walls[(hash(x, y) >> 4) as usize % th.walls.len()] };
    for y in 0..RR {
        for x in 0..RC {
            let (px, py) = (x as i32 * TS, y as i32 * TS);
            let t = r.tiles[y][x];
            match t {
                T_WALL => img.draw(wall_at(x, y), px, py),
                T_CRACK => {
                    img.draw(wall_at(x, y), px, py);
                    let c = rgb(0x181010);
                    for &(cx, cy) in &[(7, 2), (6, 3), (7, 4), (8, 5), (8, 6), (7, 7), (9, 7), (10, 8), (6, 8), (5, 9), (7, 10), (8, 11), (8, 12)] {
                        img.fill(px + cx, py + cy, 1, 1, c);
                    }
                    img.fill(px + 3, py + 12, 2, 1, c);
                    img.fill(px + 11, py + 3, 2, 1, c);
                }
                T_WATER => {
                    img.fill(px, py, TS, TS, rgb(0x0c2c6c));
                    img.fill(px, py, TS, 1, rgb(0x08204c));
                    img.fill(px + 2, py + 5, 5, 1, rgb(0x2c5cac));
                    img.fill(px + 9, py + 11, 5, 1, rgb(0x2c5cac));
                }
                T_ICE => {
                    img.fill(px, py, TS, TS, rgb(0x78c8ec));
                    img.fill(px, py, TS, 1, rgb(0xa4e4fc));
                    img.fill(px + 2, py + 11, 6, 1, WHITE);
                    img.fill(px + 8, py + 4, 5, 1, rgb(0xd4f4fc));
                    img.fill(px + 3, py + 3, 1, 1, WHITE);
                }
                T_LOCK => {
                    img.fill(px, py, TS, TS, rgb(0x5c3410));
                    img.fill(px + 1, py + 1, TS - 2, TS - 2, rgb(0x8c5020));
                    img.fill(px, py + 4, TS, 2, rgb(0x404040));
                    img.fill(px, py + 11, TS, 2, rgb(0x404040));
                    if x == 7 || x == 8 {
                        let kx = if x == 7 { px + 13 } else { px + 1 };
                        img.fill(kx, py + 6, 2, 4, rgb(0xfcbc3c));
                        img.fill(kx, py + 8, 2, 2, rgb(0x101010));
                    }
                }
                T_SEAL => {
                    img.draw(floor_at(x, y), px, py);
                    for bx in [1, 5, 9, 13] {
                        img.fill(px + bx, py, 2, TS, rgb(0x5c5c64));
                        img.fill(px + bx, py, 1, TS, rgb(0x9c9ca4));
                    }
                    img.fill(px, py + 3, TS, 2, rgb(0x5c5c64));
                    img.fill(px, py + 11, TS, 2, rgb(0x5c5c64));
                }
                T_PLATE => {
                    img.draw(&th.floor, px, py);
                    img.blend(px + 1, py + 1, 14, 14, BLACK, 0.35);
                    img.fill(px + 2, py + 2, 12, 12, rgb(0x303038));
                    img.fill(px + 3, py + 3, 10, 10, rgb(0x747480));
                    img.fill(px + 3, py + 3, 10, 1, rgb(0xa0a0ac));
                    img.fill(px + 6, py + 6, 4, 4, rgb(0x505058));
                }
                T_STAIRS | T_BARRIER => {
                    img.fill(px, py, TS, TS, rgb(0x080808));
                    for s in 0..4 {
                        let c = [rgb(0x7c7c84), rgb(0x5c5c64), rgb(0x3c3c44), rgb(0x202028)][s as usize];
                        img.fill(px, py + s * 4, TS, 3, c);
                    }
                }
                _ => img.draw(floor_at(x, y), px, py),
            }
        }
    }
    for y in 0..RR {
        for x in 0..RC {
            if solid_tile(r.tiles[y][x]) && r.tiles[y][x] != T_WATER {
                continue;
            }
            let (px, py) = (x as i32 * TS, y as i32 * TS);
            let wallish = |t: u8| matches!(t, T_WALL | T_CRACK | T_LOCK);
            // Soft two-step ambient-occlusion shadow cast by walls onto the floor.
            if y > 0 && wallish(r.tiles[y - 1][x]) {
                img.blend(px, py, TS, 2, BLACK, 0.45);
                img.blend(px, py + 2, TS, 3, BLACK, 0.22);
            }
            if x > 0 && wallish(r.tiles[y][x - 1]) {
                img.blend(px, py, 2, TS, BLACK, 0.4);
                img.blend(px + 2, py, 2, TS, BLACK, 0.18);
            }
        }
    }
    if th.bevel {
        // Raised masonry: a lit top edge and a shadowed front face where walls meet floor.
        let wallish = |t: u8| matches!(t, T_WALL | T_CRACK);
        for y in 0..RR {
            for x in 0..RC {
                if !wallish(r.tiles[y][x]) {
                    continue;
                }
                let (px, py) = (x as i32 * TS, y as i32 * TS);
                if y + 1 < RR && !wallish(r.tiles[y + 1][x]) {
                    img.blend(px, py + 10, TS, 6, BLACK, 0.3);
                    img.blend(px, py + 15, TS, 1, BLACK, 0.4);
                }
                if y > 0 && !wallish(r.tiles[y - 1][x]) {
                    img.blend(px, py, TS, 2, WHITE, 0.16);
                }
            }
        }
    }
    if r.special == SP_MONOLITH {
        // Wildflowers around the clearing.
        let mut fr = Mul(r.seed ^ 0x5eed);
        for _ in 0..70 {
            let (x, y) = ((fr.f() * 224.0) as i32 + 16, (fr.f() * 176.0) as i32 + 16);
            let (tx, ty) = ((x / TS) as usize, (y / TS) as usize);
            if ty < RR && tx < RC && r.tiles[ty][tx] == T_FLOOR {
                let c = [rgb(0xf878f8), rgb(0xfce040), WHITE, rgb(0xa4e4fc)][(fr.f() * 4.0) as usize];
                img.fill(x, y, 1, 1, c);
                img.fill(x, y + 1, 1, 1, rgb(0x2c7c1c));
            }
        }
    }
    r.img = img;
}

/// A closed boss arena with four pillars.
pub fn make_arena(th: &Theme, tier: usize) -> Room {
    let mut r = Room::new(usize::MAX, 0, 0, 0);
    r.theme = tier;
    r.frame();
    for &(x, y) in &[(3, 4), (12, 4), (3, 9), (12, 9)] {
        r.tiles[y][x] = T_WALL;
    }
    render(&mut r, th);
    r
}
