//! Format 2 lairs ("keeps"): hand-designed dungeons with free room layouts, small keys and
//! locked doors, the big key and big chest (holding the lair's relic), the map and finder,
//! shutter rooms, floor and crystal switches with coloured barriers, element crystals,
//! sliding ice blocks, pots and crates, lore tablets, props, dark rooms, pits and lava.
//!
//! Progress lives in `SaveData::keeps[n]` as tokens:
//!   `v<room>` visited, `c<room>.<obj>` chest opened, `d<room>.<door>` door unlocked,
//!   `s<room>` room solved, `f:<flag>` flag set, `blue` crystal switches toggled,
//!   `key:<n>` small keys held, `map`, `finder`, `bigkey`.
use super::dungeon::*;
use super::*;
use crate::keepdef::{KObj, Keep, Lock, Relic, Shutter};
use crate::levels::Loot;

/// Live state of a keep beside the generic dungeon data.
pub(super) struct KeepRt {
    pub def: Keep,
    /// Texts of lore tablets and names of props (indexed by Obj::tag).
    pub texts: Vec<String>,
    /// Flags set by levers and floor switches (indexed by Obj::tag).
    pub flags: Vec<String>,
    /// The tablet last read (so it isn't shown every frame).
    pub reading: Option<usize>,
    /// Next crystal expected in an ordered sequence.
    pub next_crystal: u8,
    /// Frames pushing against a boulder (Titan Gloves lift it).
    pub lift_t: i32,
}

fn door_tile(lock: &Lock, opened: bool, flag_set: bool) -> u8 {
    match lock {
        Lock::Small if !opened => T_LOCK,
        Lock::Big if !opened => T_BIGLOCK,
        Lock::Bomb if !opened => T_CRACK,
        Lock::Flag(_) if !flag_set => T_SEAL,
        _ => T_FLOOR,
    }
}

