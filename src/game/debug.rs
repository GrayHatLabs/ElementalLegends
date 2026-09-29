//! Hooks for the headless `--snapshot` / `--selftest` modes. Not used in normal play.
#![allow(dead_code)]
use super::bosses::BState;
use super::dungeon::OK;
use super::*;

pub struct DebugEnemy {
    pub hp: f32,
    pub burn: i32,
    pub chill: i32,
    pub freeze: i32,
    pub dead: bool,
}
pub struct DebugBoss {
    pub name: &'static str,
    pub hp: f32,
    pub max: f32,
    pub armor: f32,
    pub phase: u8,
    pub fighting: bool,
    pub atk: String,
    pub hazards: usize,
    pub alive: bool,
    pub x: f32,
}
pub struct DebugObj {
    pub kind: String,
    pub c: i32,
    pub r: i32,
    pub on: bool,
    pub visible: bool,
}

fn ek(i: usize) -> EK {
    [EK::Slime, EK::Bat, EK::Skeleton, EK::Imp, EK::Ghost, EK::Golem, EK::Generator][i.min(6)]
}

impl Game {
    pub fn debug_no_save(&mut self) {
        self.no_save = true;
        self.has_save = false;
    }
    pub fn debug_seed(&mut self, s: u64) {
        self.rng = Rng(s | 1);
    }
    pub fn debug_force_new_game_menu(&mut self) {
        self.menu = self.menu_opts().iter().position(|o| *o == "NEW GAME").unwrap_or(0);
    }
    pub fn debug_mode(&self) -> Mode {
        self.mode
    }
    pub fn debug_new_world(&mut self, el: usize) {
        self.s = SaveData::fresh();
        self.s.el = el;
        self.setup_world();
    }
    pub fn debug_play_room(&mut self, room: usize, x: f32, y: f32) {
        self.go_play(room, x, y, false);
    }
    /// (start room, shop room)
    pub fn debug_rooms(&self) -> (usize, usize) {
        (self.start, self.shop_room)
    }
    pub fn debug_room(&self) -> usize {
        self.room
    }
    pub fn debug_room_special(&self, i: usize) -> u8 {
        self.rooms[i].special
    }
    pub fn debug_room_doors(&self, i: usize) -> [bool; 4] {
        self.rooms[i].doors
    }
    pub fn debug_room_xy(&self, i: usize) -> (usize, usize) {
        (self.rooms[i].x, self.rooms[i].y)
    }
    pub fn debug_gate_room(&self, n: usize) -> usize {
        self.rooms.iter().position(|r| r.gate == n).unwrap_or(self.start)
    }
    pub fn debug_player(&self) -> (f32, f32) {
        (self.pl.x, self.pl.y)
    }
    pub fn debug_set_player(&mut self, x: f32, y: f32, dir: u8) {
        self.pl.x = x;
        self.pl.y = y;
        self.pl.dir = dir;
        let (fx, fy) = match dir {
            b'u' => (0.0, -1.0),
            b'd' => (0.0, 1.0),
            b'l' => (-1.0, 0.0),
            _ => (1.0, 0.0),
        };
        self.pl.fx = fx;
        self.pl.fy = fy;
    }
    pub fn debug_god(&mut self) {
        self.s.max_hp = 999;
        self.s.hp = 999;
        self.s.max_mp = 999;
        self.s.mp = 999.0;
        self.s.food = 100.0;
    }
    pub fn debug_hp(&self) -> i32 {
        self.s.hp
    }
    pub fn debug_set_element(&mut self, i: usize) {
        self.s.el = i.min(3);
    }
    pub fn debug_element(&self) -> usize {
        self.s.el
    }
    pub fn debug_gold(&self) -> i32 {
        self.s.gold
    }
    pub fn debug_set_gold(&mut self, g: i32) {
        self.s.gold = g;
    }
    pub fn debug_clear_to(&mut self, n: usize) {
        for i in 1..n {
            self.s.cleared[i] = true;
        }
    }
    pub fn debug_cleared(&self, n: usize) -> bool {
        self.s.cleared[n]
    }
    /// Spawn an enemy of kind index (0 slime .. 5 golem). `still` pins it in place.
    pub fn debug_spawn(&mut self, kind: usize, x: f32, y: f32, hp: f32, still: bool) -> u32 {
        let mut e = self.make_enemy(ek(kind), x, y, 0);
        e.spawn = 0;
        e.hp = hp;
        if still {
            e.spd = 0.0;
        }
        let id = e.id;
        self.enemies.push(e);
        id
    }
    pub fn debug_enemy(&self, id: u32) -> Option<DebugEnemy> {
        self.enemies.iter().find(|e| e.id == id).map(|e| DebugEnemy {
            hp: e.hp, burn: e.st.burn, chill: e.st.chill, freeze: e.st.freeze, dead: e.dead,
        })
    }
    pub fn debug_enemy_count(&self) -> usize {
        self.enemies.iter().filter(|e| !e.dead).count()
    }
    pub fn debug_kill_enemies(&mut self) {
        let mut en = std::mem::take(&mut self.enemies);
        for e in en.iter_mut() {
            self.damage_enemy(e, 9999.0, Elem::Neutral);
        }
        en.append(&mut self.enemies);
        self.enemies = en;
    }
    pub fn debug_player_bolts(&self) -> Vec<(f32, f32)> {
        self.pb.iter().map(|b| (b.x, b.y)).collect()
    }
    pub fn debug_shards(&self) -> usize {
        self.parts.iter().filter(|p| matches!(p.kind, PK::Shard)).count()
    }
    pub fn debug_flame_parts(&self) -> usize {
        self.parts.iter().filter(|p| matches!(p.kind, PK::Ember | PK::Burst(Elem::Fire, _))).count()
    }
    /// (dungeon number, current room, progress flags, keys)
    pub fn debug_dungeon(&self) -> Option<(usize, usize, u8, i32)> {
        self.dungeon.as_ref().map(|d| (d.n, d.cur, self.s.dprog[d.n], self.dungeon_keys()))
    }
    pub fn debug_dungeon_sealed(&self) -> bool {
        self.dungeon.as_ref().map_or(false, |d| d.sealed)
    }
    pub fn debug_puzzles(&self) -> Option<(bool, String, String)> {
        self.dungeon.as_ref().map(|d| {
            (d.hub_combat, format!("{:?}", d.puz[dungeon::R_WEST].unwrap()), format!("{:?}", d.puz[dungeon::R_EAST].unwrap()))
        })
    }
    pub fn debug_objs(&self) -> Vec<DebugObj> {
        self.dungeon.as_ref().map_or(vec![], |d| {
            d.objs[d.cur]
                .iter()
                .map(|o| DebugObj {
                    kind: match o.k {
                        OK::Torch => "torch".into(),
                        OK::Block => "block".into(),
                        OK::Lever => "lever".into(),
                        OK::Chest => "chest".into(),
                        OK::Shrine(_) => "shrine".into(),
                    },
                    c: o.c,
                    r: o.r,
                    on: o.on,
                    visible: o.visible,
                })
                .collect()
        })
    }
    pub fn debug_tile(&self, c: i32, r: i32) -> u8 {
        self.tile_at(c, r)
    }
    pub fn debug_boss(&self) -> Option<DebugBoss> {
        self.boss.as_ref().map(|b| DebugBoss {
            name: b.name,
            hp: b.hp,
            max: b.max,
            armor: b.armor,
            phase: b.phase,
            fighting: matches!(b.state, BState::Tele | BState::Act),
            atk: format!("{:?}", b.atk),
            hazards: b.hazards.len(),
            alive: b.alive,
            x: b.x,
        })
    }
    pub fn debug_boss_weak(&self) -> Option<usize> {
        self.boss.as_ref().map(|b| b.weak.idx().min(3))
    }
    pub fn debug_boss_hp(&mut self, frac: f32) {
        if let Some(b) = self.boss.as_mut() {
            b.hp = (b.max * frac).max(0.5);
            if frac < 0.5 {
                b.armor = 0.0;
            }
        }
    }
    /// True if the mage overlaps anything solid (e.g. trapped inside a door).
    pub fn debug_player_stuck(&self) -> bool {
        self.box_solid(self.pl.x, self.pl.y, self.pl.w, self.pl.h)
    }
    pub fn debug_dir(&self) -> u8 {
        self.pl.dir
    }
    pub fn debug_scrolling(&self) -> bool {
        self.scroll.is_some()
    }
    pub fn debug_food(&self) -> f32 {
        self.s.food
    }
    pub fn debug_enemy_bullets(&self) -> usize {
        self.eb.len()
    }
    pub fn debug_msg(&self) -> Option<String> {
        self.msg.as_ref().map(|m| m.0.clone())
    }
    pub fn debug_summary(&self) -> String {
        format!(
            "mode={:?} room={} lair={} dungeon={:?} hp={}/{} mp={:.0} food={:.1} gold={} el={} enemies={} boss_hp={:?}",
            self.mode,
            self.room,
            self.in_lair,
            self.debug_dungeon(),
            self.s.hp,
            self.s.max_hp,
            self.s.mp,
            self.s.food,
            self.s.gold,
            self.el().name(),
            self.enemies.len(),
            self.boss.as_ref().map(|b| b.hp as i32)
        )
    }
}
