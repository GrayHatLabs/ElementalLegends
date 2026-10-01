//! Hooks for the headless `--snapshot` / `--selftest` modes. Not used in normal play.
#![allow(dead_code)]
use super::bosses::BState;
use super::dungeon::OK;
use super::*;

/// A snapshot of one creature for the encounter tests.
pub struct DebugMob {
    pub id: u32,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub mode: u8,
    pub hp: f32,
    pub active: bool,
}

impl Game {
    /// Room holding encounter `id` (1 hoard, 2 dryad, 3 treant, 4 graveyard).
    pub fn debug_mini_room(&self, id: u8) -> Option<usize> {
        self.rooms.iter().position(|r| r.mini == id)
    }
    /// The dryad's poison-pool screen and the door that leads there from her screen.
    pub fn debug_pool_room(&self) -> Option<(usize, usize)> {
        let pool = self.rooms.iter().position(|r| r.pool.is_some())?;
        let a = self.debug_mini_room(2)?;
        Some((pool, self.rooms[a].mini_dir))
    }
    pub fn debug_fruit_rooms(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.fruit.iter().map(|f| f.room).collect();
        v.dedup();
        v
    }
    /// (col, row, fruit left, regrow timer, is the treant) for trees on this screen.
    pub fn debug_trees(&self) -> Vec<(i32, i32, i32, i32, bool)> {
        self.fruit.iter().filter(|f| f.room == self.room).map(|f| (f.c, f.r, f.fruit, f.regrow, f.treant)).collect()
    }
    pub fn debug_regrow_now(&mut self) {
        for f in self.fruit.iter_mut() {
            if f.regrow > 0 {
                f.regrow = 1;
            }
        }
    }
    pub fn debug_graves(&self) -> Vec<(i32, i32)> {
        self.rooms[self.room].graves.clone()
    }
    pub fn debug_mini_done(&self, id: u8) -> bool {
        self.s.mini_done(id)
    }
    pub fn debug_mini_seen(&self, id: u8) -> bool {
        self.s.mini_seen & (1 << id) != 0
    }
    pub fn debug_mobs(&self) -> Vec<DebugMob> {
        self.enemies
            .iter()
            .filter(|e| !e.dead)
            .map(|e| DebugMob { id: e.id, kind: format!("{:?}", e.k), x: e.x, y: e.y, mode: e.mode, hp: e.hp, active: e.active() })
            .collect()
    }
    /// Overworld screen holding cave k (1-based).
    pub fn debug_cave_room(&self, k: usize) -> Option<usize> {
        self.rooms.iter().find(|r| r.cave == k).map(|r| r.i)
    }
    pub fn debug_door(&self, room: usize) -> (f32, f32) {
        self.door_pos(room)
    }
    /// (spellbook pages found, Blink learned)
    pub fn debug_quest(&self) -> (u32, bool) {
        (self.pages_found(), self.s.blink)
    }
    pub fn debug_set_pages(&mut self, bits: u32) {
        self.s.pages = bits;
    }
    pub fn debug_cave_cleared(&self, k: usize) -> bool {
        self.cave_cleared(k)
    }
    /// Move a monster and freeze it solid (for the cave plate puzzle).
    pub fn debug_freeze_at(&mut self, id: u32, x: f32, y: f32) {
        if let Some(e) = self.enemies.iter_mut().find(|e| e.id == id) {
            e.x = x;
            e.y = y;
            e.spawn = 0;
            e.st.freeze_now(600);
        }
    }
    pub fn debug_mob(&self, kind: &str) -> Option<DebugMob> {
        self.debug_mobs().into_iter().find(|m| m.kind == kind)
    }
    /// Hit a creature through the normal damage path (weaknesses, reactions, rewards).
    pub fn debug_hit(&mut self, id: u32, dmg: f32, el: usize) {
        let mut en = std::mem::take(&mut self.enemies);
        for e in en.iter_mut().filter(|e| e.id == id) {
            self.damage_enemy(e, dmg, Elem::from_idx(el));
        }
        en.append(&mut self.enemies);
        self.enemies = en;
    }
    /// (kind, x, y, is a hoard coin)
    pub fn debug_items(&self) -> Vec<(String, f32, f32, bool)> {
        self.items.iter().filter(|i| !i.dead).map(|i| (format!("{:?}", i.kind), i.x, i.y, i.tag == ITEM_HOARD)).collect()
    }
    pub fn debug_hoard_left(&self) -> i32 {
        self.s.hoard_left
    }
    pub fn debug_hoard_anger(&self) -> f32 {
        self.hoard_anger
    }
    pub fn debug_zombies(&self) -> i32 {
        self.s.zombies
    }
    pub fn debug_poison(&self) -> i32 {
        self.poison
    }
    pub fn debug_set_poison(&mut self, f: i32) {
        self.poison = f;
    }
    pub fn debug_antidotes(&self) -> i32 {
        self.s.antidotes
    }
    pub fn debug_set_antidotes(&mut self, n: i32) {
        self.s.antidotes = n;
    }
    pub fn debug_dryad_stage(&self) -> u8 {
        self.dryad_stage
    }
    pub fn debug_max(&self) -> (i32, i32) {
        (self.s.max_hp, self.s.max_mp)
    }
    pub fn debug_hazards(&self) -> usize {
        self.hazards.len()
    }
    pub fn debug_wilt_marks(&self) -> usize {
        self.parts.iter().filter(|p| matches!(p.kind, PK::Dot) && p.max == 420).count()
    }
    pub fn debug_paused(&self) -> bool {
        self.paused
    }
}

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
    /// Step inside the village shop (as if through the cottage door).
    pub fn debug_enter_shop(&mut self) {
        self.gate_room = self.shop_room;
        self.start_dungeon(shop::SHOP_N);
    }
    /// Overworld relic gates: (area, side, segment, relic).
    pub fn debug_relic_gates(&self) -> Vec<(usize, usize, usize, usize)> {
        self.rooms.iter().flat_map(|r| r.relic_gates.iter().map(move |g| (r.i, g.0, g.1, g.2))).collect()
    }
    pub fn debug_clear_relics(&mut self) {
        self.s.relics = 0;
    }
    /// Walk into lair n's dungeon directly (as if through its door).
    pub fn debug_enter_lair(&mut self, n: usize) {
        self.gate_room = self.rooms.iter().position(|r| r.gate == n).unwrap_or(self.start);
        self.start_dungeon(n);
    }
    /// Jump to a format 2 lair room by id, standing near its south side.
    pub fn debug_keep_goto(&mut self, id: &str) -> bool {
        let Some(i) = self.dungeon.as_ref().and_then(|d| d.keep.as_ref()).and_then(|k| k.def.rooms.iter().position(|r| r.id == id)) else { return false };
        if let Some(d) = self.dungeon.as_mut() {
            d.cur = i;
        }
        let h = self.cur_room().hf();
        self.pl.x = 128.0;
        self.pl.y = h - 30.0;
        self.enter_droom();
        self.follow_cam(true);
        true
    }
    /// (small keys, big key, map, finder, crystal switches blue, current room solved)
    pub fn debug_keep(&self) -> (i32, bool, bool, bool, bool, bool) {
        let cur = self.dungeon.as_ref().map_or(0, |d| d.cur);
        (self.kkeys(), self.ktok("bigkey"), self.ktok("map"), self.ktok("finder"), self.crystals_blue(), self.ktok(&format!("s{cur}")))
    }
    pub fn debug_has_flag(&self, f: &str) -> bool {
        self.ktok(&format!("f:{f}"))
    }
    pub fn debug_relics(&self) -> u8 {
        self.s.relics
    }
    pub fn debug_grant_relic(&mut self, i: usize) {
        self.s.relics |= 1 << i;
    }
    /// Would this tile block the mage right now?
    pub fn debug_solid_for_mage(&self, c: i32, r: i32) -> bool {
        self.moving_pl.set(true);
        let (x, y) = tile_center(c, r);
        let s = self.solid_at(x, y);
        self.moving_pl.set(false);
        s
    }
    /// Select a bag slot by name ("VINE WHIP", "FEATHER CLOAK", ...).
    pub fn debug_select_slot(&mut self, name: &str) -> bool {
        match self.slots().iter().position(|s| s.name() == name) {
            Some(i) => {
                self.s.bag_sel = i;
                true
            }
            None => false,
        }
    }
    pub fn debug_hover(&self) -> i32 {
        self.hover
    }
    pub fn debug_dark(&self) -> Option<f32> {
        self.keep_darkness()
    }
    /// Forget every level file (the self-test runs on the generated layouts).
    pub fn debug_levels_clear(&mut self) {
        self.levels = crate::levels::Levels::default();
    }
    /// Load the built-in level files (as the real game does, without the override folders).
    pub fn debug_levels_builtin(&mut self) {
        self.levels = crate::levels::Levels::default();
        for (file, text) in crate::levels_gen::LEVELS {
            self.levels.add(file, text, "built-in");
        }
    }
    pub fn debug_add_level(&mut self, file: &str, text: &str) -> bool {
        self.levels.add(file, text, "self-test");
        let stem = file.trim_end_matches(".json");
        self.levels.by_name.contains_key(stem) || stem.strip_prefix("lair_").and_then(|n| n.parse().ok()).map_or(false, |n| self.levels.keeps.contains_key(&n))
    }
    /// Jump to a room of the current dungeon.
    pub fn debug_droom(&mut self, room: usize) {
        if let Some(d) = self.dungeon.as_mut() {
            d.cur = room;
        }
        self.enter_droom();
    }
    pub fn debug_room_dist(&self, i: usize) -> i32 {
        self.rooms[i].dist
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
    /// A 2x2 wilderness area and one of its doorways: (room, side, segment).
    pub fn debug_big_room(&self) -> Option<(usize, usize, usize)> {
        self.rooms.iter().filter(|r| r.big()).find_map(|r| r.links.first().map(|l| (r.i, l.d, l.seg)))
    }
    /// (width, bottom edge) of an overworld area in logic units.
    pub fn debug_room_size(&self, i: usize) -> (f32, f32) {
        (self.rooms[i].wf(), self.rooms[i].hf())
    }
    pub fn debug_cam(&self) -> (f32, f32) {
        self.cam
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
    /// (selected slot, bombs, elixirs, lit bombs on the ground)
    pub fn debug_bag(&self) -> (usize, i32, i32, usize) {
        (self.s.bag_sel, self.s.bombs, self.s.elixirs, self.bombs.len())
    }
    pub fn debug_set_bag(&mut self, sel: usize, bombs: i32, elixirs: i32) {
        self.s.bag_sel = sel;
        self.s.bombs = bombs;
        self.s.elixirs = elixirs;
    }
    pub fn debug_muted(&self) -> bool {
        self.muted
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
                        other => format!("{other:?}").to_lowercase(),
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
    /// Placed food (feast hall / pantry) currently on the floor.
    pub fn debug_larder_items(&self) -> usize {
        self.items.iter().filter(|i| !i.dead && i.life > i32::MAX / 4).count()
    }
    pub fn debug_mp(&self) -> f32 {
        self.s.mp
    }
    pub fn debug_set_mp(&mut self, mp: f32, max: i32) {
        self.s.max_mp = max;
        self.s.mp = mp;
    }
    pub fn debug_potions(&self) -> i32 {
        self.s.potions
    }
    pub fn debug_set_potions(&mut self, n: i32) {
        self.s.potions = n;
    }
    pub fn debug_add_potion_item(&mut self, x: f32, y: f32) {
        self.add_item(IK::Potion, x, y, 0, Elem::Neutral);
    }
    pub fn debug_set_food(&mut self, f: f32) {
        self.s.food = f;
    }
    pub fn debug_set_hp(&mut self, hp: i32, max: i32) {
        self.s.max_hp = max;
        self.s.hp = hp;
    }
    pub fn debug_set_spell_lv(&mut self, lv: i32) {
        self.s.spell_lv = lv;
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
