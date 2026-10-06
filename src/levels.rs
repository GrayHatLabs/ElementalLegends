//! Hand-made cave and lair rooms from JSON level files (see docs/LEVEL_FORMAT.md).
//!
//! Built-in files are embedded by `scripts/import_levels.py` (src/levels_gen.rs); a
//! `levels/` folder next to the executable or in the current directory overrides them.
//! Bad files never crash the game: problems are printed and the room (or file) is skipped.

use crate::world::*;
use serde_json::Value;
use std::collections::BTreeMap;

pub const ROOM_ROWS: usize = RR;
pub const ROOM_COLS: usize = RC;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Cave,
    Lair,
}

/// Monster kinds a level can place (maps onto the game's enemy kinds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mob {
    Slime,
    Bat,
    Skeleton,
    Imp,
    Ghost,
    Golem,
    Zombie,
    Generator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnemyDef {
    pub kind: Mob,
    /// 0 fire, 1 ice, 2 storm, 3 earth, 4 neutral; None = the kind's usual element.
    pub element: Option<usize>,
    pub x: i32,
    pub y: i32,
    pub hp: Option<f32>,
    pub still: bool,
}

/// What a chest holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loot {
    /// Whatever the game would normally put there.
    Default,
    Key,
    Gold(i32),
    Bombs(i32),
    Elixirs(i32),
    Heart,
    Mana,
    Potion,
    Page,
    /// Format 2 lairs: a small key, the map, the finder and the big key.
    SmallKey,
    Map,
    Finder,
    BigKey,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjDef {
    Torch { x: i32, y: i32, lit: bool },
    Block { x: i32, y: i32 },
    Lever { x: i32, y: i32, pulled: bool },
    Chest { x: i32, y: i32, loot: Loot, hidden: bool },
    Shrine { x: i32, y: i32, element: usize },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoomDef {
    /// tiles[row][col] as world T_* values (door gaps are carved by the game).
    pub tiles: Option<Vec<Vec<u8>>>,
    /// "none", "combat", "torches", "plates", "icebridge", "hidden", "freeze_plate".
    pub puzzle: Option<String>,
    pub combat: bool,
    pub enemies: Vec<EnemyDef>,
    pub random_enemies: Option<i32>,
    pub objects: Vec<ObjDef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub kind: Kind,
    pub number: usize,
    pub name: Option<String>,
    pub theme: Option<usize>,
    pub rooms: BTreeMap<String, RoomDef>,
}

/// Every loaded level, keyed by file stem ("cave_2", "lair_1").
#[derive(Default)]
pub struct Levels {
    pub by_name: BTreeMap<String, Level>,
    /// Format 2 lairs (free room layouts), by lair number.
    pub keeps: BTreeMap<usize, crate::keepdef::Keep>,
}

impl Levels {
    /// Built-in levels, then overrides from `levels/` folders.
    pub fn load() -> Self {
        let mut out = Levels::default();
        for (file, text) in crate::levels_gen::LEVELS {
            out.add(file, text, "built-in");
        }
        let mut dirs = vec![];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(d) = exe.parent() {
                dirs.push(d.join("levels"));
            }
        }
        dirs.push(std::path::PathBuf::from("levels"));
        let mut seen = std::collections::BTreeSet::new();
        for d in dirs {
            let Ok(canon) = d.canonicalize() else { continue };
            if !seen.insert(canon.clone()) {
                continue;
            }
            let Ok(rd) = std::fs::read_dir(&canon) else { continue };
            let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().map_or(false, |x| x == "json")).collect();
            files.sort();
            for p in files {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                match std::fs::read_to_string(&p) {
                    Ok(text) => out.add(&name, &text, &canon.display().to_string()),
                    Err(e) => eprintln!("levels: can't read {}: {e}", p.display()),
                }
            }
        }
        out
    }
    pub fn add(&mut self, file: &str, text: &str, from: &str) {
        let stem = file.trim_end_matches(".json").to_string();
        // Format 2: a whole lair with free room layouts.
        if let Ok(v) = serde_json::from_str::<Value>(text) {
            if v.get("format").and_then(Value::as_i64) == Some(2) {
                match crate::keepdef::parse_keep(&v) {
                    Ok((k, warnings)) => {
                        for w in &warnings {
                            eprintln!("levels: {file} ({from}): {w}");
                        }
                        for p in crate::keepdef::check_solvable(&k) {
                            eprintln!("levels: {file} ({from}): {p}");
                        }
                        if stem != format!("lair_{}", k.number) {
                            eprintln!("levels: {file} ({from}): number says lair_{}.json, ignoring the file", k.number);
                            return;
                        }
                        self.by_name.remove(&stem);
                        self.keeps.insert(k.number, k);
                    }
                    Err(e) => eprintln!("levels: {file} ({from}): {e}; using the built-in layout"),
                }
                return;
            }
        }
        match parse(text) {
            Ok((lv, warnings)) => {
                for w in &warnings {
                    eprintln!("levels: {file} ({from}): {w}");
                }
                let want = format!("{}_{}", if lv.kind == Kind::Cave { "cave" } else { "lair" }, lv.number);
                if want != stem {
                    eprintln!("levels: {file} ({from}): kind/number say {want}.json, ignoring the file");
                    return;
                }
                self.by_name.insert(stem, lv);
            }
            Err(e) => eprintln!("levels: {file} ({from}): {e}; using the built-in layout"),
        }
    }
    pub fn cave(&self, k: usize) -> Option<&Level> {
        self.by_name.get(&format!("cave_{k}"))
    }
    pub fn keep(&self, n: usize) -> Option<&crate::keepdef::Keep> {
        self.keeps.get(&n)
    }
    pub fn lair(&self, n: usize) -> Option<&Level> {
        self.by_name.get(&format!("lair_{n}"))
    }
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
pub fn loot(s: &str) -> Option<Loot> {
    let (k, n) = match s.split_once(':') {
        Some((k, n)) => (k, n.trim().parse::<i32>().ok().filter(|&n| n > 0)?),
        None => (s, 1),
    };
    Some(match k {
        "key" => Loot::Key,
        "gold" => Loot::Gold(n.min(9999)),
        "bombs" => Loot::Bombs(n.min(99)),
        "elixir" | "elixirs" => Loot::Elixirs(n.min(9)),
        "heart" => Loot::Heart,
        "mana" => Loot::Mana,
        "potion" => Loot::Potion,
        "page" => Loot::Page,
        "small_key" => Loot::SmallKey,
        "map" => Loot::Map,
        "finder" => Loot::Finder,
        "big_key" => Loot::BigKey,
        _ => return None,
    })
}
pub fn tile(c: char) -> Option<u8> {
    Some(match c {
        '.' => T_FLOOR,
        '#' => T_WALL,
        '~' => T_WATER,
        'i' => T_ICE,
        'c' => T_CRACK,
        'o' => T_PLATE,
        'D' => T_DECOR,
        _ => return None,
    })
}
const PUZZLES: [&str; 7] = ["none", "combat", "torches", "plates", "icebridge", "hidden", "freeze_plate"];
const CAVE_SLOTS: [&str; 3] = ["mouth", "challenge", "treasure"];
const LAIR_SLOTS: [&str; 7] = ["entry", "hub", "west", "east", "stairs", "feast", "pantry"];

