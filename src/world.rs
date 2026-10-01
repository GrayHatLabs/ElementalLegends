// Overworld generation: an 8x8 grid of screen-sized cells grouped into areas of
// 1x1 or 2x2 cells (A Link to the Past style: the camera scrolls inside an area and
// slides between areas). Areas are connected by a randomized maze of doorways and
// hold dungeon buildings, treasure, shrines, encounters and dead-end secrets. Also
// the shared tile set used by dungeon rooms and boss arenas.
use crate::gfx::*;
use crate::sprites::Theme;
use std::collections::{BTreeMap, VecDeque};

pub const HUD: i32 = 32;
/// Tile size in logic units (drawn at 24 screen pixels by the world zoom).
pub const TS: i32 = 16;
/// Tiles per cell (one classic screen).
pub const RC: usize = 16;
pub const RR: usize = 13;
/// Logic size of one cell.
pub const CELL_W: f32 = (RC as i32 * TS) as f32;
pub const CELL_H: f32 = (RR as i32 * TS) as f32;
pub const WW: usize = 8;
pub const WH: usize = 8;
pub const START_X: usize = 3;
pub const START_Y: usize = 7;
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
/// Bottomless pit: the mage falls in (unless floating with the Feather Cloak); monsters avoid it.
pub const T_PIT: u8 = 11;
/// Lava: solid unless the mage wears the Ember Boots.
pub const T_LAVA: u8 = 12;
/// Coloured barrier pegs toggled by crystal switches (orange starts raised, blue lowered).
pub const T_ORANGE: u8 = 13;
pub const T_BLUE: u8 = 14;
/// A hidden bridge: looks and acts like a pit unless the Spirit Lantern reveals it.
pub const T_HIDDEN: u8 = 15;
/// Big-key door.
pub const T_BIGLOCK: u8 = 16;
/// Thorny vines: solid until cut with the Vine Whip.
pub const T_THORNS: u8 = 17;
/// A huge boulder on an overworld path: the Titan Gloves heave it aside.
pub const T_ROCK: u8 = 18;

/// Blocks the player and walking enemies.
pub fn solid_tile(t: u8) -> bool {
    matches!(t, T_WALL | T_WATER | T_CRACK | T_LOCK | T_SEAL | T_BARRIER | T_DECOR | T_BIGLOCK | T_THORNS)
}
/// Stops projectiles.
pub fn shot_solid(t: u8) -> bool {
    matches!(t, T_WALL | T_CRACK | T_LOCK | T_SEAL | T_BARRIER | T_DECOR | T_BIGLOCK | T_THORNS)
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
/// Village layout (tile columns / rows on the village screen).
pub const INN_COLS: std::ops::RangeInclusive<usize> = 2..=5;
pub const INN_ROWS: std::ops::RangeInclusive<usize> = 9..=10;
pub const BOARD_TILE: (usize, usize) = (12, 10);

/// A doorway on side `d` (n, s, e, w) of an area, in edge segment `seg` (one per cell
/// along that side), leading to area `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    pub d: usize,
    pub seg: usize,
    pub to: usize,
}

pub struct Room {
    pub i: usize,
    /// Top-left cell of the area in the world grid.
    pub x: usize,
    pub y: usize,
    /// Size in cells (1x1 or 2x2).
    pub cw: usize,
    pub ch: usize,
    /// Any doorway on each side (n, s, e, w).
    pub doors: [bool; 4],
    /// Overworld doorways; empty for dungeon rooms and arenas (they use `doors`).
    pub links: Vec<Link>,
    pub visited: bool,
    pub gate: usize,
    /// Optional cave entrance on this screen (1..=CAVES), 0 for none.
    pub cave: usize,
    /// Where the cave mouth opens: logic x of its centre line and the tile row just below it.
    pub cave_door: (i32, i32),
    /// Relic gates on this area's doorways: (side, segment, relic 0..5 = whip, lantern,
    /// gloves, boots, cloak). The obstacle sits just inside the doorway.
    pub relic_gates: Vec<(usize, usize, usize)>,
    /// Progression zone: entering zone z needs relic z-1 (0 = open from the start).
    pub zone: usize,
    pub tank: bool,
    pub cache: bool,
    pub dist: i32,
    pub theme: usize,
    pub seed: u32,
    pub special: u8,
    /// tiles[row][col]; rows = RR * ch, cols = RC * cw.
    pub tiles: Vec<Vec<u8>>,
    pub img: Sprite,
    /// Native-resolution (24 px per tile) image from generated tilesets, if the theme has them.
    pub img_hd: Option<Sprite>,
    /// (x, y, contents, gold value)
    pub chest: Option<(f32, f32, u8, i32)>,
    /// Element index of an orb shrine in the centre of the room.
    pub shrine: Option<usize>,
    /// Overworld mini-boss living on this screen (MINI_*), 0 for none.
    pub mini: u8,
    /// Deceiving Dryad: the door leading to her poison-pool screen.
    pub mini_dir: usize,
    /// Fruit trees (tile positions) and which of them is a disguised treant.
    pub trees: Vec<(i32, i32)>,
    pub treant: Option<usize>,
    /// Graveyard tombstones.
    pub graves: Vec<(i32, i32)>,
    /// Poison pool (top-left tile of a 4x2 pool).
    pub pool: Option<(i32, i32)>,
    fruit: bool,
}

/// Overworld mini-boss ids (also their save-flag bit numbers).
pub const MINI_HOARD: u8 = 1;
pub const MINI_DRYAD: u8 = 2;
pub const MINI_TREANT: u8 = 3;
pub const MINI_GRAVE: u8 = 4;
/// The poison pool is 4 tiles wide and 2 tall.
pub const POOL_W: i32 = 4;
pub const POOL_H: i32 = 2;

