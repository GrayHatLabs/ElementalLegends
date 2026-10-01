//! Format 2 lair files: hand-designed dungeons with free room layouts (see
//! docs/LEVEL_FORMAT.md, "Format 2"). Parsed into a `Keep`, which the game builds into a
//! dungeon (src/game/keep.rs).

use crate::levels::{loot, tile, EnemyDef, Loot, Mob};
use crate::world::*;
use serde_json::Value;

/// The five relics, one per lair (the Dark Tower needs them all).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relic {
    Whip,
    Lantern,
    Gloves,
    Boots,
    Cloak,
}
pub const RELICS: [Relic; 5] = [Relic::Whip, Relic::Lantern, Relic::Gloves, Relic::Boots, Relic::Cloak];
impl Relic {
    pub fn idx(self) -> usize {
        self as usize
    }
    pub fn key(self) -> &'static str {
        ["vine_whip", "spirit_lantern", "titan_gloves", "ember_boots", "feather_cloak"][self.idx()]
    }
    pub fn name(self) -> &'static str {
        ["VINE WHIP", "SPIRIT LANTERN", "TITAN GLOVES", "EMBER BOOTS", "FEATHER CLOAK"][self.idx()]
    }
    pub fn from_key(s: &str) -> Option<Relic> {
        RELICS.iter().copied().find(|r| r.key() == s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lock {
    None,
    /// Needs a small key (used up).
    Small,
    /// Needs the big key.
    Big,
    /// Closed while the room's shutter condition is unmet (fights, switches, puzzles).
    Shutter,
    /// A cracked wall: bombs, Quake or three bolts.
    Bomb,
    /// Opens when a lever or floor switch somewhere in the lair sets this flag.
    Flag(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shutter {
    None,
    /// Defeat every monster (the doors slam shut on entry).
    Combat,
    /// Press every floor switch in the room.
    Switch,
    /// Light every torch.
    Torches,
    /// Light every element crystal (in order when they have one).
    Crystals,
    /// Cover every pressure plate with a block, ice block or frozen monster.
    Plates,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KDoor {
    /// 0 n, 1 s, 2 e, 3 w.
    pub side: usize,
    /// Which cell along that side (0 for single-screen rooms).
    pub seg: usize,
    /// Room index, or None for the way out of the lair.
    pub to: Option<usize>,
    pub lock: Lock,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KObj {
    Torch { x: i32, y: i32, lit: bool },
    Block { x: i32, y: i32 },
    IceBlock { x: i32, y: i32 },
    Lever { x: i32, y: i32, sets: Option<String> },
    Chest { x: i32, y: i32, loot: Loot, hidden: bool },
    BigChest { x: i32, y: i32, relic: Option<Relic> },
    Shrine { x: i32, y: i32, element: usize },
    Pot { x: i32, y: i32 },
    Crate { x: i32, y: i32 },
    FloorSwitch { x: i32, y: i32, sets: Option<String> },
    CrystalSwitch { x: i32, y: i32 },
    Crystal { x: i32, y: i32, element: usize, order: u8 },
    Post { x: i32, y: i32 },
    Boulder { x: i32, y: i32 },
    Tablet { x: i32, y: i32, text: String },
    Prop { x: i32, y: i32, name: String, solid: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub struct KRoom {
    pub id: String,
    pub name: String,
    /// Grid position (cells) and size in screens.
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    /// tiles[row][col], 13*h rows of 16*w columns.
    pub tiles: Vec<Vec<u8>>,
    pub dark: bool,
    pub doors: Vec<KDoor>,
    pub shutter: Shutter,
    /// When a flag is set, tiles of one kind turn into another (e.g. water drains to floor).
    pub flag_tiles: Vec<(String, u8, u8)>,
    pub enemies: Vec<EnemyDef>,
    pub random_enemies: Option<i32>,
    pub objects: Vec<KObj>,
    /// The boss staircase (columns 7-8, rows 3-4) is in this room.
    pub boss_stairs: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Keep {
    pub number: usize,
    pub name: Option<String>,
    pub theme: Option<usize>,
    pub relic: Option<Relic>,
    pub rooms: Vec<KRoom>,
    pub entry: usize,
}

fn side(s: &str) -> Option<usize> {
    ["n", "s", "e", "w"].iter().position(|d| *d == s)
}
fn element(s: &str) -> Option<usize> {
    ["fire", "ice", "storm", "earth", "neutral"].iter().position(|e| *e == s)
}
fn mob(s: &str) -> Option<Mob> {
    Some(match s {
        "slime" => Mob::Slime,
        "bat" => Mob::Bat,
        "skeleton" => Mob::Skeleton,
        "imp" => Mob::Imp,
        "ghost" => Mob::Ghost,
        "golem" => Mob::Golem,
        "zombie" => Mob::Zombie,
        "generator" => Mob::Generator,
        _ => return None,
    })
}
/// Format 2 adds pits, lava, coloured barriers, hidden bridges and thorns.
pub fn tile2(c: char) -> Option<u8> {
    tile(c).or(Some(match c {
        'p' => T_PIT,
        'l' => T_LAVA,
        'r' => T_ORANGE,
        'u' => T_BLUE,
        'h' => T_HIDDEN,
        't' => T_THORNS,
        _ => return None,
    }))
}

/// Parse a format 2 lair. Returns the lair and warnings for skipped parts.
pub fn parse_keep(v: &Value) -> Result<(Keep, Vec<String>), String> {
    let o = v.as_object().ok_or("the file must be a JSON object")?;
    if o.get("kind").and_then(Value::as_str) != Some("lair") {
        return Err("format 2 is for lairs (\"kind\": \"lair\")".into());
    }
    let number = o.get("number").and_then(Value::as_u64).ok_or("missing \"number\"")? as usize;
    if !(1..=6).contains(&number) {
        return Err("\"number\" must be 1-6".into());
    }
    let mut warn = vec![];
    let name = o.get("name").and_then(Value::as_str).map(|s| s.to_uppercase()).filter(|s| !s.trim().is_empty());
    let theme = o.get("theme").and_then(Value::as_u64).map(|t| t as usize).filter(|&t| t <= 10);
    let relic = match o.get("relic").and_then(Value::as_str) {
        None => None,
        Some(s) => Some(Relic::from_key(s).ok_or(format!("unknown relic \"{s}\""))?),
    };
    let list = o.get("rooms").and_then(Value::as_array).ok_or("missing \"rooms\" list")?;
    if list.is_empty() {
        return Err("\"rooms\" is empty".into());
    }
    let ids: Vec<String> = list.iter().map(|r| r.get("id").and_then(Value::as_str).unwrap_or("").to_string()).collect();
    for (i, id) in ids.iter().enumerate() {
        if id.is_empty() {
            return Err(format!("room {i} has no \"id\""));
        }
        if ids[..i].contains(id) {
            return Err(format!("two rooms are called \"{id}\""));
        }
    }
    let mut rooms = vec![];
    for (i, rv) in list.iter().enumerate() {
        rooms.push(parse_room(rv, &ids, &mut warn).map_err(|e| format!("room \"{}\": {e}", ids[i]))?);
    }
    // Rooms must not overlap.
    for i in 0..rooms.len() {
        for j in 0..i {
            let (a, b) = (&rooms[i], &rooms[j]);
            if a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h {
                return Err(format!("rooms \"{}\" and \"{}\" overlap", a.id, b.id));
            }
        }
    }
    // Make every door two-way: add the matching door on the other side where missing,
    // and check the two rooms really touch there.
    let mut extra: Vec<(usize, KDoor)> = vec![];
    for (i, r) in rooms.iter().enumerate() {
        for d in &r.doors {
            let Some(j) = d.to else { continue };
            let (cx, cy) = match d.side {
                0 => (r.x + d.seg, r.y),
                1 => (r.x + d.seg, r.y + r.h - 1),
                2 => (r.x + r.w - 1, r.y + d.seg),
                _ => (r.x, r.y + d.seg),
            };
            let (nx, ny) = match d.side {
                0 => (cx as i64, cy as i64 - 1),
                1 => (cx as i64, cy as i64 + 1),
                2 => (cx as i64 + 1, cy as i64),
                _ => (cx as i64 - 1, cy as i64),
            };
            let o = &rooms[j];
            if nx < o.x as i64 || ny < o.y as i64 || nx >= (o.x + o.w) as i64 || ny >= (o.y + o.h) as i64 {
                return Err(format!("room \"{}\": the {} door doesn't lead into \"{}\" (they don't touch there)", r.id, ["north", "south", "east", "west"][d.side], o.id));
            }
            let back = [1, 0, 3, 2][d.side];
            let bseg = if back < 2 { nx as usize - o.x } else { ny as usize - o.y };
            if !o.doors.iter().any(|od| od.side == back && od.seg == bseg) && !extra.iter().any(|(k, od)| *k == j && od.side == back && od.seg == bseg) {
                // The far side mirrors the lock, except shutters, which belong to one room.
                let lock = if d.lock == Lock::Shutter { Lock::None } else { d.lock.clone() };
                extra.push((j, KDoor { side: back, seg: bseg, to: Some(i), lock }));
            }
        }
    }
    for (j, d) in extra {
        rooms[j].doors.push(d);
    }
    let entry = rooms.iter().position(|r| r.doors.iter().any(|d| d.to.is_none())).ok_or("no room has a door out of the lair (\"to\": \"exit\")")?;
    if !rooms.iter().any(|r| r.boss_stairs) {
        warn.push("no room has \"boss_stairs\": the boss can't be reached".into());
    }
    if !rooms.iter().any(|r| r.objects.iter().any(|o| matches!(o, KObj::BigChest { .. }))) && number <= 5 {
        warn.push("no big chest: this lair gives no relic".into());
    }
    Ok((Keep { number, name, theme, relic, rooms, entry }, warn))
}

fn num(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_i64)
}
fn pair(v: &Value, k: &str) -> Option<(usize, usize)> {
    let a = v.get(k)?.as_array()?;
    Some((a.first()?.as_u64()? as usize, a.get(1)?.as_u64()? as usize))
}

fn parse_room(v: &Value, ids: &[String], warn: &mut Vec<String>) -> Result<KRoom, String> {
    let id = v.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let name = v.get("name").and_then(Value::as_str).unwrap_or("").to_uppercase();
    let (x, y) = pair(v, "at").ok_or("needs \"at\": [x, y]")?;
    let (w, h) = pair(v, "size").unwrap_or((1, 1));
    if !(1..=2).contains(&w) || !(1..=2).contains(&h) || x > 12 || y > 12 {
        return Err("\"size\" must be 1 or 2 screens each way and \"at\" within 0-12".into());
    }
    let (rows, cols) = (RR * h, RC * w);
    let tv = v.get("tiles").and_then(Value::as_array).ok_or("needs \"tiles\"")?;
    if tv.len() != rows {
        return Err(format!("\"tiles\" needs {rows} rows, found {}", tv.len()));
    }
    let mut tiles = vec![];
    for (ry, row) in tv.iter().enumerate() {
        let s: Vec<char> = row.as_str().ok_or("tile rows must be strings")?.chars().collect();
        if s.len() != cols {
            return Err(format!("tile row {ry} needs {cols} characters, found {}", s.len()));
        }
        tiles.push(s.iter().enumerate().map(|(cx, &c)| tile2(c).ok_or(format!("unknown tile '{c}' at {cx},{ry}"))).collect::<Result<Vec<u8>, String>>()?);
    }
    let inside = |ox: i64, oy: i64| ox >= 1 && oy >= 1 && ox < cols as i64 - 1 && oy < rows as i64 - 1;
    let mut doors = vec![];
    for d in v.get("doors").and_then(Value::as_array).into_iter().flatten() {
        let s = d.get("side").and_then(Value::as_str).and_then(side).ok_or("a door needs \"side\": n, s, e or w")?;
        let seg = num(d, "seg").unwrap_or(0) as usize;
        if seg >= if s < 2 { w } else { h } {
            return Err(format!("door \"seg\" {seg} is past the room's edge"));
        }
        let to_s = d.get("to").and_then(Value::as_str).ok_or("a door needs \"to\" (a room id or \"exit\")")?;
        let to = if to_s == "exit" { None } else { Some(ids.iter().position(|i| i == to_s).ok_or(format!("door to unknown room \"{to_s}\""))?) };
        let lock = match d.get("lock").and_then(Value::as_str).unwrap_or("none") {
            "none" => Lock::None,
            "small" => Lock::Small,
            "big" => Lock::Big,
            "shutter" => Lock::Shutter,
            "bomb" => Lock::Bomb,
            f if f.starts_with("flag:") && f.len() > 5 => Lock::Flag(f[5..].to_string()),
            other => return Err(format!("unknown door lock \"{other}\"")),
        };
        doors.push(KDoor { side: s, seg, to, lock });
    }
    let shutter = match v.get("shutter").and_then(Value::as_str).unwrap_or("none") {
        "none" => Shutter::None,
        "combat" => Shutter::Combat,
        "switch" => Shutter::Switch,
        "torches" => Shutter::Torches,
        "crystals" => Shutter::Crystals,
        "plates" => Shutter::Plates,
        other => return Err(format!("unknown shutter \"{other}\"")),
    };
    let mut flag_tiles = vec![];
    for f in v.get("flag_tiles").and_then(Value::as_array).into_iter().flatten() {
        let flag = f.get("flag").and_then(Value::as_str).ok_or("flag_tiles needs \"flag\"")?;
        let ch = |k: &str| f.get(k).and_then(Value::as_str).and_then(|s| s.chars().next()).and_then(tile2);
        let (Some(a), Some(b)) = (ch("from"), ch("to")) else { return Err("flag_tiles needs \"from\" and \"to\" tile characters".into()) };
        flag_tiles.push((flag.to_string(), a, b));
    }
    let mut enemies = vec![];
    for e in v.get("enemies").and_then(Value::as_array).into_iter().flatten() {
        let k = e.get("kind").and_then(Value::as_str).unwrap_or("");
        let (Some(kind), Some(ex), Some(ey)) = (mob(k), num(e, "x"), num(e, "y")) else {
            warn.push(format!("room \"{id}\": enemy \"{k}\" skipped (needs a known kind, x and y)"));
            continue;
        };
        if !inside(ex, ey) || solid_tile(tiles[ey as usize][ex as usize]) {
            warn.push(format!("room \"{id}\": enemy at {ex},{ey} is in a wall or outside; skipped"));
            continue;
        }
        let el = e.get("element").and_then(Value::as_str).and_then(element);
        let hp = e.get("hp").and_then(Value::as_f64).map(|h| (h as f32).clamp(1.0, 999.0));
        let still = e.get("still").and_then(Value::as_bool).unwrap_or(false);
        enemies.push(EnemyDef { kind, element: el, x: ex as i32, y: ey as i32, hp, still });
    }
    let mut objects = vec![];
    for ob in v.get("objects").and_then(Value::as_array).into_iter().flatten() {
        let t = ob.get("type").and_then(Value::as_str).unwrap_or("");
        let (Some(ox), Some(oy)) = (num(ob, "x"), num(ob, "y")) else {
            warn.push(format!("room \"{id}\": object \"{t}\" needs x and y; skipped"));
            continue;
        };
        if !inside(ox, oy) {
            warn.push(format!("room \"{id}\": object \"{t}\" at {ox},{oy} is outside; skipped"));
            continue;
        }
        let (x, y) = (ox as i32, oy as i32);
        let flag = |k: &str| ob.get(k).and_then(Value::as_bool).unwrap_or(false);
        let sets = ob.get("sets").and_then(Value::as_str).map(str::to_string);
        let el = || ob.get("element").and_then(Value::as_str).and_then(element).filter(|&e| e < 4);
        let o = match t {
            "torch" => KObj::Torch { x, y, lit: flag("lit") },
            "block" => KObj::Block { x, y },
            "ice_block" => KObj::IceBlock { x, y },
            "lever" => KObj::Lever { x, y, sets },
            "chest" => {
                let c = ob.get("contents").and_then(Value::as_str).unwrap_or("gold:20");
                match loot(c) {
                    Some(l) => KObj::Chest { x, y, loot: l, hidden: flag("hidden") },
                    None => {
                        warn.push(format!("room \"{id}\": unknown chest contents \"{c}\"; skipped"));
                        continue;
                    }
                }
            }
            "big_chest" => KObj::BigChest { x, y, relic: ob.get("relic").and_then(Value::as_str).and_then(Relic::from_key) },
            "shrine" => match el() {
                Some(e) => KObj::Shrine { x, y, element: e },
                None => {
                    warn.push(format!("room \"{id}\": shrine needs an element; skipped"));
                    continue;
                }
            },
            "pot" => KObj::Pot { x, y },
            "crate" => KObj::Crate { x, y },
            "floor_switch" => KObj::FloorSwitch { x, y, sets },
            "crystal_switch" => KObj::CrystalSwitch { x, y },
            "crystal" => match el() {
                Some(e) => KObj::Crystal { x, y, element: e, order: num(ob, "order").unwrap_or(0).clamp(0, 9) as u8 },
                None => {
                    warn.push(format!("room \"{id}\": crystal needs an element; skipped"));
                    continue;
                }
            },
            "whip_post" => KObj::Post { x, y },
            "boulder" => KObj::Boulder { x, y },
            "tablet" => KObj::Tablet { x, y, text: ob.get("text").and_then(Value::as_str).unwrap_or("").to_uppercase() },
            "prop" => KObj::Prop { x, y, name: ob.get("name").and_then(Value::as_str).unwrap_or("").to_string(), solid: ob.get("solid").and_then(Value::as_bool).unwrap_or(true) },
            other => {
                warn.push(format!("room \"{id}\": unknown object type \"{other}\"; skipped"));
                continue;
            }
        };
        objects.push(o);
    }
    Ok(KRoom {
        id,
        name,
        x,
        y,
        w,
        h,
        tiles,
        dark: v.get("dark").and_then(Value::as_bool).unwrap_or(false),
        doors,
        shutter,
        flag_tiles,
        enemies,
        random_enemies: num(v, "random_enemies").map(|n| n.clamp(0, 12) as i32),
        objects,
        boss_stairs: v.get("boss_stairs").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// A static check that the lair can be finished: walk the rooms, collecting small keys,
/// the big key and switch flags, opening doors as they become possible. Returns problems.
pub fn check_solvable(k: &Keep) -> Vec<String> {
    let n = k.rooms.len();
    let mut reach = vec![false; n];
    reach[k.entry] = true;
    let (mut keys_found, mut keys_used, mut big) = (0, 0, false);
    let mut flags: Vec<String> = vec![];
    let mut opened = vec![vec![false; 0]; n];
    for (i, r) in k.rooms.iter().enumerate() {
        opened[i] = vec![false; r.doors.len()];
    }
    let mut taken = vec![vec![false; 0]; n];
    for (i, r) in k.rooms.iter().enumerate() {
        taken[i] = vec![false; r.objects.len()];
    }
    loop {
        let mut progress = false;
        for i in 0..n {
            if !reach[i] {
                continue;
            }
            for (oi, o) in k.rooms[i].objects.iter().enumerate() {
                if taken[i][oi] {
                    continue;
                }
                match o {
                    KObj::Chest { loot: Loot::SmallKey, .. } => keys_found += 1,
                    KObj::Chest { loot: Loot::BigKey, .. } => big = true,
                    KObj::Lever { sets: Some(f), .. } | KObj::FloorSwitch { sets: Some(f), .. } => flags.push(f.clone()),
                    _ => continue,
                }
                taken[i][oi] = true;
                progress = true;
            }
            for di in 0..k.rooms[i].doors.len() {
                let d = &k.rooms[i].doors[di];
                let Some(j) = d.to else { continue };
                if opened[i][di] {
                    continue;
                }
                let ok = match &d.lock {
                    Lock::Small => {
                        if keys_found > keys_used {
                            keys_used += 1;
                            true
                        } else {
                            false
                        }
                    }
                    Lock::Big => big,
                    Lock::Flag(f) => flags.contains(f),
                    _ => true,
                };
                if ok {
                    opened[i][di] = true;
                    // The matching door on the far side opens with it.
                    let back = [1, 0, 3, 2][d.side];
                    if let Some(bi) = k.rooms[j].doors.iter().position(|bd| bd.side == back && bd.to == Some(i)) {
                        opened[j][bi] = true;
                    }
                    if !reach[j] {
                        reach[j] = true;
                    }
                    progress = true;
                }
            }
        }
        if !progress {
            break;
        }
    }
    let mut out = vec![];
    for (i, r) in k.rooms.iter().enumerate() {
        if !reach[i] {
            out.push(format!("room \"{}\" can't be reached", r.id));
        }
    }
    if let Some(s) = k.rooms.iter().position(|r| r.boss_stairs) {
        if !reach[s] {
            out.push("the boss stairs can't be reached".into());
        }
    }
    if k.rooms.iter().any(|r| r.objects.iter().any(|o| matches!(o, KObj::BigChest { .. }))) && !big {
        out.push("the big chest needs the big key, which is never found".into());
    }
    out
}