/// Parse a level file. Returns the level plus warnings for parts that were skipped.
pub fn parse(text: &str) -> Result<(Level, Vec<String>), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON ({e})"))?;
    let o = v.as_object().ok_or("the file must be a JSON object")?;
    let format = o.get("format").and_then(Value::as_i64).ok_or("missing \"format\"")?;
    if format != 1 {
        return Err(format!("format {format} is not supported (expected 1)"));
    }
    let kind = match o.get("kind").and_then(Value::as_str) {
        Some("cave") => Kind::Cave,
        Some("lair") => Kind::Lair,
        _ => return Err("\"kind\" must be \"cave\" or \"lair\"".into()),
    };
    let max = if kind == Kind::Cave { CAVES } else { 6 };
    let number = o.get("number").and_then(Value::as_u64).ok_or("missing \"number\"")? as usize;
    if !(1..=max).contains(&number) {
        return Err(format!("\"number\" must be 1-{max}"));
    }
    let mut warn = vec![];
    let name = o.get("name").and_then(Value::as_str).map(|s| s.to_uppercase()).filter(|s| !s.trim().is_empty());
    let theme = match o.get("theme") {
        None | Some(Value::Null) => None,
        Some(t) => match t.as_u64() {
            Some(t) if t <= 11 => Some(t as usize),
            _ => {
                warn.push("\"theme\" must be 0-11; using the default".into());
                None
            }
        },
    };
    let slots: &[&str] = if kind == Kind::Cave { &CAVE_SLOTS } else { &LAIR_SLOTS };
    let mut rooms = BTreeMap::new();
    if let Some(rs) = o.get("rooms").and_then(Value::as_object) {
        for (slot, rv) in rs {
            if !slots.contains(&slot.as_str()) {
                warn.push(format!("unknown room slot \"{slot}\" (expected one of {})", slots.join(", ")));
                continue;
            }
            match parse_room(rv, kind, slot, &mut warn) {
                Ok(r) => {
                    rooms.insert(slot.clone(), r);
                }
                Err(e) => warn.push(format!("room \"{slot}\" skipped: {e}")),
            }
        }
    } else {
        return Err("missing \"rooms\" object".into());
    }
    if kind == Kind::Cave && !rooms.is_empty() && !rooms.contains_key("challenge") {
        warn.push("a cave with rooms needs a \"challenge\" room; its layout is generated".into());
    }
    Ok((Level { kind, number, name, theme, rooms }, warn))
}