pub(super) fn build_keep(k: &Keep, themes: &[Theme], toks: &std::collections::BTreeSet<String>) -> Dungeon {
    let n = k.number;
    let theme = k.theme.filter(|&t| t < themes.len()).unwrap_or(dungeon_theme(n));
    let has_flag = |f: &str| toks.contains(&format!("f:{f}"));
    let mut texts: Vec<String> = vec![];
    let mut flags: Vec<String> = vec![];
    let flag_idx = |f: &str, flags: &mut Vec<String>| -> usize {
        match flags.iter().position(|x| x == f) {
            Some(i) => i,
            None => {
                flags.push(f.to_string());
                flags.len() - 1
            }
        }
    };
    let mut rooms = vec![];
    let mut objs = vec![];
    for (i, kr) in k.rooms.iter().enumerate() {
        let mut r = Room::sized(i, kr.x, kr.y, 9300 + n as u32 * 100 + i as u32, kr.w, kr.h);
        r.tiles = kr.tiles.clone();
        r.theme = (n - 1).min(3);
        r.visited = toks.contains(&format!("v{i}"));
        let solved = toks.contains(&format!("s{i}"));
        for (di, d) in kr.doors.iter().enumerate() {
            if let Some(to) = d.to {
                r.links.push(Link { d: d.side, seg: d.seg, to });
            }
            r.doors[d.side] = true;
            let flag = matches!(&d.lock, Lock::Flag(f) if has_flag(f));
            let t = door_tile(&d.lock, toks.contains(&format!("d{i}.{di}")), flag);
            r.set_gap_seg(d.side, d.seg, t);
        }
        for (f, from, to) in &kr.flag_tiles {
            if has_flag(f) {
                for t in r.tiles.iter_mut().flatten() {
                    if *t == *from {
                        *t = *to;
                    }
                }
            }
        }
        if kr.boss_stairs {
            for dy in 0..2 {
                for dx in 0..2 {
                    r.tiles[(STAIRS_R + dy) as usize][(STAIRS_C + dx) as usize] = T_STAIRS;
                }
            }
        }
        let mut plates: Vec<(i32, i32)> = vec![];
        for (y, row) in r.tiles.iter().enumerate() {
            for (x, &t) in row.iter().enumerate() {
                if t == T_PLATE {
                    plates.push((x as i32, y as i32));
                }
            }
        }
        let mut o = vec![];
        for (oi, ko) in kr.objects.iter().enumerate() {
            let opened = toks.contains(&format!("c{i}.{oi}"));
            let ob = match ko {
                KObj::Torch { x, y, lit } => {
                    let mut t = Obj::new(OK::Torch, *x, *y);
                    t.on = *lit || (solved && kr.shutter == Shutter::Torches);
                    t
                }
                KObj::Block { x, y } | KObj::IceBlock { x, y } => {
                    let k = if matches!(ko, KObj::Block { .. }) { OK::Block } else { OK::IceBlock };
                    let mut b = Obj::new(k, *x, *y);
                    if solved && kr.shutter == Shutter::Plates && !plates.is_empty() {
                        (b.c, b.r) = plates.remove(0);
                        b.on = true;
                    }
                    b
                }
                KObj::Lever { x, y, sets } => {
                    let mut l = Obj::new(OK::Lever, *x, *y);
                    if let Some(f) = sets {
                        l.tag = flag_idx(f, &mut flags);
                        l.on = has_flag(f);
                    }
                    l
                }
                KObj::Chest { x, y, loot, hidden } => {
                    let mut c = Obj::new(OK::Chest, *x, *y);
                    c.loot = *loot;
                    c.on = opened;
                    c.visible = !*hidden || solved;
                    c
                }
                KObj::BigChest { x, y, .. } => {
                    let mut c = Obj::new(OK::BigChest, *x, *y);
                    c.on = opened;
                    c
                }
                KObj::Shrine { x, y, element } => Obj::new(OK::Shrine(Elem::from_idx(*element)), *x, *y),
                KObj::Pot { x, y } => Obj::new(OK::Pot, *x, *y),
                KObj::Crate { x, y } => Obj::new(OK::Crate, *x, *y),
                KObj::FloorSwitch { x, y, sets } => {
                    let mut s = Obj::new(OK::FloorSwitch, *x, *y);
                    if let Some(f) = sets {
                        s.tag = flag_idx(f, &mut flags);
                        s.on = has_flag(f);
                    }
                    s.on |= solved && kr.shutter == Shutter::Switch;
                    s
                }
                KObj::CrystalSwitch { x, y } => Obj::new(OK::CrystalSwitch, *x, *y),
                KObj::Crystal { x, y, element, order } => {
                    let mut c = Obj::new(OK::Crystal(Elem::from_idx(*element)), *x, *y);
                    c.tag = *order as usize;
                    c.on = solved;
                    c
                }
                KObj::Post { x, y } => Obj::new(OK::Post, *x, *y),
                KObj::Boulder { x, y } => Obj::new(OK::Boulder, *x, *y),
                KObj::Tablet { x, y, text } => {
                    let mut t = Obj::new(OK::Tablet, *x, *y);
                    texts.push(text.clone());
                    t.tag = texts.len() - 1;
                    t
                }
                KObj::Prop { x, y, name, solid } => {
                    let mut p = Obj::new(OK::Prop, *x, *y);
                    texts.push(name.clone());
                    p.tag = texts.len() - 1;
                    p.hard = *solid;
                    p
                }
            };
            o.push(ob);
        }
        render(&mut r, &themes[theme]);
        rooms.push(r);
        objs.push(o);
    }
    let len = rooms.len();
    let spawns = k.rooms.iter().map(|r| r.enemies.clone()).collect();
    let random = k.rooms.iter().map(|r| r.random_enemies.or(if r.enemies.is_empty() { None } else { Some(0) })).collect();
    Dungeon {
        n, rooms, objs, puz: vec![None; len], has: vec![true; len], larder: vec![vec![]; len], hub_combat: false, cur: k.entry,
        dirty: false, sealed: false, push_t: 0, crack_hits: vec![], seen: vec![false; len], warned: false, cave: false, theme,
        spawns, random, chal: 0, treasure: 0,
        keep: Some(Box::new(KeepRt { def: k.clone(), texts, flags, reading: None, next_crystal: 1, lift_t: 0 })),
    }
}

impl Game {
    pub(super) fn in_keep(&self) -> bool {
        self.dungeon.as_ref().map_or(false, |d| d.keep.is_some())
    }
    pub(super) fn has_relic(&self, r: Relic) -> bool {
        self.s.relics & (1 << r.idx()) != 0
    }
    fn keep_n(&self) -> usize {
        self.dungeon.as_ref().map_or(0, |d| d.n)
    }
    pub(super) fn ktok(&self, t: &str) -> bool {
        self.s.keeps.get(&self.keep_n()).map_or(false, |s| s.contains(t))
    }
    pub(super) fn set_ktok(&mut self, t: &str) {
        let n = self.keep_n();
        self.s.keeps.entry(n).or_default().insert(t.to_string());
    }
    /// Small keys held in the current keep.
    pub(super) fn kkeys(&self) -> i32 {
        self.s.keeps.get(&self.keep_n()).and_then(|s| s.iter().find_map(|t| t.strip_prefix("key:")?.parse().ok())).unwrap_or(0)
    }
    fn set_kkeys(&mut self, k: i32) {
        let n = self.keep_n();
        let set = self.s.keeps.entry(n).or_default();
        set.retain(|t| !t.starts_with("key:"));
        if k > 0 {
            set.insert(format!("key:{k}"));
        }
    }
    pub(super) fn crystals_blue(&self) -> bool {
        self.ktok("blue")
    }

