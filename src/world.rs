// Overworld generation: a 6x6 grid of Zelda-style screens connected by a
// randomized maze, with lairs, treasure, shrines and dead-end secrets.
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
    pub tiles: [[u8; RC]; RR],
    pub img: Sprite,
    /// (x, y, contents, gold value)
    pub chest: Option<(f32, f32, u8, i32)>,
    /// Element index of an orb shrine in the centre of the room.
    pub shrine: Option<usize>,
}

impl Room {
    fn new(i: usize, x: usize, y: usize, seed: u32) -> Self {
        Room {
            i, x, y, doors: [false; 4], visited: false, gate: 0, tank: false, cache: false, dist: -1, theme: 0, seed,
            tiles: [[0; RC]; RR], img: Sprite::new(1, 1), chest: None, shrine: None,
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

pub fn gen_world(themes: &[Theme]) -> (Vec<Room>, usize) {
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
    let mut order: Vec<usize> = (0..rooms.len()).filter(|&i| i != start).collect();
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
        let is_start = r.i == start;
        build_room(r, themes, is_start);
    }
    (rooms, start)
}

fn build_room(r: &mut Room, themes: &[Theme], is_start: bool) {
    let mut rng = Mul(r.seed);
    for y in 0..RR {
        for x in 0..RC {
            r.tiles[y][x] = (x == 0 || y == 0 || x == RC - 1 || y == RR - 1) as u8;
        }
    }
    if r.doors[0] {
        (6..=9).for_each(|x| r.tiles[0][x] = 0);
    }
    if r.doors[1] {
        (6..=9).for_each(|x| r.tiles[RR - 1][x] = 0);
    }
    if r.doors[2] {
        (5..=7).for_each(|y| r.tiles[y][RC - 1] = 0);
    }
    if r.doors[3] {
        (5..=7).for_each(|y| r.tiles[y][0] = 0);
    }
    if !is_start {
        let pat = QPATS[(rng.f() * QPATS.len() as f64) as usize];
        let diag = rng.f() < 0.3;
        for &(c, y) in pat.iter() {
            let (mc, my) = (RC - 1 - c, RR - 1 - y);
            r.tiles[y][c] = 1;
            r.tiles[my][mc] = 1;
            if !diag {
                r.tiles[y][mc] = 1;
                r.tiles[my][c] = 1;
            }
        }
        if rng.f() < 0.35 {
            for _ in 0..30 {
                let c = 1 + (rng.f() * 14.0) as usize;
                let y = 1 + (rng.f() * 11.0) as usize;
                if r.tiles[y][c] > 0 || ((c as f32 - 7.5).abs() < 3.0 && (y as f32 - 6.0).abs() < 2.5) {
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
    render(r, &themes[r.theme]);
}

fn render(r: &mut Room, th: &Theme) {
    let mut img = Sprite::new(W, RR as i32 * TS);
    for y in 0..RR {
        for x in 0..RC {
            let t = if r.tiles[y][x] > 0 { &th.wall } else { &th.floor };
            img.draw(t, x as i32 * TS, y as i32 * TS);
        }
    }
    for y in 0..RR {
        for x in 0..RC {
            if r.tiles[y][x] > 0 {
                continue;
            }
            let (px, py) = (x as i32 * TS, y as i32 * TS);
            if y > 0 && r.tiles[y - 1][x] > 0 {
                img.blend(px, py, TS, 3, BLACK, 0.4);
            }
            if x > 0 && r.tiles[y][x - 1] > 0 {
                img.blend(px, py, 3, TS, BLACK, 0.4);
            }
        }
    }
    r.img = img;
}

/// A closed boss arena with four pillars.
pub fn make_arena(th: &Theme, tier: usize) -> Room {
    let mut r = Room::new(usize::MAX, 0, 0, 0);
    r.theme = tier;
    for y in 0..RR {
        for x in 0..RC {
            r.tiles[y][x] = (x == 0 || y == 0 || x == RC - 1 || y == RR - 1) as u8;
        }
    }
    for &(x, y) in &[(4, 4), (11, 4), (4, 8), (11, 8)] {
        r.tiles[y][x] = 1;
    }
    render(&mut r, th);
    r
}