fn xy(v: &Value, what: &str) -> Result<(i32, i32), String> {
    let x = v.get("x").and_then(Value::as_i64).ok_or(format!("{what} needs \"x\""))? as i32;
    let y = v.get("y").and_then(Value::as_i64).ok_or(format!("{what} needs \"y\""))? as i32;
    if !(1..ROOM_COLS as i32 - 1).contains(&x) || !(1..ROOM_ROWS as i32 - 1).contains(&y) {
        return Err(format!("{what} at {x},{y} is outside the room"));
    }
    Ok((x, y))
}

fn parse_room(v: &Value, kind: Kind, slot: &str, warn: &mut Vec<String>) -> Result<RoomDef, String> {
    let o = v.as_object().ok_or("must be an object")?;
    let mut r = RoomDef::default();
    if let Some(t) = o.get("tiles") {
        let rows = t.as_array().ok_or("\"tiles\" must be a list of strings")?;
        if rows.len() != ROOM_ROWS {
            return Err(format!("\"tiles\" needs {ROOM_ROWS} rows, found {}", rows.len()));
        }
        let mut grid = vec![];
        for (y, row) in rows.iter().enumerate() {
            let s = row.as_str().ok_or("each tile row must be a string")?;
            let chars: Vec<char> = s.chars().collect();
            if chars.len() != ROOM_COLS {
                return Err(format!("tile row {y} needs {ROOM_COLS} characters, found {}", chars.len()));
            }
            let mut line = vec![];
            for (x, c) in chars.into_iter().enumerate() {
                line.push(tile(c).ok_or(format!("unknown tile '{c}' at {x},{y}"))?);
            }
            grid.push(line);
        }
        r.tiles = Some(grid);
    }
    if let Some(p) = o.get("puzzle").and_then(Value::as_str) {
        if !PUZZLES.contains(&p) {
            return Err(format!("unknown puzzle \"{p}\""));
        }
        let ok = match (kind, slot) {
            (Kind::Cave, "challenge") => matches!(p, "none" | "combat" | "freeze_plate"),
            (Kind::Cave, _) => p == "none",
            (Kind::Lair, "west" | "east") => !matches!(p, "freeze_plate" | "none"),
            (Kind::Lair, _) => p == "none",
        };
        if ok {
            r.puzzle = Some(p.to_string());
        } else {
            warn.push(format!("room \"{slot}\": puzzle \"{p}\" isn't allowed there; ignored"));
        }
    }
    r.combat = o.get("combat").and_then(Value::as_bool).unwrap_or(false);
    for e in o.get("enemies").and_then(Value::as_array).into_iter().flatten() {
        let res = (|| -> Result<EnemyDef, String> {
            let k = e.get("kind").and_then(Value::as_str).ok_or("enemy needs \"kind\"")?;
            let kind = mob(k).ok_or(format!("unknown enemy kind \"{k}\""))?;
            let (x, y) = xy(e, "enemy")?;
            let element = match e.get("element").and_then(Value::as_str) {
                None => None,
                Some(s) => Some(element(s).ok_or(format!("unknown element \"{s}\""))?),
            };
            let hp = e.get("hp").and_then(Value::as_f64).map(|h| (h as f32).clamp(1.0, 999.0));
            let still = e.get("still").and_then(Value::as_bool).unwrap_or(false);
            Ok(EnemyDef { kind, element, x, y, hp, still })
        })();
        match res {
            Ok(d) => r.enemies.push(d),
            Err(e) => warn.push(format!("room \"{slot}\": {e}; skipped")),
        }
    }
    r.random_enemies = o.get("random_enemies").and_then(Value::as_i64).map(|n| n.clamp(0, 12) as i32);
    for ob in o.get("objects").and_then(Value::as_array).into_iter().flatten() {
        let res = (|| -> Result<ObjDef, String> {
            let t = ob.get("type").and_then(Value::as_str).ok_or("object needs \"type\"")?;
            let (x, y) = xy(ob, t)?;
            let flag = |k: &str| ob.get(k).and_then(Value::as_bool).unwrap_or(false);
            Ok(match t {
                "torch" => ObjDef::Torch { x, y, lit: flag("lit") },
                "block" => ObjDef::Block { x, y },
                "lever" => ObjDef::Lever { x, y, pulled: flag("pulled") },
                "chest" => {
                    let l = match ob.get("contents").and_then(Value::as_str) {
                        None => Loot::Default,
                        Some(s) => loot(s).ok_or(format!("unknown chest contents \"{s}\""))?,
                    };
                    ObjDef::Chest { x, y, loot: l, hidden: flag("hidden") }
                }
                "shrine" => {
                    let s = ob.get("element").and_then(Value::as_str).unwrap_or("fire");
                    ObjDef::Shrine { x, y, element: element(s).filter(|&e| e < 4).ok_or(format!("unknown shrine element \"{s}\""))? }
                }
                _ => return Err(format!("unknown object type \"{t}\"")),
            })
        })();
        match res {
            Ok(d) => r.objects.push(d),
            Err(e) => warn.push(format!("room \"{slot}\": {e}; skipped")),
        }
    }
    if let Some(tiles) = &r.tiles {
        let plates = tiles.iter().flatten().filter(|&&t| t == T_PLATE).count();
        let blocks = r.objects.iter().filter(|o| matches!(o, ObjDef::Block { .. })).count();
        match r.puzzle.as_deref() {
            Some("plates") if blocks < plates || plates == 0 => warn.push(format!("room \"{slot}\": plates puzzle has {plates} plates but {blocks} blocks")),
            Some("freeze_plate") if plates == 0 => warn.push(format!("room \"{slot}\": freeze_plate puzzle has no plates")),
            _ => {}
        }
        r.enemies.retain(|e| {
            let ok = !solid_tile(tiles[e.y as usize][e.x as usize]);
            if !ok {
                warn.push(format!("room \"{slot}\": enemy at {},{} stands in a wall; skipped", e.x, e.y));
            }
            ok
        });
    }
    if kind == Kind::Lair && slot == "west" && !r.objects.is_empty() && !r.objects.iter().any(|o| matches!(o, ObjDef::Chest { loot: Loot::Key, .. })) {
        warn.push("room \"west\" has objects but no key chest (\"contents\": \"key\"); the key can't be found".into());
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(rows: &[&str]) -> String {
        format!("[{}]", rows.iter().map(|r| format!("\"{r}\"")).collect::<Vec<_>>().join(","))
    }
    const WALLED: [&str; 13] = [
        "################",
        "#..............#",
        "#..............#",
        "#..............#",
        "#....o....o....#",
        "#..............#",
        "#..............#",
        "#..............#",
        "#..~~~~~~~~~~..#",
        "#..............#",
        "#..............#",
        "#..............#",
        "################",
    ];

    #[test]
    fn parses_a_cave_level() {
        let text = format!(
            r#"{{"format":1,"kind":"cave","number":2,"name":"Test Grotto","rooms":{{
                "challenge":{{"tiles":{},"puzzle":"freeze_plate","enemies":[{{"kind":"slime","element":"ice","x":5,"y":9,"hp":40}}],
                              "objects":[{{"type":"shrine","x":3,"y":7,"element":"ice"}}]}},
                "treasure":{{"objects":[{{"type":"chest","x":8,"y":5,"contents":"elixirs:2"}}]}}}}}}"#,
            room(&WALLED)
        );
        let (lv, warn) = parse(&text).expect("parses");
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(lv.kind, Kind::Cave);
        assert_eq!(lv.name.as_deref(), Some("TEST GROTTO"));
        let c = &lv.rooms["challenge"];
        assert_eq!(c.tiles.as_ref().unwrap()[4][5], T_PLATE);
        assert_eq!(c.tiles.as_ref().unwrap()[8][3], T_WATER);
        assert_eq!(c.enemies[0].element, Some(1));
        assert_eq!(lv.rooms["treasure"].objects[0], ObjDef::Chest { x: 8, y: 5, loot: Loot::Elixirs(2), hidden: false });
    }

    #[test]
    fn bad_levels_are_rejected_or_trimmed_not_fatal() {
        assert!(parse("not json").is_err());
        assert!(parse(r#"{"format":2,"kind":"cave","number":1,"rooms":{}}"#).is_err());
        assert!(parse(r#"{"format":1,"kind":"cave","number":9,"rooms":{}}"#).is_err());
        // A broken room and a bad enemy are skipped with warnings; the rest still loads.
        let text = r##"{"format":1,"kind":"lair","number":1,"rooms":{
            "hub":{"tiles":["#"]},
            "entry":{"enemies":[{"kind":"dragon","x":3,"y":3},{"kind":"bat","x":4,"y":4}]}}}"##;
        let (lv, warn) = parse(text).expect("top level is fine");
        assert!(!lv.rooms.contains_key("hub"));
        assert_eq!(lv.rooms["entry"].enemies.len(), 1);
        assert_eq!(warn.len(), 2, "{warn:?}");
    }

    #[test]
    fn every_built_in_level_loads_cleanly() {
        for (file, text) in crate::levels_gen::LEVELS {
            let v: Value = serde_json::from_str(text).unwrap_or_else(|e| panic!("{file}: {e}"));
            if v.get("format").and_then(Value::as_i64) == Some(2) {
                let (k, warn) = crate::keepdef::parse_keep(&v).unwrap_or_else(|e| panic!("{file}: {e}"));
                assert!(warn.is_empty(), "{file}: {warn:?}");
                let problems = crate::keepdef::check_solvable(&k);
                assert!(problems.is_empty(), "{file}: {problems:?}");
                continue;
            }
            let (_, warn) = parse(text).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert!(warn.is_empty(), "{file}: {warn:?}");
        }
    }
}