    // ------------------------------------------------------------ room entry
    pub(super) fn enter_keep_room(&mut self) {
        let Some(d) = self.dungeon.as_mut() else { return };
        let cur = d.cur;
        let first = !d.seen[cur];
        d.seen[cur] = true;
        d.rooms[cur].visited = true;
        d.warned = false;
        let kr = d.keep.as_ref().unwrap().def.rooms[cur].clone();
        if let Some(k) = d.keep.as_mut() {
            k.reading = None;
            k.lift_t = 0;
            k.next_crystal = 1;
        }
        let solved = self.ktok(&format!("s{cur}"));
        self.set_ktok(&format!("v{cur}"));
        let Some(d) = self.dungeon.as_mut() else { return };
        // Unsolved puzzle rooms keep their shutter doors closed; fights seal every door.
        if !solved && kr.shutter != Shutter::None {
            for (di, door) in kr.doors.iter().enumerate() {
                if door.lock == Lock::Shutter || kr.shutter == Shutter::Combat && door.to.is_some() {
                    let _ = di;
                    d.rooms[cur].set_gap_seg(door.side, door.seg, T_SEAL);
                }
            }
            d.sealed = kr.shutter == Shutter::Combat;
            d.dirty = true;
        }
        // Unsolved ordered crystals and floor switches start fresh.
        if !solved {
            for o in d.objs[cur].iter_mut() {
                if matches!(o.k, OK::Crystal(_)) {
                    o.on = false;
                }
            }
        }
        self.dungeon_flush();
        let combat = !solved && kr.shutter == Shutter::Combat;
        if !self.spawn_listed(cur, combat) {
            let th = self.dungeon.as_ref().map_or(0, |d| d.rooms[cur].theme);
            self.spawn_pack(if combat { 5 } else { 2 }, th, false);
        }
        if combat {
            self.show_msg("THE DOORS SLAM SHUT! DEFEAT EVERY MONSTER.");
            self.sfx(Sfx::Rumble);
            self.shake = 10;
        } else if first && !kr.name.is_empty() {
            self.float(kr.name.clone(), 128.0 - kr.name.len() as f32 * 4.0, HUDF + 24.0, rgb(0xfcbc3c));
        }
        // The finder chimes when this room still holds treasure.
        if self.ktok("finder") && self.room_has_treasure(cur) {
            self.sfx(Sfx::Coin);
            self.float("TREASURE NEARBY", 64.0, HUDF + 36.0, rgb(0xa4e4fc));
        }
        if kr.dark && !self.has_relic(Relic::Lantern) && first {
            self.show_msg("IT IS PITCH DARK HERE. A LANTERN WOULD HELP.");
        }
    }
    pub(super) fn room_has_treasure(&self, room: usize) -> bool {
        self.dungeon.as_ref().map_or(false, |d| d.objs[room].iter().any(|o| matches!(o.k, OK::Chest | OK::BigChest) && !o.on))
    }

    // ------------------------------------------------------------ per frame (player)
    pub(super) fn keep_player(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        self.keep_push(ix, iy, blocked);
        self.push_frozen_any(ix, iy, blocked);
        self.keep_doors(ix, iy, blocked);
        if self.mode != Mode::Play {
            return;
        }
        let (px, py) = (self.pl.x, self.pl.y);
        let cur = self.dungeon.as_ref().map_or(0, |d| d.cur);
        // Boss staircase.
        let stairs = self.dungeon.as_ref().map_or(false, |d| d.keep.as_ref().unwrap().def.rooms[cur].boss_stairs);
        if stairs {
            let (sx, sy) = tile_center(STAIRS_C, STAIRS_R);
            let (cx, cy) = (sx + 8.0, sy + 8.0);
            if (px - cx).abs() < 18.0 && py < cy + 20.0 && py > cy - 20.0 {
                if self.s.cleared[self.keep_n().min(6)] {
                    if !self.dungeon.as_ref().map_or(true, |d| d.warned) {
                        if let Some(d) = self.dungeon.as_mut() {
                            d.warned = true;
                        }
                        self.show_msg("THE LAIR BELOW IS SILENT. ITS GUARDIAN HAS FALLEN.");
                    }
                } else {
                    self.begin_descend();
                    return;
                }
            }
        }
        // Touching objects.
        let touched: Vec<(usize, OK, bool)> = self.dungeon.as_ref().map_or(vec![], |d| {
            d.objs[d.cur]
                .iter()
                .enumerate()
                .filter(|(_, o)| o.visible)
                .filter(|(_, o)| {
                    let (ox, oy) = tile_center(o.c, o.r);
                    let reach = if o.solid() { 18.0 } else { 12.0 };
                    (ox - px).abs() < reach && (oy - py).abs() < reach
                })
                .map(|(i, o)| (i, o.k, o.on))
                .collect()
        });
        let mut reading = None;
        for (i, k, on) in touched {
            match k {
                OK::Chest if !on => self.open_keep_chest(i),
                OK::BigChest if !on => self.open_big_chest(i),
                OK::Shrine(el) if self.el() != el => self.set_element(el),
                OK::Lever if !on => {
                    self.keep_set_obj_flag(i);
                    self.sfx(Sfx::Click);
                }
                OK::FloorSwitch if !on => {
                    self.press_switch(i);
                }
                OK::Tablet => reading = Some(i),
                _ => {}
            }
        }
        let was = self.dungeon.as_ref().and_then(|d| d.keep.as_ref().unwrap().reading);
        if reading.is_some() && reading != was {
            let text = self.dungeon.as_ref().and_then(|d| {
                let i = reading.unwrap();
                d.keep.as_ref().unwrap().texts.get(d.objs[d.cur][i].tag).cloned()
            });
            if let Some(t) = text {
                self.show_msg(format!("THE TABLET READS: {t}"));
                if let Some(m) = self.msg.as_mut() {
                    m.1 = 300;
                }
            }
        }
        if let Some(k) = self.dungeon.as_mut().and_then(|d| d.keep.as_mut()) {
            k.reading = reading;
        }
    }