impl Room {
    pub fn new(i: usize, x: usize, y: usize, seed: u32) -> Self {
        Self::sized(i, x, y, seed, 1, 1)
    }
    pub fn sized(i: usize, x: usize, y: usize, seed: u32, cw: usize, ch: usize) -> Self {
        Room {
            i, x, y, cw, ch, doors: [false; 4], links: vec![], visited: false, gate: 0, cave: 0, cave_door: (8, 6), relic_gates: vec![], zone: 0, tank: false, cache: false,
            dist: -1, theme: 0, seed, special: SP_NONE, tiles: vec![vec![0; RC * cw]; RR * ch], img: Sprite::new(1, 1), img_hd: None,
            chest: None, shrine: None, mini: 0, mini_dir: 0, trees: vec![], treant: None, graves: vec![], pool: None,
            fruit: false,
        }
    }
    pub fn cols(&self) -> usize {
        RC * self.cw
    }
    pub fn rows(&self) -> usize {
        RR * self.ch
    }
    /// Logic width / bottom edge of the area (the top edge is HUD).
    pub fn wf(&self) -> f32 {
        (self.cols() as i32 * TS) as f32
    }
    pub fn hf(&self) -> f32 {
        (HUD + self.rows() as i32 * TS) as f32
    }
    pub fn big(&self) -> bool {
        self.cw > 1 || self.ch > 1
    }
    /// Centre of the area: where buildings, shrines, hearts and hoards stand.
    pub fn center(&self) -> (f32, f32) {
        ((self.cols() as i32 * TS / 2) as f32, (HUD + self.rows() as i32 * TS / 2) as f32)
    }
    /// The link through side `d` at the cell segment containing logic coordinate `along`
    /// (x for n/s sides, y for e/w sides).
    pub fn link_at(&self, d: usize, along: f32) -> Option<Link> {
        let seg = if d < 2 { (along / CELL_W).floor() } else { ((along - HUD as f32) / CELL_H).floor() };
        let seg = seg.max(0.0) as usize;
        self.links.iter().copied().find(|l| l.d == d && l.seg == seg)
    }
    /// A plain screen with nothing else assigned to it.
    fn free(&self, start: usize, shop: usize) -> bool {
        !self.big()
            && self.i != start
            && self.dist >= 2
            && self.i != shop
            && self.gate == 0
            && self.cave == 0
            && !self.tank
            && !self.cache
            && self.special == SP_NONE
            && self.mini == 0
            && self.pool.is_none()
            && !self.fruit
    }
    /// Screens whose layout must stay clear (no random obstacles, chests or shrines).
    fn clean(&self) -> bool {
        matches!(self.mini, MINI_HOARD | MINI_DRYAD | MINI_GRAVE) || self.pool.is_some()
    }
    /// Border walls with door gaps: one per link, or the standard gap for each open side
    /// of a single-cell room without links (dungeon rooms).
    pub fn frame(&mut self) {
        let (cols, rows) = (self.cols(), self.rows());
        for y in 0..rows {
            for x in 0..cols {
                self.tiles[y][x] = if x == 0 || y == 0 || x == cols - 1 || y == rows - 1 { T_WALL } else { T_FLOOR };
            }
        }
        if self.links.is_empty() {
            for d in 0..4 {
                if self.doors[d] {
                    self.set_gap(d, T_FLOOR);
                }
            }
        } else {
            for l in self.links.clone() {
                self.set_gap_seg(l.d, l.seg, T_FLOOR);
            }
        }
    }
    /// Fill one door gap (n, s, e, w) of the first segment with a tile.
    pub fn set_gap(&mut self, d: usize, t: u8) {
        self.set_gap_seg(d, 0, t);
    }
    /// Fill the door gap on side `d` in cell segment `seg`.
    pub fn set_gap_seg(&mut self, d: usize, seg: usize, t: u8) {
        let (cols, rows) = (self.cols(), self.rows());
        let (ox, oy) = (seg * RC, seg * RR);
        match d {
            0 => (6..=9).for_each(|x| self.tiles[0][ox + x] = t),
            1 => (6..=9).for_each(|x| self.tiles[rows - 1][ox + x] = t),
            2 => (5..=7).for_each(|y| self.tiles[oy + y][cols - 1] = t),
            _ => (5..=7).for_each(|y| self.tiles[oy + y][0] = t),
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

fn cell(x: i32, y: i32) -> Option<usize> {
    if x >= 0 && y >= 0 && (x as usize) < WW && (y as usize) < WH {
        Some(y as usize * WW + x as usize)
    } else {
        None
    }
}

/// Divide the grid into 2x2 wilderness areas and 1x1 screens. Returns (x, y, size) per area
/// and the owning area of every cell.
fn partition(rng: &mut Mul) -> (Vec<(usize, usize, usize)>, Vec<usize>) {
    let mut owner = vec![usize::MAX; WW * WH];
    let mut areas = vec![];
    let mut order: Vec<usize> = (0..WW * WH).collect();
    for i in (1..order.len()).rev() {
        let j = (rng.f() * (i + 1) as f64) as usize;
        order.swap(i, j);
    }
    let start = START_Y * WW + START_X;
    // First the big wilderness areas (about two thirds of the map), then single screens.
    const BIG_AREAS: usize = 11;
    for &c in &order {
        if areas.len() >= BIG_AREAS {
            break;
        }
        let (x, y) = (c % WW, c / WW);
        if x + 1 >= WW || y + 1 >= WH {
            continue;
        }
        let quad = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)];
        if quad.iter().all(|&(qx, qy)| {
            let q = qy * WW + qx;
            // The monolith and the screen beside it stay single screens (keeps the world layout stable).
            owner[q] == usize::MAX && q != start && q != start - 1
        }) {
            let id = areas.len();
            for &(qx, qy) in &quad {
                owner[qy * WW + qx] = id;
            }
            areas.push((x, y, 2));
        }
    }
    for c in 0..WW * WH {
        if owner[c] == usize::MAX {
            owner[c] = areas.len();
            areas.push((c % WW, c / WW, 1));
        }
    }
    (areas, owner)
}

/// Every possible doorway between two touching areas: (a, b) -> [(side of a, segment on a, segment on b)].
fn candidates(rooms: &[Room], owner: &[usize]) -> BTreeMap<(usize, usize), Vec<(usize, usize, usize)>> {
    let mut out: BTreeMap<(usize, usize), Vec<(usize, usize, usize)>> = BTreeMap::new();
    for r in rooms {
        for d in 0..4 {
            let len = if d < 2 { r.cw } else { r.ch };
            for seg in 0..len {
                // The cell of this area on side d at this segment, and the one beyond it.
                let (cx, cy) = match d {
                    0 => (r.x + seg, r.y),
                    1 => (r.x + seg, r.y + r.ch - 1),
                    2 => (r.x + r.cw - 1, r.y + seg),
                    _ => (r.x, r.y + seg),
                };
                let Some(n) = cell(cx as i32 + DIRS[d].0, cy as i32 + DIRS[d].1) else { continue };
                let b = owner[n];
                let nb = &rooms[b];
                let (nx, ny) = (n % WW, n / WW);
                let bseg = if d < 2 { nx - nb.x } else { ny - nb.y };
                out.entry((r.i, b)).or_default().push((d, seg, bseg));
            }
        }
    }
    out
}

fn link(rooms: &mut [Room], a: usize, b: usize, (d, seg, bseg): (usize, usize, usize)) {
    rooms[a].links.push(Link { d, seg, to: b });
    rooms[a].doors[d] = true;
    let od = DIRS[d].2;
    rooms[b].links.push(Link { d: od, seg: bseg, to: a });
    rooms[b].doors[od] = true;
}

/// Returns (rooms, start room, shop room).
pub fn gen_world(themes: &[Theme]) -> (Vec<Room>, usize, usize) {
    let mut rng = Mul(WORLD_SEED);
    let (areas, owner) = partition(&mut rng);
    let mut rooms: Vec<Room> = areas
        .iter()
        .enumerate()
        .map(|(i, &(x, y, s))| {
            let seed = (rng.f() * 1e9) as u32;
            Room::sized(i, x, y, seed, s, s)
        })
        .collect();
    let cand = candidates(&rooms, &owner);
    let neighbors = |a: usize| -> Vec<usize> { cand.keys().filter(|k| k.0 == a).map(|k| k.1).collect() };
    let start = owner[START_Y * WW + START_X];
    let mut seen = vec![false; rooms.len()];
    seen[start] = true;
    let mut stack = vec![start];
    while let Some(&c) = stack.last() {
        let opts: Vec<usize> = neighbors(c).into_iter().filter(|&n| !seen[n]).collect();
        if opts.is_empty() {
            stack.pop();
            continue;
        }
        let n = opts[(rng.f() * opts.len() as f64) as usize];
        let segs = &cand[&(c, n)];
        let pick = segs[(rng.f() * segs.len() as f64) as usize];
        link(&mut rooms, c, n, pick);
        seen[n] = true;
        stack.push(n);
    }
    // A few extra loops so the world isn't a pure tree.
    let pairs: Vec<(usize, usize)> = cand.keys().copied().filter(|&(a, b)| a < b).collect();
    for _ in 0..10 {
        let (a, b) = pairs[(rng.f() * pairs.len() as f64) as usize];
        if rooms[a].links.iter().any(|l| l.to == b) {
            continue;
        }
        let segs = &cand[&(a, b)];
        let pick = segs[(rng.f() * segs.len() as f64) as usize];
        link(&mut rooms, a, b, pick);
    }
    rooms[start].dist = 0;
    let mut q = VecDeque::from([start]);
    while let Some(c) = q.pop_front() {
        for l in rooms[c].links.clone() {
            if rooms[l.to].dist < 0 {
                rooms[l.to].dist = rooms[c].dist + 1;
                q.push_back(l.to);
            }
        }
    }
    let max_d = rooms.iter().map(|r| r.dist).max().unwrap_or(1);
    for r in rooms.iter_mut() {
        r.theme = ((r.dist * 4) / (max_d + 1)).min(3) as usize;
    }
    // The village (shop, inn, notice board) is a single screen two or three areas out from the
    // monolith, so the mage has to explore a little to find it.
    let village = |want: &dyn Fn(i32) -> bool| {
        // Also at least two map squares from the monolith, so it isn't simply next door.
        let far = |r: &Room| r.x.abs_diff(START_X) + r.y.abs_diff(START_Y) >= 2;
        let mut v: Vec<usize> =
            (0..rooms.len()).filter(|&i| i != start && !rooms[i].big() && far(&rooms[i]) && want(rooms[i].dist)).collect();
        v.sort_by_key(|&i| (rooms[i].dist, rooms[i].seed));
        v.first().copied()
    };
    let shop = village(&|d| d == 2)
        .or_else(|| village(&|d| d == 3))
        .or_else(|| village(&|d| d >= 1))
        .expect("a single screen for the village");
    rooms[start].special = SP_MONOLITH;
    if shop != start {
        rooms[shop].special = SP_SHOP;
    }
    // Buildings, secrets and encounters live on single screens; 2x2 areas are open wilderness.
    let mut order: Vec<usize> = (0..rooms.len()).filter(|&i| i != start && i != shop && !rooms[i].big()).collect();
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
    let dead: Vec<usize> =
        order.iter().copied().filter(|&i| rooms[i].gate == 0 && rooms[i].links.len() == 1).collect();
    let mut tanks = 0;
    for (j, &i) in dead.iter().enumerate() {
        if tanks < 3 && j % 2 == 0 {
            rooms[i].tank = true;
            tanks += 1;
        } else {
            rooms[i].cache = true;
        }
    }
    assign_minis(&mut rooms, start, shop);
    assign_caves(&mut rooms, start, shop);
    assign_zones(&mut rooms, start, shop);
    for r in rooms.iter_mut() {
        build_room(r, themes);
    }
    (rooms, start, shop)
}

/// Relic progression on the overworld. Lair i (1-5) holds relic i. An area is in zone z
/// when it lies farther from the monolith than lairs 1..z, and walking from a lower zone
/// into zone z needs relic z: thorns (whip), a hidden bridge (lantern), a boulder
/// (gloves), lava (boots) or a pit (cloak) blocks that doorway. The village and the way
/// to it stay open.
fn assign_zones(rooms: &mut [Room], start: usize, shop: usize) {
    let lair_dist: Vec<i32> = (1..=5).map(|g| rooms.iter().find(|r| r.gate == g).map_or(i32::MAX, |r| r.dist)).collect();
    for r in rooms.iter_mut() {
        r.zone = (1..=5).filter(|&i| lair_dist[i - 1] < r.dist).max().unwrap_or(0);
    }
    // Keep the village (and every area on the shortest way there) in zone 0.
    let mut prev = vec![usize::MAX; rooms.len()];
    let mut q = VecDeque::from([start]);
    let mut seen = vec![false; rooms.len()];
    seen[start] = true;
    while let Some(c) = q.pop_front() {
        for l in rooms[c].links.clone() {
            if !seen[l.to] {
                seen[l.to] = true;
                prev[l.to] = c;
                q.push_back(l.to);
            }
        }
    }
    let mut c = shop;
    while c != usize::MAX {
        rooms[c].zone = 0;
        c = prev[c];
    }
    rooms[start].zone = 0;
    // A lair's own building must be reachable before its relic exists.
    for g in 1..=5 {
        if let Some(i) = rooms.iter().position(|r| r.gate == g) {
            if rooms[i].zone >= g {
                rooms[i].zone = g - 1;
            }
        }
    }
    // Gate every doorway that climbs into a higher zone, on the lower side.
    for a in 0..rooms.len() {
        for l in rooms[a].links.clone() {
            let (za, zb) = (rooms[a].zone, rooms[l.to].zone);
            if zb > za {
                rooms[a].relic_gates.push((l.d, l.seg, zb - 1));
            }
        }
    }
}

/// Lay the obstacle for a relic gate just inside a doorway.
fn place_relic_gate(r: &mut Room, side: usize, seg: usize, relic: usize) {
    let t = [T_THORNS, T_HIDDEN, T_ROCK, T_LAVA, T_PIT][relic.min(4)];
    let (cols, rows) = (r.cols(), r.rows());
    let (ox, oy) = (seg * RC, seg * RR);
    // Two tiles deep, across the whole doorway (plus a tile either side for safety).
    let cells: Vec<(usize, usize)> = match side {
        0 => (5..=10).flat_map(|x| [(ox + x, 1), (ox + x, 2)]).collect(),
        1 => (5..=10).flat_map(|x| [(ox + x, rows - 2), (ox + x, rows - 3)]).collect(),
        2 => (4..=8).flat_map(|y| [(cols - 2, oy + y), (cols - 3, oy + y)]).collect(),
        _ => (4..=8).flat_map(|y| [(1, oy + y), (2, oy + y)]).collect(),
    };
    for (x, y) in cells {
        if x < cols && y < rows && matches!(r.tiles[y][x], T_FLOOR | T_WALL | T_DECOR) {
            // Keep walls that frame the doorway; fill the passage itself.
            let edge = match side {
                0 | 1 => x == ox + 5 || x == ox + 10,
                _ => y == oy + 4 || y == oy + 8,
            };
            if edge && r.tiles[y][x] == T_WALL {
                continue;
            }
            r.tiles[y][x] = t;
        }
    }
}

/// Optional caves to clear, numbered from the nearest (1) to the farthest (CAVES).
pub const CAVES: usize = 8;
fn assign_caves(rooms: &mut [Room], start: usize, shop: usize) {
    // Plain single screens and wilderness areas (their mouth opens at the area's centre).
    let ok = |r: &Room| r.free(start, shop) || (r.big() && r.dist >= 2 && r.special == SP_NONE && r.gate == 0 && r.mini == 0);
    let mut spots: Vec<usize> = rooms.iter().filter(|r| ok(r)).map(|r| r.i).collect();
    spots.sort_by_key(|&i| (rooms[i].dist, rooms[i].seed));
    // Spread them out: evenly spaced along the distance order.
    let n = spots.len();
    let want = CAVES.min(n);
    for k in 0..want {
        let i = spots[k * n / want];
        let r = &mut rooms[i];
        r.cave = k + 1;
        r.cave_door = ((r.cols() / 2) as i32, if r.big() { r.rows() as i32 / 2 } else { 6 });
    }
}

/// Fixed screens for the overworld mini-boss encounters (deterministic per world).
fn assign_minis(rooms: &mut [Room], start: usize, shop: usize) {
    let by_seed = |rooms: &[Room], ok: &dyn Fn(&Room) -> bool| -> Vec<usize> {
        let mut v: Vec<usize> = rooms.iter().filter(|r| ok(r)).map(|r| r.i).collect();
        v.sort_by_key(|&i| rooms[i].seed);
        v
    };
    // Hoard Dragon: the farthest dead-end treasure screen (it replaces that hoard).
    let hoard = rooms
        .iter()
        .filter(|r| r.cache)
        .max_by_key(|r| r.dist)
        .map(|r| r.i)
        .or_else(|| by_seed(rooms, &|r| r.free(start, shop)).last().copied());
    if let Some(i) = hoard {
        rooms[i].cache = false;
        rooms[i].mini = MINI_HOARD;
    }
    // Deceiving Dryad: a Greenwood screen next to another plain screen that holds her poison pool.
    'dryad: for pass in 0..2 {
        for a in by_seed(rooms, &|r| r.free(start, shop) && (pass == 1 || r.theme == 0)) {
            for d in [0usize, 2, 3, 1] {
                let Some(b) = rooms[a].links.iter().find(|l| l.d == d).map(|l| l.to) else { continue };
                if rooms[b].free(start, shop) {
                    rooms[a].mini = MINI_DRYAD;
                    rooms[a].mini_dir = d;
                    rooms[b].pool = Some((6, 5));
                    break 'dryad;
                }
            }
        }
    }
    // Graveyard: an Old Crypt screen.
    let grave = by_seed(rooms, &|r| r.free(start, shop) && r.theme == 1)
        .first()
        .copied()
        .or_else(|| by_seed(rooms, &|r| r.free(start, shop)).first().copied());
    if let Some(i) = grave {
        rooms[i].mini = MINI_GRAVE;
    }
    // Fruit trees on three Greenwood screens; one of them hides the treant.
    let mut fruit = by_seed(rooms, &|r| r.free(start, shop) && r.theme == 0);
    if fruit.len() < 3 {
        fruit.extend(by_seed(rooms, &|r| r.free(start, shop) && r.theme != 0));
    }
    for (k, &i) in fruit.iter().take(3).enumerate() {
        rooms[i].fruit = true;
        if k == 0 {
            rooms[i].mini = MINI_TREANT;
        }
    }
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
            // The village: the merchant's cottage along the north wall, the inn in the
            // south-west corner and the notice board to the south-east.
            for y in 1..=2 {
                for x in 6..=9 {
                    r.tiles[y][x] = T_DECOR;
                }
            }
            for y in INN_ROWS {
                for x in INN_COLS {
                    r.tiles[y][x] = T_DECOR;
                }
            }
            r.tiles[BOARD_TILE.1][BOARD_TILE.0] = T_DECOR;
        }
        _ => {
            let clean = r.clean();
            // One mirrored obstacle pattern per cell (a 2x2 wilderness area gets four).
            for qy in 0..r.ch {
                for qx in 0..r.cw {
                    let pat = QPATS[(rng.f() * QPATS.len() as f64) as usize];
                    let diag = rng.f() < 0.3;
                    let (ox, oy) = (qx * RC, qy * RR);
                    for &(c, y) in pat.iter().filter(|_| !clean) {
                        let (mc, my) = (RC - 1 - c, RR - 1 - y);
                        r.tiles[oy + y][ox + c] = T_WALL;
                        r.tiles[oy + my][ox + mc] = T_WALL;
                        if !diag {
                            r.tiles[oy + y][ox + mc] = T_WALL;
                            r.tiles[oy + my][ox + c] = T_WALL;
                        }
                    }
                }
            }
            if r.mini == MINI_GRAVE {
                for &(c, y) in &[(3, 3), (5, 3), (10, 3), (12, 3), (3, 9), (5, 9), (10, 9), (12, 9)] {
                    r.tiles[y][c] = T_DECOR;
                    r.graves.push((c as i32, y as i32));
                }
            }
            if r.fruit {
                let spots = [(3, 2), (12, 2), (3, 10), (12, 10), (5, 9), (10, 3), (2, 6), (13, 6)];
                let want = if r.mini == MINI_TREANT { 3 } else { 2 };
                for &(c, y) in spots.iter() {
                    if r.trees.len() >= want {
                        break;
                    }
                    if r.tiles[y][c] == T_FLOOR {
                        r.tiles[y][c] = T_DECOR;
                        r.trees.push((c as i32, y as i32));
                    }
                }
                if r.mini == MINI_TREANT && !r.trees.is_empty() {
                    r.treant = Some(r.trees.len() - 1);
                }
            }
            if r.cave > 0 {
                // A rocky hillside with the mouth above cave_door (like a building door),
                // and open ground in front of it.
                let (dc, dr) = (r.cave_door.0 as usize, r.cave_door.1 as usize);
                for y in dr - 3..=dr + 1 {
                    for x in dc - 3..=dc + 2 {
                        r.tiles[y][x] = if y < dr && (dc - 2..=dc + 1).contains(&x) { T_DECOR } else { T_FLOOR };
                    }
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
            // Wilderness areas always hide a chest somewhere; screens sometimes do.
            let chest_odds = if r.big() { 1.0 } else { 0.35 };
            let (mid_c, mid_r) = (r.cols() as f32 / 2.0 - 0.5, r.rows() as f32 / 2.0 - 0.5);
            if rng.f() < chest_odds && !clean {
                for _ in 0..30 {
                    let c = 1 + (rng.f() * (r.cols() - 2) as f64) as usize;
                    let y = 1 + (rng.f() * (r.rows() - 2) as f64) as usize;
                    if r.tiles[y][c] != T_FLOOR || ((c as f32 - mid_c).abs() < 3.0 && (y as f32 - mid_r).abs() < 2.5) {
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
            if r.gate == 0 && r.cave == 0 && !r.tank && !r.cache && !clean && r.mini == 0 && rng.f() < 0.14 {
                r.shrine = Some((rng.f() * 4.0) as usize);
            }
        }
    }
    for (side, seg, relic) in r.relic_gates.clone() {
        place_relic_gate(r, side, seg, relic);
    }
    render(r, &themes[r.theme]);
}

/// Re-draw a room's static image from its tiles (call after tiles change).
pub fn render(r: &mut Room, th: &Theme) {
    let (cols, rows) = (r.cols(), r.rows());
    let mut img = Sprite::new(cols as i32 * TS, rows as i32 * TS);
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
    for y in 0..rows {
        for x in 0..cols {
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
                T_PIT | T_HIDDEN => {
                    img.fill(px, py, TS, TS, rgb(0x040406));
                    let edge = |yy: usize, xx: usize| yy < rows && xx < cols && !matches!(r.tiles[yy][xx], T_PIT | T_HIDDEN);
                    if y > 0 && edge(y - 1, x) {
                        img.fill(px, py, TS, 3, rgb(0x34303c));
                        img.fill(px, py + 3, TS, 1, rgb(0x18161e));
                    }
                }
                T_LAVA => {
                    img.fill(px, py, TS, TS, rgb(0xc83000));
                    img.fill(px + 2, py + 3, 6, 2, rgb(0xfc9838));
                    img.fill(px + 9, py + 10, 5, 2, rgb(0xfce040));
                    img.fill(px + 1, py + 12, 3, 1, rgb(0x881800));
                }
                T_ORANGE | T_BLUE => img.draw(floor_at(x, y), px, py),
                T_ROCK => {
                    img.draw(floor_at(x, y), px, py);
                    img.fill(px + 1, py + 3, 14, 12, rgb(0x5c5c64));
                    img.fill(px + 2, py + 2, 12, 2, rgb(0x8c8c98));
                    img.fill(px + 12, py + 4, 3, 10, rgb(0x3c3c44));
                    img.fill(px + 4, py + 7, 3, 2, rgb(0x3c3c44));
                }
                T_BIGLOCK => {
                    img.fill(px, py, TS, TS, rgb(0x3c2410));
                    img.fill(px + 1, py + 1, TS - 2, TS - 2, rgb(0x7c4818));
                    img.fill(px, py + 3, TS, 2, rgb(0xd8a040));
                    img.fill(px, py + 11, TS, 2, rgb(0xd8a040));
                    if x == 7 || x == 8 {
                        let kx = if x == 7 { px + 12 } else { px };
                        img.fill(kx, py + 5, 4, 6, rgb(0xfce040));
                        img.fill(kx + 1, py + 7, 2, 2, rgb(0xd82800));
                    }
                }
                T_THORNS => {
                    img.draw(floor_at(x, y), px, py);
                    for k in 0..4 {
                        let (ax, ay) = (px + (k * 5) % 14, py + (k * 7) % 12);
                        img.fill(ax, ay + 2, 8, 2, rgb(0x2c5c18));
                        img.fill(ax + 2, ay, 2, 6, rgb(0x2c5c18));
                        img.fill(ax + 6, ay + 1, 1, 1, rgb(0xd8d8a0));
                    }
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
    for y in 0..rows {
        for x in 0..cols {
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
        for y in 0..rows {
            for x in 0..cols {
                if !wallish(r.tiles[y][x]) {
                    continue;
                }
                let (px, py) = (x as i32 * TS, y as i32 * TS);
                if y + 1 < rows && !wallish(r.tiles[y + 1][x]) {
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
            if ty < rows && tx < cols && r.tiles[ty][tx] == T_FLOOR {
                let c = [rgb(0xf878f8), rgb(0xfce040), WHITE, rgb(0xa4e4fc)][(fr.f() * 4.0) as usize];
                img.fill(x, y, 1, 1, c);
                img.fill(x, y + 1, 1, 1, rgb(0x2c7c1c));
            }
        }
    }
    r.img = img;
    r.img_hd = th.hd_wall.as_ref().map(|wall| render_hd(r, th, wall));
}

/// Screen pixels per tile in native art.
pub const HD_TS: i32 = 24;

/// Native-resolution room image built on a dual grid: each display tile sits on a cell
/// corner and is picked from the four cells around it (corner tileset), so floor, wall and
/// water edges blend smoothly. Special tiles (locks, seals, stairs, plates, ice, cracks)
/// are overlaid from the code-drawn image, scaled up.
fn render_hd(r: &Room, th: &Theme, wall: &[Sprite]) -> Sprite {
    let (cols, rows) = (r.cols() as i32, r.rows() as i32);
    let mut img = Sprite::new(cols * HD_TS, rows * HD_TS);
    let t = |c: i32, rr: i32| -> u8 {
        if c < 0 || rr < 0 || c >= cols || rr >= rows {
            T_WALL
        } else {
            r.tiles[rr as usize][c as usize]
        }
    };
    let wallish = |v: u8| matches!(v, T_WALL | T_CRACK | T_LOCK);
    let wet = |v: u8| matches!(v, T_WATER | T_ICE);
    let half = HD_TS / 2;
    for j in 0..=rows {
        for i in 0..=cols {
            let cs = [t(i - 1, j - 1), t(i, j - 1), t(i - 1, j), t(i, j)]; // NW, NE, SW, SE
            let bits = |f: &dyn Fn(u8) -> bool| cs.iter().fold(0usize, |a, &v| (a << 1) | f(v) as usize);
            let wb = bits(&|v| wallish(v));
            let tile = match (&th.hd_water, wb) {
                (_, w) if w != 0 => &wall[w],
                (Some(water), _) if cs.iter().any(|&v| wet(v)) => &water[bits(&|v| !wet(v))],
                _ => &wall[0],
            };
            img.draw(tile, i * HD_TS - half, j * HD_TS - half);
        }
    }
    // Break up the repeated floor tile with small deterministic details in its own colours:
    // grass tufts outdoors in Greenwood, pebbles and cracks everywhere else.
    let base = {
        let f = &wall[0];
        let (mut sr, mut sg, mut sb, mut n) = (0u32, 0u32, 0u32, 0u32);
        for &p in f.px.iter().filter(|&&p| p != 0) {
            sr += (p >> 16) & 255;
            sg += (p >> 8) & 255;
            sb += p & 255;
            n += 1;
        }
        let n = n.max(1);
        rgb(((sr / n) << 16) | ((sg / n) << 8) | (sb / n))
    };
    let (dark, light) = (mix(base, BLACK, 0.3), mix(base, WHITE, 0.2));
    let grassy = th.name == "GREENWOOD";
    let cell_hash = |c: i32, rr: i32, salt: u32| -> u32 {
        let mut h = (c as u32).wrapping_mul(0x27d4_eb2d) ^ (rr as u32).wrapping_mul(0x1656_67b1) ^ r.seed ^ salt;
        h ^= h >> 15;
        h = h.wrapping_mul(0x85eb_ca6b);
        h ^ (h >> 13)
    };
    for rr in 0..rows {
        for c in 0..cols {
            if t(c, rr) != T_FLOOR {
                continue;
            }
            for k in 0..2u32 {
                let h = cell_hash(c, rr, k * 0x9e37);
                if h % 100 >= if grassy { 55 } else { 30 } {
                    continue;
                }
                let (x, y) = (c * HD_TS + 3 + (h >> 8) as i32 % 18, rr * HD_TS + 3 + (h >> 16) as i32 % 18);
                if grassy {
                    img.fill(x, y, 1, 2, dark);
                    img.fill(x + 2, y, 1, 2, dark);
                    img.fill(x + 1, y + 1, 1, 2, dark);
                    img.fill(x + 1, y, 1, 1, light);
                } else {
                    img.fill(x, y, 2, 1, dark);
                    img.fill(x + 1, y + 1, 1, 1, dark);
                    img.fill(x, y - 1, 1, 1, light);
                }
            }
        }
    }
    if r.special == SP_MONOLITH {
        // Wildflowers around the clearing (same placement as the code-drawn version).
        let mut fr = Mul(r.seed ^ 0x5eed);
        for _ in 0..90 {
            let (x, y) = ((fr.f() * 224.0) as i32 + 16, (fr.f() * 176.0) as i32 + 16);
            let (tx, ty) = (x / TS, y / TS);
            let c = [rgb(0xf878f8), rgb(0xfce040), WHITE, rgb(0xa4e4fc)][(fr.f() * 4.0) as usize];
            if t(tx, ty) == T_FLOOR {
                let (hx, hy) = (x * HD_TS / TS, y * HD_TS / TS);
                img.fill(hx, hy, 2, 2, c);
                img.fill(hx, hy + 2, 1, 2, rgb(0x2c7c1c));
            }
        }
    }
    // Props scattered on open ground (overworld only, deterministic per room).
    let plain = r.special == SP_NONE && r.gate == 0 && r.mini == 0 && r.pool.is_none();
    if plain && !th.hd_deco.is_empty() {
        for rr in 2..rows - 2 {
            for c in 2..cols - 2 {
                let clear = (-1..=1).all(|dy| (-1..=1).all(|dx| t(c + dx, rr + dy) == T_FLOOR));
                let mut h = (c as u32).wrapping_mul(73_856_093) ^ (rr as u32).wrapping_mul(19_349_663) ^ r.seed;
                h ^= h >> 13;
                h = h.wrapping_mul(0x5bd1_e995);
                h ^= h >> 15;
                if clear && h % 19 == 0 {
                    let d = &th.hd_deco[(h >> 8) as usize % th.hd_deco.len()];
                    img.draw(d, c * HD_TS + (HD_TS - d.w) / 2, rr * HD_TS + (HD_TS - d.h) / 2);
                }
            }
        }
    }
    // Special tiles keep their code-drawn look, scaled up from the 16 px image.
    for rr in 0..rows {
        for c in 0..cols {
            if !matches!(t(c, rr), T_ICE | T_CRACK | T_LOCK | T_SEAL | T_PLATE | T_STAIRS | T_BARRIER | T_PIT | T_HIDDEN | T_LAVA | T_BIGLOCK) {
                continue;
            }
            for dy in 0..HD_TS {
                for dx in 0..HD_TS {
                    let (sx, sy) = (c * TS + dx * TS / HD_TS, rr * TS + dy * TS / HD_TS);
                    let p = r.img.px[(sy * r.img.w + sx) as usize];
                    if p != 0 {
                        img.px[((rr * HD_TS + dy) * img.w + c * HD_TS + dx) as usize] = p;
                    }
                }
            }
        }
    }
    img
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprites::build_themes;

    #[test]
    fn world_layout_is_sane() {
        let (rooms, start, shop) = gen_world(&build_themes());
        // Every cell belongs to exactly one area.
        let mut cover = vec![0; WW * WH];
        for r in &rooms {
            for y in r.y..r.y + r.ch {
                for x in r.x..r.x + r.cw {
                    cover[y * WW + x] += 1;
                }
            }
        }
        assert!(cover.iter().all(|&n| n == 1), "cells covered exactly once");
        assert!(rooms.iter().any(|r| r.big()), "some 2x2 wilderness areas exist");
        assert!(!rooms[start].big() && !rooms[shop].big(), "monolith and shop are single screens");
        assert!((2..=3).contains(&rooms[shop].dist), "the village is two or three areas from the monolith");
        assert!(rooms[shop].x.abs_diff(START_X) + rooms[shop].y.abs_diff(START_Y) >= 2, "the village isn't next door on the map");
        // Relic progression: with k relics every area of zone <= k is reachable, which
        // includes lair k+1 (holding relic k+1); the village needs none.
        for k in 0..=5 {
            let mut seen = vec![false; rooms.len()];
            seen[start] = true;
            let mut q = VecDeque::from([start]);
            while let Some(c) = q.pop_front() {
                for l in &rooms[c].links {
                    let gated = rooms[c].relic_gates.iter().any(|g| g.0 == l.d && g.1 == l.seg && g.2 >= k);
                    if !seen[l.to] && !gated {
                        seen[l.to] = true;
                        q.push_back(l.to);
                    }
                }
            }
            assert!(seen[shop], "the village is reachable with {k} relics");
            for r in rooms.iter().filter(|r| r.zone <= k) {
                assert!(seen[r.i], "area {} (zone {}) is reachable with {k} relics", r.i, r.zone);
            }
            if k < 5 {
                let lair = rooms.iter().find(|r| r.gate == k + 1).unwrap();
                assert!(seen[lair.i], "lair {} is reachable with {k} relics", k + 1);
            }
        }
        assert!(rooms.iter().any(|r| !r.relic_gates.is_empty()), "the world has relic gates");
        let caves: Vec<&Room> = rooms.iter().filter(|r| r.cave > 0).collect();
        assert_eq!(caves.len(), CAVES, "every cave has a screen");
        assert!(caves.iter().all(|r| r.gate == 0 && r.mini == 0 && r.special == SP_NONE && r.dist >= 2), "caves are on screens of their own");
        for r in &caves {
            let (c, y) = (r.cave_door.0 as usize, r.cave_door.1 as usize);
            assert!(r.tiles[y][c] == T_FLOOR && r.tiles[y][c - 1] == T_FLOOR && r.tiles[y - 1][c] == T_DECOR, "cave {} has an open mouth", r.cave);
        }
        for k in 1..=CAVES {
            assert_eq!(caves.iter().filter(|r| r.cave == k).count(), 1, "cave {k} exists once");
        }
        for n in 1..=6 {
            let g: Vec<&Room> = rooms.iter().filter(|r| r.gate == n).collect();
            assert_eq!(g.len(), 1, "exactly one building for lair {n}");
            assert!(!g[0].big(), "lair {n} building is on a single screen");
        }
        // Everything reachable, and links are symmetric with matching gaps.
        assert!(rooms.iter().all(|r| r.dist >= 0), "all areas reachable from the start");
        for r in &rooms {
            for l in &r.links {
                let back = rooms[l.to].links.iter().find(|b| b.to == r.i && b.d == DIRS[l.d].2);
                assert!(back.is_some(), "link {} -> {} has a way back", r.i, l.to);
            }
        }
        let big = rooms.iter().filter(|r| r.big()).count();
        println!("areas: {} ({} big), start {} at {:?} links {:?}, shop {} at {:?}", rooms.len(), big, start, (rooms[start].x, rooms[start].y), rooms[start].links, shop, (rooms[shop].x, rooms[shop].y));
    }

    #[test]
    fn doorways_line_up_between_areas() {
        let (rooms, _, _) = gen_world(&build_themes());
        for r in &rooms {
            for l in &r.links {
                let o = &rooms[l.to];
                // Global cell of the gap on each side must be adjacent.
                let (ax, ay) = match l.d {
                    0 | 1 => (r.x + l.seg, if l.d == 0 { r.y } else { r.y + r.ch - 1 }),
                    2 => (r.x + r.cw - 1, r.y + l.seg),
                    _ => (r.x, r.y + l.seg),
                };
                let (bx, by) = (ax as i32 + DIRS[l.d].0, ay as i32 + DIRS[l.d].1);
                assert!(bx >= o.x as i32 && bx < (o.x + o.cw) as i32 && by >= o.y as i32 && by < (o.y + o.ch) as i32);
                let gap_open = match l.d {
                    0 => r.tiles[0][l.seg * RC + 7] == T_FLOOR,
                    1 => r.tiles[r.rows() - 1][l.seg * RC + 7] == T_FLOOR,
                    2 => r.tiles[l.seg * RR + 6][r.cols() - 1] == T_FLOOR,
                    _ => r.tiles[l.seg * RR + 6][0] == T_FLOOR,
                };
                assert!(gap_open, "gap carved for link {} side {} seg {}", r.i, l.d, l.seg);
            }
        }
    }
}