    fn keep_set_obj_flag(&mut self, i: usize) {
        let (flag, name) = {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
            let t = d.objs[cur][i].tag;
            (t, d.keep.as_ref().unwrap().flags.get(t).cloned())
        };
        let _ = flag;
        if let Some(f) = name {
            self.set_flag(&f);
        }
    }
    /// Set a lair flag: open flag doors and change flag tiles in every room.
    pub(super) fn set_flag(&mut self, f: &str) {
        if self.ktok(&format!("f:{f}")) {
            return;
        }
        self.set_ktok(&format!("f:{f}"));
        let themes = &self.themes;
        let Some(d) = self.dungeon.as_mut() else { return };
        let theme = d.theme;
        let defs = d.keep.as_ref().unwrap().def.rooms.clone();
        for (i, kr) in defs.iter().enumerate() {
            let mut changed = false;
            for door in &kr.doors {
                if matches!(&door.lock, Lock::Flag(g) if g == f) {
                    d.rooms[i].set_gap_seg(door.side, door.seg, T_FLOOR);
                    changed = true;
                }
            }
            for (g, from, to) in &kr.flag_tiles {
                if g == f {
                    for t in d.rooms[i].tiles.iter_mut().flatten() {
                        if *t == *from {
                            *t = *to;
                            changed = true;
                        }
                    }
                }
            }
            if changed {
                render(&mut d.rooms[i], &themes[theme]);
            }
        }
        self.show_msg("SOMEWHERE IN THE LAIR, SOMETHING SHIFTS...");
        self.sfx(Sfx::Rumble);
        self.shake = 8;
        self.save();
    }
    fn press_switch(&mut self, i: usize) {
        let has_flag = {
            let d = self.dungeon.as_ref().unwrap();
            d.objs[d.cur][i].tag != usize::MAX
        };
        {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
        }
        self.sfx(Sfx::Click);
        if has_flag {
            self.keep_set_obj_flag(i);
        }
    }

    fn open_keep_chest(&mut self, i: usize) {
        let (x, y, loot, cur) = {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
            let o = &d.objs[cur][i];
            let (x, y) = tile_center(o.c, o.r);
            (x, y, o.loot, cur)
        };
        self.set_ktok(&format!("c{cur}.{i}"));
        let msg = match loot {
            Loot::SmallKey => {
                let k = self.kkeys() + 1;
                self.set_kkeys(k);
                "A SMALL KEY! IT OPENS ONE LOCKED DOOR IN THIS LAIR.".to_string()
            }
            Loot::Map => {
                self.set_ktok("map");
                "THE LAIR MAP! PAUSE TO SEE EVERY ROOM.".to_string()
            }
            Loot::Finder => {
                self.set_ktok("finder");
                "THE FINDER! IT MARKS HIDDEN TREASURE AND CHIMES WHEN SOME IS NEAR.".to_string()
            }
            Loot::BigKey => {
                self.set_ktok("bigkey");
                "THE BIG KEY! IT OPENS THE GREAT CHEST AND THE GUARDIAN'S DOOR.".to_string()
            }
            Loot::Default => {
                self.s.gold = (self.s.gold + 20).min(9999);
                "20 GOLD!".to_string()
            }
            other => self.give_loot(other),
        };
        self.float("TREASURE!", x - 36.0, y - 22.0, WHITE);
        self.show_msg(msg);
        self.part(x, y, 0.0, 0.0, 16, rgb(0xfcbc3c), 1, PK::Glow(14.0));
        self.sfx(Sfx::Chest);
        self.save();
    }
    fn open_big_chest(&mut self, i: usize) {
        if !self.ktok("bigkey") {
            if !self.dungeon.as_ref().map_or(true, |d| d.warned) {
                if let Some(d) = self.dungeon.as_mut() {
                    d.warned = true;
                }
                self.show_msg("THIS GREAT CHEST IS LOCKED. FIND THE BIG KEY.");
                self.sfx(Sfx::Deny);
            }
            return;
        }
        let (x, y, cur, relic) = {
            let d = self.dungeon.as_mut().unwrap();
            let cur = d.cur;
            d.objs[cur][i].on = true;
            let (x, y) = tile_center(d.objs[cur][i].c, d.objs[cur][i].r);
            let k = d.keep.as_ref().unwrap();
            let relic = k.def.relic.or_else(|| {
                k.def.rooms[cur].objects.iter().find_map(|o| if let KObj::BigChest { relic, .. } = o { *relic } else { None })
            });
            (x, y, cur, relic.unwrap_or(crate::keepdef::RELICS[(d.n - 1).min(4)]))
        };
        self.set_ktok(&format!("c{cur}.{i}"));
        self.s.relics |= 1 << relic.idx();
        let how = match relic {
            Relic::Whip => "SELECT IT IN YOUR BAG AND PRESS Y TO LASH IT: SWING TO POSTS ACROSS GAPS, CUT THORNS, FLIP SWITCHES.",
            Relic::Lantern => "DARK ROOMS NOW GLOW AROUND YOU, AND HIDDEN BRIDGES APPEAR.",
            Relic::Gloves => "PUSH AGAINST A BOULDER TO LIFT IT OUT OF THE WAY.",
            Relic::Boots => "YOU CAN NOW WALK ACROSS LAVA.",
            Relic::Cloak => "SELECT IT IN YOUR BAG AND PRESS Y TO FLOAT OVER PITS FOR A MOMENT.",
        };
        self.show_msg(format!("YOU FOUND THE {}! {}", relic.name(), how));
        if let Some(m) = self.msg.as_mut() {
            m.1 = 300;
        }
        self.held_up = Some((relic, 90));
        self.part(x, y, 0.0, 0.0, 24, rgb(0xfce040), 1, PK::Glow(22.0));
        self.sfx(Sfx::Fanfare);
        self.flash = 8;
        self.save();
    }

    /// Small-key and big-key doors open when walked into with the key.
    fn keep_doors(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        let axis = if ix != 0.0 && iy == 0.0 && blocked.0 {
            (ix.signum(), 0.0)
        } else if iy != 0.0 && ix == 0.0 && blocked.1 {
            (0.0, iy.signum())
        } else {
            return;
        };
        let probe = (self.pl.x + axis.0 * (self.pl.w / 2.0 + 3.0), self.pl.y + axis.1 * (self.pl.h / 2.0 + 3.0));
        let (c, r) = tile_of(probe.0, probe.1);
        let t = self.tile_at(c, r);
        if !matches!(t, T_LOCK | T_BIGLOCK) {
            return;
        }
        let Some(d) = self.dungeon.as_ref() else { return };
        let cur = d.cur;
        let room = &d.rooms[cur];
        let kr = &d.keep.as_ref().unwrap().def.rooms[cur];
        // Which door's gap holds this tile?
        let (cols, rows) = (room.cols() as i32, room.rows() as i32);
        let di = kr.doors.iter().position(|door| {
            let (ox, oy) = (door.seg as i32 * RC as i32, door.seg as i32 * RR as i32);
            match door.side {
                0 => r == 0 && (6..=9).contains(&(c - ox)),
                1 => r == rows - 1 && (6..=9).contains(&(c - ox)),
                2 => c == cols - 1 && (5..=7).contains(&(r - oy)),
                _ => c == 0 && (5..=7).contains(&(r - oy)),
            }
        });
        let Some(di) = di else { return };
        let big = t == T_BIGLOCK;
        let ok = if big { self.ktok("bigkey") } else { self.kkeys() > 0 };
        if !ok {
            if !d.warned {
                if let Some(d) = self.dungeon.as_mut() {
                    d.warned = true;
                }
                self.show_msg(if big { "A GREAT LOCK. ONLY THE BIG KEY WILL OPEN IT." } else { "A LOCKED DOOR. YOU NEED A SMALL KEY." });
                self.sfx(Sfx::Deny);
            }
            return;
        }
        if !big {
            let k = self.kkeys() - 1;
            self.set_kkeys(k);
        }
        self.unlock_door(cur, di);
        self.show_msg(if big { "THE BIG KEY TURNS IN THE GREAT LOCK!" } else { "THE KEY TURNS. THE DOOR SWINGS OPEN!" });
        self.sfx(Sfx::Unlock);
        let (x, y) = tile_center(c, r);
        self.part(x, y, 0.0, 0.0, 14, rgb(0xfcbc3c), 1, PK::Glow(16.0));
        self.save();
    }
    /// Open a door for good on both sides.
    fn unlock_door(&mut self, room: usize, di: usize) {
        self.set_ktok(&format!("d{room}.{di}"));
        let Some(d) = self.dungeon.as_ref() else { return };
        let door = d.keep.as_ref().unwrap().def.rooms[room].doors[di].clone();
        let mut back = None;
        if let Some(to) = door.to {
            let bside = [1, 0, 3, 2][door.side];
            let kr = &d.keep.as_ref().unwrap().def.rooms[to];
            if let Some(bi) = kr.doors.iter().position(|bd| bd.side == bside && bd.to == Some(room)) {
                back = Some((to, bi, kr.doors[bi].side, kr.doors[bi].seg));
            }
        }
        if let Some((to, bi, _, _)) = back {
            self.set_ktok(&format!("d{to}.{bi}"));
        }
        let themes = &self.themes;
        let d = self.dungeon.as_mut().unwrap();
        let theme = d.theme;
        d.rooms[room].set_gap_seg(door.side, door.seg, T_FLOOR);
        render(&mut d.rooms[room], &themes[theme]);
        if let Some((to, _, side, seg)) = back {
            d.rooms[to].set_gap_seg(side, seg, T_FLOOR);
            render(&mut d.rooms[to], &themes[theme]);
        }
    }

    /// Push blocks one tile; ice blocks slide until they hit something.
    fn keep_push(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
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
                if let Some(k) = d.keep.as_mut() {
                    k.lift_t = 0;
                }
            }
            return;
        };
        let probe = (self.pl.x + dx as f32 * (self.pl.w / 2.0 + 3.0), self.pl.y + dy as f32 * (self.pl.h / 2.0 + 3.0));
        let (c, r) = tile_of(probe.0, probe.1);
        let Some(d) = self.dungeon.as_ref() else { return };
        let cur = d.cur;
        let Some(bi) = d.objs[cur].iter().position(|o| o.visible && o.c == c && o.r == r && o.slide == 0 && matches!(o.k, OK::Block | OK::IceBlock | OK::Boulder)) else {
            return;
        };
        let kind = d.objs[cur][bi].k;
        if kind == OK::Boulder {
            // Titan Gloves: push against a boulder for a moment to heave it away.
            if !self.has_relic(Relic::Gloves) {
                if !d.warned {
                    if let Some(d) = self.dungeon.as_mut() {
                        d.warned = true;
                    }
                    self.show_msg("THIS BOULDER IS FAR TOO HEAVY TO MOVE BY HAND.");
                }
                return;
            }
            let k = self.dungeon.as_mut().unwrap().keep.as_mut().unwrap();
            k.lift_t += 1;
            if k.lift_t >= 16 {
                k.lift_t = 0;
                let d = self.dungeon.as_mut().unwrap();
                d.objs[cur][bi].visible = false;
                let (x, y) = tile_center(c, r);
                for _ in 0..12 {
                    let (vx, vy) = (self.rng.range(-1.6, 1.6), self.rng.range(-2.4, -0.6));
                    self.part(x, y, vx, vy, 26, rgb(0x8c8c98), 3, PK::Shard);
                }
                self.sfx(Sfx::Rumble);
                self.shake = 6;
                self.float("HEAVE!", x - 20.0, y - 18.0, WHITE);
            }
            return;
        }
        if d.objs[cur][bi].on {
            return;
        }
        let ice = kind == OK::IceBlock;
        let free = |g: &Game, tc: i32, tr: i32| -> bool {
            let d = g.dungeon.as_ref().unwrap();
            let room = &d.rooms[d.cur];
            tc > 0
                && tr > 0
                && tc < room.cols() as i32 - 1
                && tr < room.rows() as i32 - 1
                && matches!(g.tile_at(tc, tr), T_FLOOR | T_PLATE | T_ICE)
                && !d.objs[d.cur].iter().any(|o| o.visible && o.solid() && o.c == tc && o.r == tr)
                && !g.enemies.iter().any(|e| !e.dead && tile_of(e.x, e.y) == (tc, tr))
        };
        if !free(self, c + dx, r + dy) {
            return;
        }
        let d = self.dungeon.as_mut().unwrap();
        d.push_t += 1;
        if d.push_t < 14 {
            return;
        }
        d.push_t = 0;
        // Ice slides all the way.
        let (mut tc, mut tr) = (c + dx, r + dy);
        if ice {
            while free(self, tc + dx, tr + dy) && self.tile_at(tc, tr) != T_PLATE {
                tc += dx;
                tr += dy;
            }
        }
        let d = self.dungeon.as_mut().unwrap();
        let b = &mut d.objs[cur][bi];
        let steps = (tc - c).abs() + (tr - r).abs();
        b.c = tc;
        b.r = tr;
        b.slide = 8 * steps;
        b.sdx = dx;
        b.sdy = dy;
        self.sfx(Sfx::Push);
    }
    /// Walking into a frozen monster shoves it (caves and keeps).
    fn push_frozen_any(&mut self, ix: f32, iy: f32, blocked: (bool, bool)) {
        let _ = blocked;
        let axis = if ix != 0.0 && iy == 0.0 { Some((ix.signum(), 0.0)) } else if iy != 0.0 && ix == 0.0 { Some((0.0, iy.signum())) } else { None };
        let Some((dx, dy)) = axis else { return };
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
        let e = &mut self.enemies[i];
        e.kbx = dx * 4.0;
        e.kby = dy * 4.0;
        self.sfx(Sfx::Push);
    }

    // ------------------------------------------------------------ per frame (room)
    pub(super) fn keep_update(&mut self) {
        let Some(d) = self.dungeon.as_mut() else { return };
        let cur = d.cur;
        for o in d.objs[cur].iter_mut() {
            if o.slide > 0 {
                o.slide -= 1;
            }
        }
        let kr = d.keep.as_ref().unwrap().def.rooms[cur].clone();
        // Blocks resting on floor switches press them.
        let mut press = vec![];
        for (i, o) in d.objs[cur].iter().enumerate() {
            if o.k == OK::FloorSwitch && !o.on && d.objs[cur].iter().any(|b| matches!(b.k, OK::Block | OK::IceBlock) && b.slide == 0 && b.c == o.c && b.r == o.r) {
                press.push(i);
            }
        }
        for i in press {
            self.press_switch(i);
        }
        let solved = self.ktok(&format!("s{cur}"));
        if solved || kr.shutter == Shutter::None {
            return;
        }
        let d = self.dungeon.as_ref().unwrap();
        let objs = &d.objs[cur];
        let done = match kr.shutter {
            Shutter::Combat => d.sealed && self.enemies.iter().all(|e| e.dead),
            Shutter::Torches => objs.iter().filter(|o| o.k == OK::Torch).all(|o| o.on),
            Shutter::Switch => objs.iter().filter(|o| o.k == OK::FloorSwitch).all(|o| o.on),
            Shutter::Crystals => objs.iter().filter(|o| matches!(o.k, OK::Crystal(_))).all(|o| o.on),
            Shutter::Plates => {
                let room = &d.rooms[cur];
                let mut all = true;
                for (y, row) in room.tiles.iter().enumerate() {
                    for (x, &t) in row.iter().enumerate() {
                        if t != T_PLATE {
                            continue;
                        }
                        let (c, r) = (x as i32, y as i32);
                        let covered = objs.iter().any(|o| matches!(o.k, OK::Block | OK::IceBlock) && o.slide == 0 && o.c == c && o.r == r)
                            || self.enemies.iter().any(|e| !e.dead && e.st.frozen() && tile_of(e.x, e.y) == (c, r));
                        all &= covered;
                    }
                }
                all
            }
            Shutter::None => false,
        };
        if !done {
            return;
        }
        self.set_ktok(&format!("s{cur}"));
        let d = self.dungeon.as_mut().unwrap();
        d.sealed = false;
        for door in &kr.doors {
            if door.lock == Lock::Shutter || (kr.shutter == Shutter::Combat && door.to.is_some()) {
                let opened = self.s.keeps.get(&d.n).map_or(false, |_| true);
                let _ = opened;
                let t = match &door.lock {
                    Lock::Shutter | Lock::None => T_FLOOR,
                    other => {
                        let di = kr.doors.iter().position(|x| x == door).unwrap_or(0);
                        let open = self.s.keeps.get(&d.n).map_or(false, |s| s.contains(&format!("d{cur}.{di}")));
                        let flag = matches!(other, Lock::Flag(f) if self.s.keeps.get(&d.n).map_or(false, |s| s.contains(&format!("f:{f}"))));
                        door_tile(other, open, flag)
                    }
                };
                d.rooms[cur].set_gap_seg(door.side, door.seg, t);
            }
        }
        // Hidden chests appear.
        for o in d.objs[cur].iter_mut() {
            if o.k == OK::Chest && !o.visible {
                o.visible = true;
            }
        }
        d.dirty = true;
        self.dungeon_flush();
        self.show_msg(match kr.shutter {
            Shutter::Combat => "SILENCE FALLS. THE DOORS GRIND OPEN.",
            _ => "A HEAVY CLUNK... THE WAY OPENS!",
        });
        self.sfx(Sfx::Unlock);
        self.save();
    }

    /// A player bolt struck a solid tile or object in a keep. Returns true if handled.
    pub(super) fn keep_bolt_hit(&mut self, c: i32, r: i32, el: Elem) -> bool {
        let Some(d) = self.dungeon.as_ref() else { return false };
        let cur = d.cur;
        let Some(i) = d.objs[cur].iter().position(|o| o.solid() && o.c == c && o.r == r) else { return false };
        let o = d.objs[cur][i].clone();
        let (x, y) = tile_center(c, r);
        match o.k {
            OK::Pot | OK::Crate => {
                self.dungeon.as_mut().unwrap().objs[cur][i].visible = false;
                let col = if o.k == OK::Pot { rgb(0xb06c3c) } else { rgb(0x8c5c2c) };
                for _ in 0..10 {
                    let (vx, vy) = (self.rng.range(-1.6, 1.6), self.rng.range(-2.2, -0.4));
                    self.part(x, y, vx, vy, 24, col, 2, PK::Shard);
                }
                self.sfx(Sfx::Hit);
                // ALttP-style drops: often nothing, sometimes a heart, magic or coins.
                let v = self.rng.f();
                // Lairs have no feast halls, so pots also hold food.
                let kind = if v < 0.24 {
                    Some(IK::Heart)
                } else if v < 0.36 {
                    Some(IK::Potion)
                } else if v < 0.48 {
                    Some(IK::Bread)
                } else if v < 0.54 {
                    Some(IK::Meat)
                } else if v < 0.76 {
                    Some(IK::Coin)
                } else {
                    None
                };
                if let Some(k) = kind {
                    let val = if k == IK::Coin { 5 } else { 0 };
                    self.add_item(k, x, y, val, Elem::Neutral);
                }
                true
            }
            OK::Crystal(want) => {
                if o.on {
                    return true;
                }
                if el != want {
                    self.float(format!("NEEDS {}", want.name()), x - 40.0, y - 22.0, want.light());
                    return true;
                }
                let order = o.tag as u8;
                let next = self.dungeon.as_ref().unwrap().keep.as_ref().unwrap().next_crystal;
                if order > 0 && order != next {
                    // Wrong order: every crystal in the room goes dark again.
                    let d = self.dungeon.as_mut().unwrap();
                    for c in d.objs[cur].iter_mut().filter(|c| matches!(c.k, OK::Crystal(_))) {
                        c.on = false;
                    }
                    d.keep.as_mut().unwrap().next_crystal = 1;
                    self.show_msg("THE CRYSTALS DIM... THE ORDER WAS WRONG.");
                    self.sfx(Sfx::Deny);
                    return true;
                }
                let d = self.dungeon.as_mut().unwrap();
                d.objs[cur][i].on = true;
                if order > 0 {
                    d.keep.as_mut().unwrap().next_crystal = order + 1;
                }
                self.part(x, y - 6.0, 0.0, 0.0, 14, want.light(), 1, PK::Burst(want, 12.0));
                self.sfx(Sfx::Select);
                true
            }
            OK::CrystalSwitch => {
                self.toggle_crystals();
                true
            }
            _ => false,
        }
    }
    /// Crystal switches flip which coloured barriers are raised.
    pub(super) fn toggle_crystals(&mut self) {
        let n = self.keep_n();
        let set = self.s.keeps.entry(n).or_default();
        if !set.remove("blue") {
            set.insert("blue".to_string());
        }
        self.sfx(Sfx::Zap);
        self.flash = 3;
        self.unstick_player_pub();
    }

    /// A cracked wall breaks: a bombable door opens for good, any other crack just crumbles.
    pub(super) fn keep_break_crack(&mut self, c: i32, r: i32) {
        let Some(d) = self.dungeon.as_ref() else { return };
        let cur = d.cur;
        let room = &d.rooms[cur];
        let (cols, rows) = (room.cols() as i32, room.rows() as i32);
        let kr = &d.keep.as_ref().unwrap().def.rooms[cur];
        let di = kr.doors.iter().position(|door| {
            let (ox, oy) = (door.seg as i32 * RC as i32, door.seg as i32 * RR as i32);
            door.lock == Lock::Bomb
                && match door.side {
                    0 => r == 0 && (6..=9).contains(&(c - ox)),
                    1 => r == rows - 1 && (6..=9).contains(&(c - ox)),
                    2 => c == cols - 1 && (5..=7).contains(&(r - oy)),
                    _ => c == 0 && (5..=7).contains(&(r - oy)),
                }
        });
        let (x, y) = tile_center(c, r);
        match di {
            Some(di) => self.unlock_door(cur, di),
            None => {
                self.set_any_tile(c, r, T_FLOOR);
                self.dungeon_flush();
            }
        }
        for _ in 0..14 {
            let (vx, vy) = (self.rng.range(-1.8, 1.8), self.rng.range(-2.4, -0.4));
            let col = self.rng.pick(&[rgb(0x686878), rgb(0xa0a0b0), rgb(0x3c3c48)]);
            self.part(x, y, vx, vy, 28, col, 3, PK::Shard);
        }
        self.sfx(Sfx::Rumble);
        self.shake = 8;
        self.show_msg("THE WALL CRUMBLES, REVEALING A PASSAGE!");
        self.save();
    }

    // ------------------------------------------------------------ exits
    pub(super) fn keep_exit(&mut self, dir: usize) {
        let Some(d) = self.dungeon.as_ref() else { return };
        let cur = d.cur;
        let room = &d.rooms[cur];
        let along = if dir < 2 { self.pl.x } else { self.pl.y };
        if let Some(l) = room.link_at(dir, along) {
            self.begin_scroll(dir, cur, l.to, true);
            return;
        }
        let kr = &d.keep.as_ref().unwrap().def.rooms[cur];
        if kr.doors.iter().any(|dd| dd.to.is_none() && dd.side == dir) {
            let r = self.gate_room;
            let (x, y) = self.door_pos(r);
            self.go_play(r, x, y + SPAWN_Y - GATE_Y, false);
            self.fade = 24;
            return;
        }
        let (w, h) = (room.wf(), room.hf());
        self.pl.x = self.pl.x.clamp(12.0, w - 12.0);
        self.pl.y = self.pl.y.clamp(HUDF + 12.0, h - 12.0);
    }
}
