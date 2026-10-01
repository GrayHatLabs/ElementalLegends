//! The second wave of overworld encounters (one-time, save-flagged like minis.rs):
//!
//! * MIMIC CHEST (Old Crypt) - a chest that breathes; it bites when touched. Weak to fire.
//! * TREASURE GOBLIN (anywhere, rare) - flees, spilling coins when hit; escapes off-screen.
//! * BANDIT RACCOON (Greenwood) - steals gold or food and runs across screens; corner it.
//! * MERCHANT OGRE (anywhere, wanders) - trades if you're fed; if you're starving, so is he.
//! * FAIRY KING (Greenwood) - step into the mushroom ring: the ring closes, beat his pixie
//!   court before time runs out. Reward: speed charm.
//! * HONEY BEAR (Greenwood) - shoot the hive and the bees swarm everything, bear included.
//! * HEADLESS KNIGHT (Old Crypt) - the armour can't be hurt; hit the rolling head.
//! * BANSHEE (Old Crypt) - invisible until you stand still or a storm bolt reveals her;
//!   her scream stuns.
//! * BOG WITCH (Mirefen) - sells cheap food that curses (reversed controls, poison).
//!   Reward: a real discount card for the village shop.
//! * GIANT TOAD (Mirefen) - its tongue pulls you in; swallowed, hammer A to hit its belly.
//! * WILL-O'-WISP (Mirefen) - lures you across quicksand; ice freezes the quicksand.
//! * SALAMANDER QUEEN (Emberpeak) - only hurt while cooled: ice turns her to stone for a
//!   moment, then fire or earth break her.
//! * LAVA GOLEM FORGE (Emberpeak) - reforges from its lava pools until you freeze them all.
//! * PHOENIX (Emberpeak) - dies into an egg; destroy the egg before it hatches.
//! * DOPPELGANGER MAGE (anywhere) - copies your element (immune to it) and copies again a
//!   few seconds after you switch at one of the shrines.
use super::*;
use super::minis::POISON_TIME;

/// Encounter charms (SaveData::charms bits).
pub(super) const CHARM_SPEED: u8 = 1;
pub(super) const CHARM_DISCOUNT: u8 = 2;
pub(super) const CHARM_MIRROR: u8 = 4;
const FAIRY_TIME: i32 = 45 * 60;
const RING_R: f32 = 46.0;
const QUICK_SINK: i32 = 100;

/// Stats for encounter creatures: (hp, speed, w, h, touch, element).
pub(super) fn enc_stats(k: EK) -> Option<(f32, f32, f32, f32, i32, Elem)> {
    Some(match k {
        EK::Mimic => (40.0, 0.9, 14.0, 12.0, 3, Elem::Neutral),
        EK::Goblin => (24.0, 1.3, 10.0, 11.0, 0, Elem::Neutral),
        EK::Raccoon => (15.0, 1.5, 12.0, 9.0, 0, Elem::Neutral),
        EK::Ogre => (80.0, 0.5, 18.0, 20.0, 4, Elem::Earth),
        EK::FairyKing => (30.0, 0.9, 10.0, 10.0, 2, Elem::Storm),
        EK::Pixie => (3.0, 1.4, 6.0, 6.0, 1, Elem::Storm),
        EK::Bear => (60.0, 0.6, 22.0, 18.0, 4, Elem::Earth),
        EK::KnightBody => (999.0, 0.45, 12.0, 14.0, 3, Elem::Neutral),
        EK::KnightHead => (30.0, 1.1, 9.0, 9.0, 1, Elem::Neutral),
        EK::Banshee => (35.0, 0.5, 12.0, 14.0, 0, Elem::Ice),
        EK::Witch => (50.0, 0.5, 10.0, 13.0, 0, Elem::Earth),
        EK::Toad => (60.0, 0.3, 26.0, 20.0, 3, Elem::Ice),
        EK::Wisp => (20.0, 0.9, 8.0, 8.0, 1, Elem::Storm),
        EK::Salamander => (70.0, 0.6, 22.0, 16.0, 4, Elem::Fire),
        EK::LavaGolem => (40.0, 0.4, 16.0, 16.0, 4, Elem::Fire),
        EK::Phoenix => (50.0, 1.0, 22.0, 16.0, 3, Elem::Fire),
        EK::PhoenixEgg => (12.0, 0.0, 10.0, 12.0, 0, Elem::Fire),
        EK::Doppel => (60.0, 0.7, 10.0, 12.0, 2, Elem::Neutral),
        _ => return None,
    })
}
pub(super) fn is_encounter_kind(k: EK) -> bool {
    enc_stats(k).is_some()
}

impl Game {
    fn ecenter(&self) -> (f32, f32) {
        let (c, r) = self.rooms[self.room].center_tile();
        tile_center(c, r)
    }
    fn push_mini(&mut self, k: EK, id: u8, x: f32, y: f32) {
        let e = self.make_mini(k, id, x, y);
        self.enemies.push(e);
    }
    fn reward_line(&mut self, text: &str) {
        self.show_msg(text.to_string());
        if let Some(m) = self.msg.as_mut() {
            m.1 = 260;
        }
        self.sfx(Sfx::Fanfare);
    }
    fn coins(&mut self, x: f32, y: f32, n: i32, each: i32) {
        for i in 0..n {
            let a = i as f32 * 2.399;
            self.add_item(IK::Coin, x + a.cos() * 12.0, y + a.sin() * 8.0, each, Elem::Neutral);
        }
    }

    // ------------------------------------------------------------ room entry
    /// Set up this screen's encounter (called on every overworld room entry).
    pub(super) fn enter_encounter_room(&mut self) {
        self.bees = None;
        self.enc_shrines.clear();
        self.enc_reforge = 0;
        self.fairy_t = 0;
        self.ring_tiles.clear();
        self.tongue = 0;
        self.swallowed = None;
        let ri = self.room;
        let mini = self.rooms[ri].mini;
        let (cx, cy) = self.ecenter();
        if mini >= MINI_MIMIC {
            self.s.mini_seen |= 1 << mini;
        }
        match mini {
            MINI_MIMIC if !self.done(MINI_MIMIC) => {
                let mut m = self.make_mini(EK::Mimic, MINI_MIMIC, cx, cy);
                m.touch = 0;
                self.enemies.push(m);
            }
            MINI_RACCOON if !self.done(MINI_RACCOON) && self.raccoon.is_none() => {
                let (x, y) = (cx + 40.0, cy);
                self.push_mini(EK::Raccoon, MINI_RACCOON, x, y);
            }
            MINI_FAIRY if !self.done(MINI_FAIRY) => {
                self.show_msg("A RING OF MUSHROOMS... TINY LAUGHTER COMES FROM INSIDE IT.");
            }
            MINI_BEAR if !self.done(MINI_BEAR) => {
                self.push_mini(EK::Bear, MINI_BEAR, cx, cy + 36.0);
                self.show_msg("A HONEY BEAR GUARDS ITS HIVE. THE HIVE LOOKS FULL OF ANGRY BEES...");
            }
            MINI_KNIGHT if !self.done(MINI_KNIGHT) => {
                self.push_mini(EK::KnightBody, MINI_KNIGHT, cx - 30.0, cy);
                let mut h = self.make_mini(EK::KnightHead, MINI_KNIGHT, cx + 30.0, cy + 10.0);
                h.vx = 1.0;
                h.vy = 0.6;
                self.enemies.push(h);
                self.show_msg("A HEADLESS KNIGHT! ITS ARMOUR RINGS HOLLOW... WHERE IS ITS HEAD?");
            }
            MINI_BANSHEE if !self.done(MINI_BANSHEE) => {
                self.push_mini(EK::Banshee, MINI_BANSHEE, cx, cy - 30.0);
                self.show_msg("A COLD WAIL ECHOES... SOMETHING UNSEEN DRIFTS NEAR. STAND STILL AND LOOK.");
            }
            MINI_WITCH if !self.done(MINI_WITCH) => {
                let mut w = self.make_mini(EK::Witch, MINI_WITCH, cx, cy - 34.0);
                w.touch = 0;
                self.enemies.push(w);
                self.show_msg("BOG WITCH: HEH HEH! CHEAP FOOD, DEARIE! ONLY 5 GOLD! STAND BY A BOWL AND PRESS A.");
            }
            MINI_TOAD if !self.done(MINI_TOAD) => {
                self.push_mini(EK::Toad, MINI_TOAD, cx, cy - 20.0);
            }
            MINI_WISP if !self.done(MINI_WISP) => {
                self.push_mini(EK::Wisp, MINI_WISP, cx + 30.0, cy - 10.0);
                self.show_msg("A FAINT LIGHT BECKONS ACROSS THE BOG... MIND THE QUICKSAND.");
            }
            MINI_SALAMANDER if !self.done(MINI_SALAMANDER) => {
                self.push_mini(EK::Salamander, MINI_SALAMANDER, cx, cy - 20.0);
                self.show_msg("THE SALAMANDER QUEEN! HER HIDE GLOWS TOO HOT TO HARM. COOL HER FIRST.");
            }
            MINI_FORGE if !self.done(MINI_FORGE) => {
                self.push_mini(EK::LavaGolem, MINI_FORGE, cx, cy);
                self.show_msg("A GOLEM OF LIVING LAVA. ITS POOLS BUBBLE, READY TO REFORGE IT.");
            }
            MINI_PHOENIX if !self.done(MINI_PHOENIX) => {
                self.push_mini(EK::Phoenix, MINI_PHOENIX, cx, cy - 30.0);
            }
            MINI_DOPPEL if !self.done(MINI_DOPPEL) => {
                let mut d = self.make_mini(EK::Doppel, MINI_DOPPEL, cx, cy - 30.0);
                d.el = self.el();
                self.enemies.push(d);
                for (i, (dx, dy)) in [(-60.0, -40.0), (60.0, -40.0), (-60.0, 40.0), (60.0, 40.0)].iter().enumerate() {
                    self.enc_shrines.push((cx + dx, cy + dy, i));
                }
                self.show_msg("A MAGE STEPS OUT OF YOUR SHADOW... IT WIELDS YOUR VERY OWN MAGIC!");
            }
            _ => {}
        }
        // The raccoon carries your loot onto whichever screen it fled to.
        if let Some((room, _, _)) = self.raccoon {
            if room == ri && !self.done(MINI_RACCOON) {
                let mut r = self.make_mini(EK::Raccoon, MINI_RACCOON, cx + 30.0, cy - 20.0);
                r.mode = 1;
                r.spd *= 0.75;
                self.enemies.push(r);
                self.show_msg("THERE'S THE RACCOON WITH YOUR LOOT! CORNER IT!");
            }
        }
        // Roaming: a rare treasure goblin, or the wandering merchant ogre.
        let plain = self.rooms[ri].special == SP_NONE && self.rooms[ri].gate == 0 && mini == 0;
        if plain && self.roamers && self.mode == Mode::Play {
            if !self.done(MINI_GOBLIN) && self.rng.f() < 0.05 {
                if let Some((x, y)) = self.free_spot(70.0, false) {
                    let mut g = self.make_mini(EK::Goblin, MINI_GOBLIN, x, y);
                    g.timer = 900;
                    self.enemies.push(g);
                    self.show_msg("A TREASURE GOBLIN! CATCH IT BEFORE IT ESCAPES!");
                }
            } else if !self.done(MINI_OGRE) && self.rng.f() < 0.06 {
                let (x, y) = (cx, cy - 20.0);
                let mut o = self.make_mini(EK::Ogre, MINI_OGRE, x, y);
                if self.s.food < 20.0 {
                    o.mode = 1;
                    self.show_msg("A HUGE OGRE SNIFFS THE AIR. HIS STOMACH ROARS... HE'S STARVING TOO! HE ATTACKS!");
                    self.sfx(Sfx::Roar);
                } else {
                    o.touch = 0;
                    self.show_msg("OGRE: HUR HUR, LITTLE MAGE! SHINY THINGS FOR SALE! COME CLOSE AND PRESS A.");
                }
                self.enemies.push(o);
            }
        }
    }

    // ------------------------------------------------------------ hits and damage
    /// Encounter reactions to a hit. Returns false when the hit does no damage.
    pub(super) fn enc_hit(&mut self, e: &mut Enemy, el: Elem) -> bool {
        match e.k {
            EK::Mimic if e.mode == 0 => {
                e.mode = 1;
                e.touch = 3;
                self.show_msg("THE CHEST HAS TEETH! A MIMIC!");
                self.sfx(Sfx::Roar);
                true
            }
            EK::Goblin => {
                let (x, y) = (e.x, e.y);
                self.coins(x, y, 2, 4);
                true
            }
            EK::Raccoon if e.mode == 0 => {
                e.mode = 1;
                true
            }
            EK::Ogre | EK::Witch if e.mode == 0 => {
                e.mode = 1;
                e.touch = if e.k == EK::Ogre { 4 } else { 2 };
                self.show_msg(if e.k == EK::Ogre { "OGRE: OW! NO MORE SHINIES FOR YOU! HUR!" } else { "BOG WITCH: HOW DARE YOU! I'LL HEX YOUR BONES!" });
                self.sfx(Sfx::Roar);
                true
            }
            EK::KnightBody => {
                if e.tip <= 0 {
                    e.tip = 30;
                    self.float("CLANG!", e.x - 20.0, e.y - 20.0, rgb(0xbcbcbc));
                }
                e.flash = 2;
                self.spark(e.x, e.y, WHITE);
                self.sfx(Sfx::Click);
                false
            }
            EK::Banshee if e.mode == 0 => {
                if el == Elem::Storm {
                    e.mode = 1;
                    e.timer = 180;
                    self.float("REVEALED!", e.x - 32.0, e.y - 20.0, rgb(0xd878fc));
                    true
                } else {
                    false
                }
            }
            EK::Salamander => {
                if el == Elem::Ice {
                    if e.mode != 2 {
                        self.float("COOLED!", e.x - 28.0, e.y - 22.0, rgb(0xa4e4fc));
                        self.part(e.x, e.y, 0.0, 0.0, 14, rgb(0xa4e4fc), 1, PK::Glow(18.0));
                    }
                    e.mode = 2;
                    e.timer = 240;
                    true
                } else if e.mode == 2 {
                    true
                } else {
                    if e.tip <= 0 {
                        e.tip = 30;
                        self.float("TOO HOT!", e.x - 32.0, e.y - 22.0, rgb(0xfc9838));
                    }
                    self.sfx(Sfx::Click);
                    false
                }
            }
            EK::Doppel if el == e.el && el != Elem::Neutral => {
                if e.tip <= 0 {
                    e.tip = 30;
                    self.float("REFLECTED!", e.x - 36.0, e.y - 20.0, el.light());
                }
                self.sfx(Sfx::Click);
                false
            }
            _ => true,
        }
    }

    /// Encounter rewards (and the forge / phoenix comebacks).
    pub(super) fn enc_defeated(&mut self, e: &Enemy) {
        let (x, y) = (e.x, e.y);
        match e.k {
            EK::Mimic => {
                self.coins(x, y, 8, 15);
                self.s.bombs = (self.s.bombs + 3).min(bag::MAX_BOMBS);
                self.reward_line("THE MIMIC BURSTS OPEN: GOLD AND 3 BOMBS!");
                self.finish_mini(MINI_MIMIC);
            }
            EK::Goblin => {
                self.coins(x, y, 10, 15);
                self.add_item(IK::Gem, x, y, 0, Elem::Neutral);
                self.reward_line("THE TREASURE GOBLIN DROPS ITS WHOLE SACK!");
                self.finish_mini(MINI_GOBLIN);
            }
            EK::Raccoon => {
                let (gold, food) = self.raccoon.map_or((0, 0.0), |(_, g, f)| (g, f));
                self.s.gold = (self.s.gold + gold + 60).min(9999);
                self.s.food = (self.s.food + food).min(100.0);
                self.add_item(IK::Bread, x - 8.0, y, 0, Elem::Neutral);
                self.add_item(IK::Bread, x + 8.0, y, 0, Elem::Neutral);
                self.raccoon = None;
                self.reward_line("CORNERED! THE RACCOON GIVES BACK YOUR LOOT, AND ITS OWN STASH: 60 GOLD AND BREAD.");
                self.finish_mini(MINI_RACCOON);
            }
            EK::Ogre => {
                self.s.elixirs = (self.s.elixirs + 1).min(bag::MAX_ELIXIRS);
                self.s.bombs = (self.s.bombs + 5).min(bag::MAX_BOMBS);
                self.reward_line("THE OGRE STOMPS OFF, DROPPING HIS PACK: AN ELIXIR AND 5 BOMBS.");
                self.finish_mini(MINI_OGRE);
            }
            EK::FairyKing => {
                self.end_fairy_ring();
                self.s.charms |= CHARM_SPEED;
                self.reward_line("THE FAIRY KING BOWS: A SPEED CHARM! YOU MOVE A LITTLE FASTER.");
                self.finish_mini(MINI_FAIRY);
            }
            EK::Bear => {
                self.s.food = 100.0;
                self.s.max_hp += 4;
                self.s.hp = self.s.max_hp;
                self.bees = None;
                self.reward_line("THE BEAR FLEES! YOU FEAST ON HONEY, AND FIND A HEART CONTAINER IN THE HIVE.");
                self.finish_mini(MINI_BEAR);
            }
            EK::KnightHead => {
                self.s.max_mp += 10;
                self.s.mp = self.s.max_mp as f32;
                self.reward_line("THE HEAD SHATTERS AND THE ARMOUR FALLS APART: A MANA CRYSTAL! MAX MAGIC UP.");
                self.finish_mini(MINI_KNIGHT);
            }
            EK::Banshee => {
                self.coins(x, y, 10, 15);
                self.s.elixirs = (self.s.elixirs + 1).min(bag::MAX_ELIXIRS);
                self.reward_line("THE BANSHEE'S WAIL FADES. SHE LEAVES GOLD AND AN ELIXIR.");
                self.finish_mini(MINI_BANSHEE);
            }
            EK::Witch => {
                self.s.charms |= CHARM_DISCOUNT;
                self.curse = 0;
                self.reward_line("THE WITCH MELTS INTO THE BOG, DROPPING A MERCHANT'S CARD: SHOP PRICES 25% OFF!");
                self.finish_mini(MINI_WITCH);
            }
            EK::Toad => {
                self.swallowed = None;
                self.coins(x, y, 8, 15);
                self.s.elixirs = (self.s.elixirs + 1).min(bag::MAX_ELIXIRS);
                self.reward_line("THE GIANT TOAD CROAKS ITS LAST. INSIDE: GOLD AND AN ELIXIR.");
                self.finish_mini(MINI_TOAD);
            }
            EK::Wisp => {
                self.s.elixirs = (self.s.elixirs + 2).min(bag::MAX_ELIXIRS);
                self.reward_line("THE WISP GUTTERS OUT, LEAVING 2 ELIXIRS GLOWING IN THE MUD.");
                self.finish_mini(MINI_WISP);
            }
            EK::Salamander => {
                self.s.max_hp += 4;
                self.s.hp = self.s.max_hp;
                self.reward_line("THE SALAMANDER QUEEN CRUMBLES TO ASH. A HEART CONTAINER! MAX LIFE UP.");
                self.finish_mini(MINI_SALAMANDER);
            }
            EK::LavaGolem => {
                let lava = self.cur_room().tiles.iter().flatten().filter(|&&t| t == T_LAVA).count();
                if lava > 0 {
                    self.enc_reforge = 150;
                    self.show_msg("THE GOLEM SLUMPS... BUT THE LAVA POOLS BEGIN TO STIR. FREEZE THEM!");
                } else {
                    self.coins(x, y, 10, 20);
                    self.s.max_mp += 10;
                    self.s.mp = self.s.max_mp as f32;
                    self.reward_line("WITH ITS POOLS FROZEN, THE GOLEM CAN'T REFORGE! GOLD AND A MANA CRYSTAL.");
                    self.finish_mini(MINI_FORGE);
                }
            }
            EK::Phoenix => {
                let mut egg = self.make_mini(EK::PhoenixEgg, MINI_PHOENIX, x, y);
                egg.timer = 480;
                self.enemies.push(egg);
                self.show_msg("THE PHOENIX FALLS INTO A GLOWING EGG... DESTROY IT BEFORE IT HATCHES!");
                self.sfx(Sfx::Ignite);
            }
            EK::PhoenixEgg => {
                self.s.max_hp += 4;
                self.s.hp = self.s.max_hp;
                self.s.elixirs = (self.s.elixirs + 1).min(bag::MAX_ELIXIRS);
                self.reward_line("THE EGG SHATTERS. THE PHOENIX IS GONE FOR GOOD: A HEART CONTAINER AND AN ELIXIR!");
                self.finish_mini(MINI_PHOENIX);
            }
            EK::Doppel => {
                self.s.charms |= CHARM_MIRROR;
                self.reward_line("YOUR SHADOW SHATTERS LIKE GLASS. A MIRROR CHARM: YOUR BOLTS HIT 20% HARDER!");
                self.finish_mini(MINI_DOPPEL);
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ behaviours
    pub(super) fn upd_encounter(&mut self, e: &mut Enemy, f: f32) {
        let (px, py) = (self.pl.x, self.pl.y);
        let d = dist(e.x, e.y, px, py);
        let a = ang(e.x, e.y, px, py);
        let (w, h) = (self.room_wf(), self.room_hf());
        match e.k {
            EK::Mimic => {
                if e.mode == 0 {
                    if d < 20.0 {
                        e.mode = 1;
                        e.touch = 3;
                        self.show_msg("THE CHEST HAS TEETH! A MIMIC!");
                        self.sfx(Sfx::Roar);
                    }
                    return;
                }
                // Hop at the mage in bursts.
                if e.t % 40 < 16 {
                    let (vx, vy) = (a.cos() * e.spd * 2.4 * f, a.sin() * e.spd * 2.4 * f);
                    self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                }
            }
            EK::Goblin => {
                e.timer -= 1;
                let away = a + PI + (e.t as f32 * 0.05).sin() * 0.6;
                let (vx, vy) = (away.cos() * e.spd * f, away.sin() * e.spd * f);
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                let at_edge = e.x < 14.0 || e.x > w - 14.0 || e.y < HUDF + 14.0 || e.y > h - 14.0;
                if e.timer <= 0 || (at_edge && d > 40.0 && e.t > 240) {
                    e.dead = true;
                    self.show_msg("THE TREASURE GOBLIN GOT AWAY...");
                    self.part(e.x, e.y, 0.0, 0.0, 12, WHITE, 1, PK::Glow(10.0));
                }
            }
            EK::Raccoon => {
                if e.mode == 0 {
                    // Sneak up and snatch something.
                    let (vx, vy) = (a.cos() * e.spd * 0.6 * f, a.sin() * e.spd * 0.6 * f);
                    self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                    if d < 14.0 {
                        let gold = (self.s.gold / 3).min(80);
                        let food = (self.s.food * 0.3).min(30.0);
                        self.s.gold -= gold;
                        self.s.food -= food;
                        self.raccoon = Some((self.room, gold, food));
                        e.mode = 1;
                        self.show_msg(format!("THE RACCOON SNATCHES {} GOLD AND SOME FOOD AND RUNS!", gold));
                        self.sfx(Sfx::Deny);
                    }
                    return;
                }
                // Flee through a doorway away from the mage (not one blocked by a relic gate);
                // leaving takes the loot to the next screen.
                let room = &self.rooms[self.room];
                let doors: Vec<(f32, f32, f32, f32, usize)> = room
                    .links
                    .iter()
                    .filter(|l| !room.relic_gates.iter().any(|g| g.0 == l.d && g.1 == l.seg))
                    .map(|l| {
                        // (doorway at the edge, a point just inside it, next screen)
                        let (sx, sy) = (l.seg as f32 * 256.0 + 128.0, HUDF + l.seg as f32 * 208.0 + 104.0);
                        match l.d {
                            0 => (sx, HUDF + 6.0, sx, HUDF + 40.0, l.to),
                            1 => (sx, h - 6.0, sx, h - 40.0, l.to),
                            2 => (w - 6.0, sy, w - 40.0, sy, l.to),
                            _ => (6.0, sy, 40.0, sy, l.to),
                        }
                    })
                    .collect();
                if doors.is_empty() {
                    return;
                }
                if e.turn <= 0 || e.turn as usize > doors.len() {
                    let pick = (0..doors.len())
                        .max_by(|&a, &b| {
                            let sc = |k: usize| dist(px, py, doors[k].0, doors[k].1) - dist(e.x, e.y, doors[k].0, doors[k].1) * 0.7;
                            sc(a).partial_cmp(&sc(b)).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .unwrap_or(0);
                    e.turn = pick as i32 + 1;
                    e.timer = 0;
                }
                let (gx, gy, ix, iy, to) = doors[e.turn as usize - 1];
                // Stage 0: get in line with the doorway; stage 1: dash out through it.
                if e.timer == 0 && dist(e.x, e.y, ix, iy) < 8.0 {
                    e.timer = 1;
                }
                let (tx, ty) = if e.timer == 0 { (ix, iy) } else { (gx, gy) };
                let b = ang(e.x, e.y, tx, ty);
                let (vx, vy) = (b.cos() * e.spd * f, b.sin() * e.spd * f);
                let (hx, hy) = self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                if hx || hy {
                    // Slide around whatever is in the way.
                    let b2 = b + if (e.t / 50) % 2 == 0 { 1.5 } else { -1.5 };
                    self.move_box(&mut e.x, &mut e.y, e.w, e.h, b2.cos() * e.spd * f, b2.sin() * e.spd * f);
                }
                if e.timer == 1 && dist(e.x, e.y, gx, gy) < 12.0 {
                    e.dead = true;
                    let (g, fd) = self.raccoon.map_or((0, 0.0), |(_, g, f)| (g, f));
                    self.raccoon = Some((to, g, fd));
                    self.show_msg("THE RACCOON ESCAPES TO THE NEXT SCREEN WITH YOUR LOOT!");
                }
            }
            EK::Ogre => {
                if e.mode == 0 {
                    return;
                }
                let (vx, vy) = (a.cos() * e.spd * f, a.sin() * e.spd * f);
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                if e.t % 120 == 0 && d < 60.0 {
                    self.shake = 8;
                    self.sfx(Sfx::Quake);
                    if d < 40.0 {
                        self.hurt_from(3, e.x, e.y);
                    }
                }
            }
            EK::FairyKing => {
                let (cx, cy) = self.ecenter();
                let t = e.t as f32 * 0.03;
                e.x = cx + t.cos() * 26.0;
                e.y = cy + (t * 1.7).sin() * 18.0;
                if e.t % 70 == 0 {
                    for k in -1..=1 {
                        self.eshot(e.x, e.y, a + k as f32 * 0.3, 1.6, Elem::Storm);
                    }
                }
            }
            EK::Pixie => {
                if e.t % 24 == 0 {
                    let b = a + self.rng.range(-1.0, 1.0);
                    e.vx = b.cos() * e.spd;
                    e.vy = b.sin() * e.spd;
                }
                let (vx, vy) = (e.vx * f, e.vy * f);
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
            }
            EK::Bear => {
                let stung = self.bees.is_some();
                let target = if stung { a + (e.t as f32 * 0.1).sin() * 1.5 } else { a };
                let speed = if stung { e.spd * 1.4 } else if d < 90.0 { e.spd } else { 0.0 };
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, target.cos() * speed * f, target.sin() * speed * f);
                if !stung && e.t % 140 == 0 && d < 100.0 {
                    // A charge.
                    e.kbx = a.cos() * 4.0;
                    e.kby = a.sin() * 4.0;
                    self.sfx(Sfx::Roar);
                }
            }
            EK::KnightBody => {
                let (vx, vy) = (a.cos() * e.spd * f, a.sin() * e.spd * f);
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                if e.t % 100 == 0 && d < 50.0 {
                    self.hazards.push(super::bosses::Hazard {
                        k: super::bosses::HK::Sweep, x: e.x + a.cos() * 16.0, y: e.y + a.sin() * 16.0, r: 18.0, w: 0.0, h: 0.0, vx: 0.0,
                        warn: 18, warn_max: 18, act: 8, dmg: 3, track: false,
                    });
                }
            }
            EK::KnightHead => {
                // Rolls and bounces, keeping away from the mage.
                if d < 50.0 && e.t % 20 == 0 {
                    e.vx = -a.cos() * 1.2;
                    e.vy = -a.sin() * 1.2;
                }
                let (hx, hy) = self.move_box(&mut e.x, &mut e.y, e.w, e.h, e.vx * e.spd * f, e.vy * e.spd * f);
                if hx {
                    e.vx = -e.vx;
                }
                if hy {
                    e.vy = -e.vy;
                }
            }
            EK::Banshee => {
                let visible = e.mode == 1;
                if e.timer > 0 {
                    e.timer -= 1;
                }
                if self.still_t > 45 && !visible {
                    e.mode = 1;
                    e.timer = 150;
                }
                if visible && e.timer == 0 && self.still_t <= 45 {
                    e.mode = 0;
                }
                e.touch = if e.mode == 1 { 2 } else { 0 };
                let b = a + (e.t as f32 * 0.02).sin();
                e.x += b.cos() * e.spd * 0.6 * f;
                e.y += b.sin() * e.spd * 0.6 * f;
                e.x = e.x.clamp(12.0, w - 12.0);
                e.y = e.y.clamp(HUDF + 12.0, h - 12.0);
                if e.mode == 1 && d < 80.0 && e.t % 160 == 0 {
                    self.pl_stun = 50;
                    self.hurt(1);
                    self.sfx(Sfx::Roar);
                    self.show_msg("THE BANSHEE SCREAMS! YOU REEL, STUNNED.");
                    for k in 0..12 {
                        let c = k as f32 * PI / 6.0;
                        self.part(e.x, e.y, c.cos() * 2.0, c.sin() * 2.0, 20, rgb(0xd4f4fc), 2, PK::Dot);
                    }
                }
            }
            EK::Witch => {
                if e.mode == 0 {
                    return;
                }
                let b = a + PI / 2.0;
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, b.cos() * e.spd * f, b.sin() * e.spd * f);
                if e.t % 70 == 0 {
                    self.eshot(e.x, e.y, a, 1.8, Elem::Earth);
                }
                let slimes = self.enemies.iter().filter(|m| m.k == EK::Slime && !m.dead).count();
                if e.t % 300 == 0 && slimes < 3 {
                    let th = self.rooms[self.room].theme;
                    let s = self.make_enemy(EK::Slime, e.x + 16.0, e.y + 10.0, th);
                    self.enemies.push(s);
                }
            }
            EK::Toad => {
                if self.swallowed == Some(e.id) {
                    return;
                }
                let in_front = py > e.y && (px - e.x).abs() < 22.0 && py - e.y < 110.0;
                if self.tongue > 0 {
                    self.tongue -= 1;
                    // Pull the mage in.
                    let b = ang(px, py, e.x, e.y);
                    self.pl.kbx = b.cos() * 3.0;
                    self.pl.kby = b.sin() * 3.0;
                    if d < 20.0 {
                        self.tongue = 0;
                        self.swallowed = Some(e.id);
                        self.gulps = 0;
                        e.timer = 240;
                        self.show_msg("GULP! YOU'RE INSIDE THE TOAD! HAMMER A TO STRIKE ITS BELLY!");
                        self.sfx(Sfx::Roar);
                    }
                } else if in_front && e.t % 120 == 0 {
                    self.tongue = 30;
                    self.sfx(Sfx::Zap);
                } else if e.t % 90 == 0 {
                    // Hop a little towards the mage.
                    e.kbx = a.cos() * 3.0;
                    e.kby = a.sin() * 3.0;
                }
            }
            EK::Wisp => {
                let away = if d < 50.0 { a + PI } else { a + (e.t as f32 * 0.03).sin() * 2.0 };
                e.x += away.cos() * e.spd * f;
                e.y += away.sin() * e.spd * f;
                let (cx, cy) = self.ecenter();
                e.x = e.x.clamp(cx - 90.0, cx + 90.0).clamp(12.0, w - 12.0);
                e.y = e.y.clamp(cy - 60.0, cy + 60.0).clamp(HUDF + 12.0, h - 12.0);
            }
            EK::Salamander => {
                if e.mode == 2 {
                    e.timer -= 1;
                    if e.timer <= 0 {
                        e.mode = 0;
                        self.float("HEATS UP AGAIN!", e.x - 48.0, e.y - 22.0, rgb(0xfc9838));
                    }
                    return;
                }
                let (vx, vy) = (a.cos() * e.spd * f, a.sin() * e.spd * f);
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                if e.t % 90 == 0 {
                    for k in -2..=2 {
                        self.eshot(e.x, e.y, a + k as f32 * 0.18, 2.0, Elem::Fire);
                    }
                    self.sfx(Sfx::Ignite);
                }
            }
            EK::LavaGolem => {
                let (vx, vy) = (a.cos() * e.spd * f, a.sin() * e.spd * f);
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, vx, vy);
                if e.t % 110 == 0 && d < 70.0 {
                    self.hazards.push(super::bosses::Hazard {
                        k: super::bosses::HK::Slam, x: px, y: py, r: 16.0, w: 0.0, h: 0.0, vx: 0.0,
                        warn: 30, warn_max: 30, act: 10, dmg: 3, track: false,
                    });
                }
            }
            EK::Phoenix => {
                let (cx, cy) = self.ecenter();
                if e.mode == 1 {
                    e.timer -= 1;
                    e.x += e.vx * f;
                    e.y += e.vy * f;
                    if e.timer <= 0 {
                        e.mode = 0;
                    }
                } else {
                    let t = e.t as f32 * 0.025;
                    let (tx, ty) = (cx + t.cos() * 70.0, cy - 20.0 + (t * 2.0).sin() * 30.0);
                    e.x += (tx - e.x) * 0.05;
                    e.y += (ty - e.y) * 0.05;
                    if e.t % 200 == 0 {
                        e.mode = 1;
                        e.timer = 40;
                        e.vx = a.cos() * 2.6;
                        e.vy = a.sin() * 2.6;
                        self.sfx(Sfx::Roar);
                    }
                    if e.t % 70 == 0 {
                        for k in -1..=1 {
                            self.eshot(e.x, e.y, a + k as f32 * 0.25, 1.9, Elem::Fire);
                        }
                    }
                }
                e.x = e.x.clamp(12.0, w - 12.0);
                e.y = e.y.clamp(HUDF + 12.0, h - 12.0);
            }
            EK::PhoenixEgg => {
                e.timer -= 1;
                if e.timer <= 0 {
                    e.dead = true;
                    let mut p = self.make_mini(EK::Phoenix, MINI_PHOENIX, e.x, e.y);
                    p.spawn = 10;
                    self.enemies.push(p);
                    self.show_msg("THE EGG HATCHES! THE PHOENIX RISES AGAIN AT FULL STRENGTH!");
                    self.sfx(Sfx::Roar);
                    self.boom(e.x, e.y, 20, Elem::Fire);
                }
            }
            EK::Doppel => {
                // It copies a new element a few seconds after you switch.
                let mine = self.el();
                if mine != e.el {
                    self.doppel_copy += 1;
                    if self.doppel_copy > 300 {
                        self.doppel_copy = 0;
                        e.el = mine;
                        self.float("IT COPIES YOUR MAGIC!", e.x - 80.0, e.y - 22.0, mine.light());
                        self.part(e.x, e.y, 0.0, 0.0, 14, mine.light(), 1, PK::Glow(16.0));
                    }
                } else {
                    self.doppel_copy = 0;
                }
                let b = a + PI / 2.0 * if (e.t / 120) % 2 == 0 { 1.0 } else { -1.0 };
                let keep = if d < 60.0 { a + PI } else { b };
                self.move_box(&mut e.x, &mut e.y, e.w, e.h, keep.cos() * e.spd * f, keep.sin() * e.spd * f);
                if e.t % 55 == 0 {
                    self.eshot(e.x, e.y, a, 2.2, e.el);
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ per frame (screen)
    pub(super) fn update_encounters(&mut self) {
        if !self.overworld() {
            return;
        }
        let moved = dist(self.pl.x, self.pl.y, self.room_entry_pos.0, self.room_entry_pos.1) > 0.0 && (self.pl.x, self.pl.y) != self.enc_last;
        self.enc_last = (self.pl.x, self.pl.y);
        self.still_t = if moved { 0 } else { self.still_t + 1 };
        let ri = self.room;
        let mini = self.rooms[ri].mini;
        let (cx, cy) = self.ecenter();
        let (px, py) = (self.pl.x, self.pl.y);
        // Fairy ring.
        if mini == MINI_FAIRY && !self.done(MINI_FAIRY) {
            if self.fairy_t == 0 && dist(px, py, cx, cy) < RING_R - 16.0 {
                self.start_fairy_ring();
            } else if self.fairy_t > 0 {
                self.fairy_t -= 1;
                if self.fairy_t == 0 {
                    self.end_fairy_ring();
                    self.enemies.retain(|e| !matches!(e.k, EK::FairyKing | EK::Pixie));
                    self.pl.x = cx;
                    self.pl.y = cy + RING_R + 18.0;
                    self.show_msg("TIME! THE RING SPITS YOU OUT. THE FAIRY KING LAUGHS... TRY AGAIN.");
                    self.sfx(Sfx::Deny);
                }
            }
        }
        // Bees: chase the bear and the mage alike.
        if let Some((bx, by, t)) = self.bees {
            let bear = self.enemies.iter().position(|e| e.k == EK::Bear && !e.dead);
            let target = match bear {
                Some(i) if (t / 90) % 2 == 0 => (self.enemies[i].x, self.enemies[i].y),
                _ => (px, py),
            };
            let (nx, ny) = (bx + (target.0 - bx) * 0.04, by + (target.1 - by) * 0.04);
            self.bees = if t > 1 { Some((nx, ny, t - 1)) } else { None };
            if t % 30 == 0 {
                if dist(nx, ny, px, py) < 22.0 {
                    self.hurt(1);
                    self.float("BZZT!", px - 20.0, py - 20.0, rgb(0xfce040));
                }
                if let Some(i) = bear {
                    if dist(nx, ny, self.enemies[i].x, self.enemies[i].y) < 30.0 {
                        let mut en = std::mem::take(&mut self.enemies);
                        self.damage_enemy(&mut en[i], 4.0, Elem::Neutral);
                        en.append(&mut self.enemies);
                        self.enemies = en;
                    }
                }
            }
        }
        // Headless knight: when the head is gone the armour falls.
        if mini == MINI_KNIGHT && !self.enemies.iter().any(|e| e.k == EK::KnightHead && !e.dead) {
            for e in self.enemies.iter_mut().filter(|e| e.k == EK::KnightBody) {
                e.dead = true;
            }
        }
        // Inside the toad: hammer A.
        if let Some(id) = self.swallowed {
            match self.enemies.iter().position(|e| e.id == id && !e.dead) {
                Some(i) => {
                    let (tx, ty) = (self.enemies[i].x, self.enemies[i].y);
                    self.pl.x = tx;
                    self.pl.y = ty;
                    self.enemies[i].timer -= 1;
                    if self.p(Btn::Fire) {
                        self.gulps += 1;
                        let mut en = std::mem::take(&mut self.enemies);
                        self.damage_enemy(&mut en[i], 4.0, Elem::Neutral);
                        en.append(&mut self.enemies);
                        self.enemies = en;
                        self.shake = 4;
                    }
                    let timeout = self.enemies.get(i).map_or(true, |e| e.timer <= 0);
                    if self.gulps >= 5 || timeout || self.enemies.get(i).map_or(true, |e| e.dead) {
                        self.swallowed = None;
                        self.pl.y = ty + 26.0;
                        self.pl.kby = 3.0;
                        self.pl.inv = 60;
                        if timeout {
                            self.s.hp -= 2;
                            self.show_msg("THE TOAD SPITS YOU OUT, SLIMY AND SORE.");
                        } else {
                            self.show_msg("THE TOAD RETCHES AND SPITS YOU OUT!");
                        }
                    }
                }
                None => self.swallowed = None,
            }
        }
        // Quicksand.
        let (c, r) = tile_of(px, py + 3.0);
        if self.tile_at(c, r) == T_QUICK {
            self.qsink += 1;
            if self.qsink % 20 == 0 {
                self.part(px, py + 6.0, 0.0, -0.2, 16, rgb(0x6c5030), 2, PK::Dot);
            }
            if self.qsink > QUICK_SINK {
                self.qsink = 0;
                self.hurt(2);
                let (rx, ry) = self.room_entry_pos;
                self.pl.x = rx;
                self.pl.y = ry;
                self.show_msg("THE QUICKSAND SWALLOWS YOU! YOU CLAW YOUR WAY BACK OUT. ICE COULD HARDEN IT.");
            }
        } else {
            self.qsink = 0;
        }
        // The lava golem reforges from an unfrozen pool.
        if self.enc_reforge > 0 {
            self.enc_reforge -= 1;
            if self.enc_reforge == 0 {
                let room = self.cur_room();
                let mut pools = vec![];
                for (y, row) in room.tiles.iter().enumerate() {
                    for (x, &t) in row.iter().enumerate() {
                        if t == T_LAVA {
                            pools.push((x as i32, y as i32));
                        }
                    }
                }
                if let Some(&(x, y)) = pools.first() {
                    let (gx, gy) = tile_center(x, y);
                    let mut g = self.make_mini(EK::LavaGolem, MINI_FORGE, gx, gy);
                    g.spawn = 20;
                    self.enemies.push(g);
                    self.boom(gx, gy, 20, Elem::Fire);
                    self.show_msg("THE GOLEM REFORGES FROM THE LAVA!");
                    self.sfx(Sfx::Roar);
                }
            }
        }
        // Doppelganger shrines.
        let shrines = self.enc_shrines.clone();
        for (sx, sy, el) in shrines {
            if dist(px, py, sx, sy) < 12.0 && self.s.el != el {
                self.set_element(Elem::from_idx(el));
            }
        }
        // Talking to the ogre and buying the witch's food.
        if let Some(i) = self.enemies.iter().position(|e| e.k == EK::Ogre && e.mode == 0 && !e.dead) {
            let (ox, oy) = (self.enemies[i].x, self.enemies[i].y);
            if dist(px, py, ox, oy) < 30.0 && self.p(Btn::Fire) {
                if self.s.bombs < bag::MAX_BOMBS && self.s.gold >= 40 {
                    self.s.gold -= 40;
                    self.s.bombs = (self.s.bombs + 5).min(bag::MAX_BOMBS);
                    self.show_msg("OGRE: 5 BOMBS FOR 40 GOLD! HUR HUR, GOOD DEAL!");
                    self.sfx(Sfx::Coin);
                } else if self.s.elixirs < bag::MAX_ELIXIRS && self.s.gold >= 90 {
                    self.s.gold -= 90;
                    self.s.elixirs += 1;
                    self.show_msg("OGRE: ONE ELIXIR, 90 GOLD. DRINK IT WHEN YOU'RE HURTING!");
                    self.sfx(Sfx::Coin);
                } else {
                    self.show_msg("OGRE: NO GOLD, NO SHINIES! COME BACK RICHER!");
                    self.sfx(Sfx::Deny);
                }
            }
        }
        if mini == MINI_WITCH && !self.done(MINI_WITCH) {
            for (bx, by) in self.witch_bowls() {
                if dist(px, py, bx, by) < 14.0 && self.p(Btn::Fire) && self.enemies.iter().any(|e| e.k == EK::Witch && e.mode == 0) {
                    if self.s.gold < 5 {
                        self.show_msg("BOG WITCH: NO GOLD? THEN NO FOOD, DEARIE.");
                        continue;
                    }
                    self.s.gold -= 5;
                    self.eat(40.0, 0, "STEW");
                    self.curse = 600;
                    self.poison = POISON_TIME;
                    for e in self.enemies.iter_mut().filter(|e| e.k == EK::Witch) {
                        e.mode = 1;
                        e.touch = 2;
                    }
                    self.show_msg("BOG WITCH: HEE HEE HEE! CURSED STEW! YOUR LEGS WON'T OBEY YOU NOW!");
                    self.sfx(Sfx::Roar);
                }
            }
        }
    }
    /// Talking spots where A shouldn't cast (the ogre, the witch's bowls).
    pub(super) fn encounter_talk_spot(&self) -> bool {
        if !self.overworld() {
            return false;
        }
        let (px, py) = (self.pl.x, self.pl.y);
        let ogre = self.enemies.iter().any(|e| e.k == EK::Ogre && e.mode == 0 && !e.dead && dist(px, py, e.x, e.y) < 30.0);
        let witch = self.rooms[self.room].mini == MINI_WITCH
            && self.enemies.iter().any(|e| e.k == EK::Witch && e.mode == 0)
            && self.witch_bowls().iter().any(|&(bx, by)| dist(px, py, bx, by) < 14.0);
        ogre || witch || self.swallowed.is_some()
    }
    pub(super) fn witch_bowls(&self) -> [(f32, f32); 2] {
        let (cx, cy) = self.ecenter();
        [(cx - 32.0, cy), (cx + 32.0, cy)]
    }
    fn start_fairy_ring(&mut self) {
        let (cx, cy) = self.ecenter();
        self.fairy_t = FAIRY_TIME;
        // The ring closes into a wall of mushrooms.
        let room = self.cur_room();
        let (cols, rows) = (room.cols() as i32, room.rows() as i32);
        let mut ring = vec![];
        for r in 1..rows - 1 {
            for c in 1..cols - 1 {
                let (x, y) = tile_center(c, r);
                let dd = dist(x, y, cx, cy);
                if (RING_R - 6.0..RING_R + 8.0).contains(&dd) && self.tile_at(c, r) == T_FLOOR {
                    ring.push((c, r));
                }
            }
        }
        for &(c, r) in &ring {
            self.set_any_tile(c, r, T_DECOR);
        }
        self.ring_tiles = ring;
        self.rerender_current();
        let mut k = self.make_mini(EK::FairyKing, MINI_FAIRY, cx, cy - 20.0);
        k.spawn = 20;
        self.enemies.push(k);
        for i in 0..5 {
            let a = i as f32 * 1.256;
            let mut p = self.make_mini(EK::Pixie, 0, cx + a.cos() * 26.0, cy + a.sin() * 20.0);
            p.mini = 0;
            self.enemies.push(p);
        }
        self.show_msg("THE RING SNAPS SHUT! BEAT THE FAIRY KING BEFORE THE MUSHROOMS CLOSE IN!");
        self.sfx(Sfx::Rumble);
    }
    fn end_fairy_ring(&mut self) {
        self.fairy_t = 0;
        let ring = std::mem::take(&mut self.ring_tiles);
        for (c, r) in ring {
            self.set_any_tile(c, r, T_FLOOR);
        }
        self.rerender_current();
    }
    /// A player bolt struck a solid overworld tile: the beehive.
    pub(super) fn enc_bolt_tile(&mut self, c: i32, r: i32) {
        if self.rooms[self.room].mini != MINI_BEAR || self.done(MINI_BEAR) || self.bees.is_some() {
            return;
        }
        let (cx, cy) = self.rooms[self.room].center_tile();
        if (cx - 1..=cx).contains(&c) && (cy - 3..=cy - 2).contains(&r) {
            let (x, y) = tile_center(c, r);
            self.bees = Some((x, y, 600));
            self.show_msg("THE HIVE SPLITS! A FURIOUS SWARM POURS OUT, STINGING EVERYTHING!");
            self.sfx(Sfx::Roar);
        }
    }
    /// Ice bolts freeze quicksand and lava pools near where they land (overworld).
    pub(super) fn enc_freeze_near(&mut self, x: f32, y: f32) {
        if !self.overworld() {
            return;
        }
        let (c0, r0) = tile_of(x, y);
        let mut changed = false;
        for r in r0 - 1..=r0 + 1 {
            for c in c0 - 1..=c0 + 1 {
                match self.tile_at(c, r) {
                    T_QUICK => {
                        self.set_any_tile(c, r, T_ICE);
                        changed = true;
                    }
                    T_LAVA if self.rooms[self.room].mini == MINI_FORGE => {
                        self.set_any_tile(c, r, T_FLOOR);
                        let (tx, ty) = tile_center(c, r);
                        self.part(tx, ty, 0.0, -0.6, 24, rgb(0xbcbcbc), 3, PK::Dot);
                        changed = true;
                    }
                    _ => {}
                }
            }
        }
        if changed {
            self.rerender_current();
            self.sfx(Sfx::Freeze);
        }
    }

    // ------------------------------------------------------------ drawing
    /// Encounter creatures from their sheets (fallbacks are simple shapes).
    pub(super) fn draw_encounter(&self, scr: &mut Screen, e: &Enemy) -> bool {
        if !is_encounter_kind(e.k) {
            return false;
        }
        let (x, y) = (e.x as i32, e.y as i32);
        let shadow = |scr: &mut Screen, w: i32| scr.blend_ellipse(x, (e.y + e.h / 2.0) as i32, w, 3, BLACK, 0.3);
        let tint_flash = if e.flash > 0 && self.frame % 4 < 2 { Tint::Mix(WHITE, 0.7) } else { Tint::None };
        let drawn = match e.k {
            EK::Mimic => {
                shadow(scr, 8);
                self.draw_creature_hd(scr, e, "enemy_mimic", Some(if e.mode == 0 { "idle" } else { "move" }), 0.0, false, tint_flash)
            }
            EK::Goblin => {
                shadow(scr, 6);
                self.draw_creature_hd(scr, e, "enemy_goblin", None, 0.0, false, tint_flash)
            }
            EK::Raccoon => {
                shadow(scr, 6);
                self.draw_creature_hd(scr, e, "enemy_raccoon", None, 0.0, false, tint_flash)
            }
            EK::Ogre => {
                shadow(scr, 12);
                let t = if e.mode == 1 { Tint::Mix(rgb(0xd82800), 0.2) } else { tint_flash };
                self.draw_creature_hd(scr, e, "npc_ogre", None, 0.0, false, t)
            }
            EK::FairyKing => self.draw_creature_hd(scr, e, "enemy_fairy_king", Some("idle"), -4.0, true, tint_flash),
            EK::Pixie => self.draw_creature_hd(scr, e, "enemy_pixie", Some("idle"), -2.0, true, tint_flash),
            EK::Bear => {
                shadow(scr, 14);
                self.draw_creature_hd(scr, e, "enemy_bear", None, 0.0, false, tint_flash)
            }
            EK::KnightBody => {
                shadow(scr, 8);
                self.draw_creature_hd(scr, e, "enemy_knight_body", None, 0.0, false, tint_flash)
            }
            EK::KnightHead => {
                shadow(scr, 5);
                self.draw_creature_hd(scr, e, "enemy_knight_head", Some("move"), 0.0, false, tint_flash)
            }
            EK::Banshee => {
                if e.mode == 0 {
                    // A faint shimmer only.
                    if self.frame % 40 < 6 {
                        scr.blend_disc(x, y - 6, 8, rgb(0xd4f4fc), 0.12);
                    }
                    true
                } else {
                    self.draw_creature_hd(scr, e, "enemy_banshee", Some(if e.t % 160 > 140 { "attack" } else { "idle" }), -4.0, true, Tint::Mix(rgb(0xd4f4fc), 0.2))
                }
            }
            EK::Witch => {
                shadow(scr, 6);
                self.draw_creature_hd(scr, e, "npc_bog_witch", None, 0.0, false, tint_flash)
            }
            EK::Toad => {
                shadow(scr, 16);
                let anim = if self.swallowed == Some(e.id) { "full" } else if self.tongue > 0 { "attack" } else { "idle" };
                let ok = self.draw_creature_hd(scr, e, "enemy_toad", Some(anim), 0.0, false, tint_flash);
                if self.tongue > 0 {
                    let (px, py) = (self.pl.x as i32, self.pl.y as i32);
                    scr.line(x, y + 4, px, py, rgb(0xd86078));
                    scr.line(x + 1, y + 4, px + 1, py, rgb(0xd86078));
                }
                ok
            }
            EK::Wisp => {
                scr.blend_disc(x, y - 6, 12, rgb(0xa4f4d4), 0.2 + (self.frame as f32 * 0.1).sin().abs() * 0.15);
                self.draw_creature_hd(scr, e, "enemy_wisp", Some("idle"), -6.0, true, tint_flash)
            }
            EK::Salamander => {
                shadow(scr, 14);
                let anim = if e.mode == 2 { "stone" } else if e.t % 90 > 75 { "attack" } else { "move" };
                let t = if e.mode == 2 { Tint::Mix(rgb(0x8c8c98), 0.35) } else { tint_flash };
                self.draw_creature_hd(scr, e, "enemy_salamander_queen", Some(anim), 0.0, false, t)
            }
            EK::LavaGolem => {
                shadow(scr, 10);
                self.draw_creature_hd(scr, e, "enemy_lava_golem", None, 0.0, false, tint_flash)
            }
            EK::Phoenix => {
                scr.blend_ellipse(x, y + 20, 12, 3, BLACK, 0.25);
                scr.blend_disc(x, y - 6, 18, rgb(0xfc9838), 0.15);
                self.draw_creature_hd(scr, e, "enemy_phoenix", Some("move"), -10.0, true, tint_flash)
            }
            EK::PhoenixEgg => {
                let pulse = 0.2 + (e.timer as f32 * if e.timer < 120 { 0.4 } else { 0.1 }).sin().abs() * 0.3;
                scr.blend_disc(x, y - 4, 12, rgb(0xfc9838), pulse);
                self.draw_creature_hd(scr, e, "obj_phoenix_egg", Some("idle"), 0.0, false, tint_flash)
            }
            EK::Doppel => {
                shadow(scr, 6);
                let sheet = ["mage_fire", "mage_ice", "mage_storm", "mage_earth"][e.el.idx().min(3)];
                let t = if e.flash > 0 && self.frame % 4 < 2 { Tint::Mix(WHITE, 0.7) } else { Tint::Mix(rgb(0x100818), 0.55) };
                let ok = self.draw_creature_hd(scr, e, sheet, None, 0.0, false, t);
                scr.blend_disc(x, y - 8, 10, e.el.light(), 0.12);
                ok
            }
            _ => false,
        };
        if !drawn {
            // Fallback shapes in each creature's colours.
            let (c, r) = match e.k {
                EK::Mimic => (rgb(0x8c5020), 8),
                EK::Goblin => (rgb(0x58a838), 6),
                EK::Raccoon => (rgb(0x6c6c78), 6),
                EK::Ogre => (rgb(0x7c9c50), 11),
                EK::FairyKing => (rgb(0xf878f8), 5),
                EK::Pixie => (rgb(0xfcb8f8), 3),
                EK::Bear => (rgb(0x8c5020), 12),
                EK::KnightBody => (rgb(0x5c5c64), 8),
                EK::KnightHead => (rgb(0x9c9ca8), 5),
                EK::Banshee => (rgb(0xd4f4fc), 7),
                EK::Witch => (rgb(0x5c7c30), 6),
                EK::Toad => (rgb(0x4c8c38), 14),
                EK::Wisp => (rgb(0xa4f4d4), 4),
                EK::Salamander => (if e.mode == 2 { rgb(0x8c8c98) } else { rgb(0xd84000) }, 11),
                EK::LavaGolem => (rgb(0x5c2c1c), 9),
                EK::Phoenix => (rgb(0xfc7800), 11),
                EK::PhoenixEgg => (rgb(0xfcbc3c), 5),
                _ => (e.el.main(), 6),
            };
            let col = if e.flash > 0 { WHITE } else { c };
            scr.disc(x, y, r, col);
            scr.disc(x - r / 3, y - r / 3, (r / 3).max(1), mix(col, WHITE, 0.35));
        }
        true
    }
    /// Screen dressing: the fairy ring, the hive, the witch's hut and bowls, the bees,
    /// the doppelganger's shrines, the wisp's fog, the fairy timer.
    pub(super) fn draw_encounter_scene(&self, scr: &mut Screen) {
        if !self.overworld() {
            return;
        }
        let mini = self.rooms[self.room].mini;
        let (cx, cy) = self.ecenter();
        match mini {
            MINI_FAIRY if !self.done(MINI_FAIRY) => {
                let n = 14;
                for i in 0..n {
                    let a = i as f32 * PI * 2.0 / n as f32;
                    let (mx, my) = (cx + a.cos() * RING_R, cy + a.sin() * RING_R * 0.8);
                    if !self.obj_hd(scr, "obj_mushroom", mx, my + 6.0) {
                        scr.fill(mx as i32 - 1, my as i32, 2, 4, rgb(0xd8d0b8));
                        scr.disc(mx as i32, my as i32 - 1, 3, rgb(0xd82800));
                        scr.pset(mx as i32 - 1, my as i32 - 2, WHITE);
                    }
                }
            }
            MINI_BEAR => {
                // The hive tree fills the 2x2 decor block above the centre.
                let (hx, hy) = (cx - 16.0, cy - 32.0);
                if !self.obj_hd(scr, "obj_hive_tree", cx - 8.0, cy - 24.0) {
                    scr.disc(hx as i32 + 8, hy as i32 - 12, 14, rgb(0x2c7c1c));
                    scr.fill(hx as i32 + 6, hy as i32 - 2, 4, 10, rgb(0x5c3410));
                    scr.disc(hx as i32 + 14, hy as i32 - 4, 5, rgb(0xd8a040));
                }
            }
            MINI_WITCH => {
                let base = cy - 56.0;
                let cx = cx - 8.0;
                if !self.obj_hd(scr, "obj_witch_hut", cx, base) {
                    scr.fill(cx as i32 - 28, base as i32 - 26, 56, 26, rgb(0x4c3c20));
                    scr.fill(cx as i32 - 32, base as i32 - 36, 64, 10, rgb(0x2c4c18));
                }
                if !self.done(MINI_WITCH) {
                    for (bx, by) in self.witch_bowls() {
                        scr.fill(bx as i32 - 6, by as i32 + 2, 12, 4, rgb(0x3c2410));
                        scr.disc(bx as i32, by as i32, 5, rgb(0x5c7c30));
                        scr.disc(bx as i32, by as i32 - 1, 3, rgb(0x98d858));
                        scr.text("5", bx as i32 + 1, by as i32 + 8, rgb(0xfcbc3c), Align::Center, 8);
                    }
                }
            }
            MINI_DOPPEL => {
                for &(sx, sy, el) in &self.enc_shrines {
                    let e = Elem::from_idx(el);
                    scr.fill(sx as i32 - 6, sy as i32 + 4, 12, 5, rgb(0x747474));
                    scr.disc(sx as i32, sy as i32 - 2, 5, e.main());
                    scr.disc(sx as i32 - 1, sy as i32 - 3, 2, e.light());
                }
            }
            _ => {}
        }
        if let Some((bx, by, _)) = self.bees {
            for i in 0..14 {
                let a = self.frame as f32 * 0.3 + i as f32 * 0.9;
                let (x, y) = (bx + a.cos() * (6.0 + (i % 4) as f32 * 3.0), by + (a * 1.3).sin() * 6.0);
                scr.fill(x as i32, y as i32, 2, 2, if i % 2 == 0 { rgb(0xfce040) } else { BLACK });
            }
        }
    }
    /// Full-screen overlays: the wisp's fog, the fairy timer, the toad's belly.
    pub(super) fn draw_encounter_overlay(&self, scr: &mut Screen) {
        if !self.overworld() {
            return;
        }
        if self.rooms[self.room].mini == MINI_WISP && !self.done(MINI_WISP) {
            scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, rgb(0x203028), 0.35);
        }
        if self.fairy_t > 0 {
            let secs = (self.fairy_t + 59) / 60;
            scr.text(&format!("RING CLOSES IN {}", secs), 128, HUD + 6, if secs < 10 { rgb(0xfc7460) } else { rgb(0xf878f8) }, Align::Center, 8);
        }
        if self.swallowed.is_some() {
            scr.blend_screen(0, HUD_PX, SW, SH - HUD_PX, rgb(0x581830), 0.55);
            scr.text("INSIDE THE TOAD! HAMMER A!", 128, 120, WHITE, Align::Center, 8);
        }
    }
}

/// Encounter state for the self-test.
#[allow(dead_code)]
pub struct DebugEnc {
    pub charms: u8,
    pub curse: i32,
    pub stun: i32,
    pub swallowed: bool,
    pub raccoon: Option<(usize, i32)>,
    pub bees: bool,
    pub fairy_t: i32,
    pub ring: usize,
    pub lava: usize,
    pub quick: usize,
    pub center: (f32, f32),
    pub prices: Vec<i32>,
}

#[allow(dead_code)]
impl Game {
    pub fn debug_enc(&self) -> DebugEnc {
        let count = |t: u8| self.cur_room().tiles.iter().flatten().filter(|&&v| v == t).count();
        DebugEnc {
            charms: self.s.charms,
            curse: self.curse,
            stun: self.pl_stun,
            swallowed: self.swallowed.is_some(),
            raccoon: self.raccoon.map(|(r, g, _)| (r, g)),
            bees: self.bees.is_some(),
            fairy_t: self.fairy_t,
            ring: self.ring_tiles.len(),
            lava: count(T_LAVA),
            quick: count(T_QUICK),
            center: self.ecenter(),
            prices: self.shop_prices().to_vec(),
        }
    }
    /// Lava / quicksand tile positions on this screen (world coords).
    pub fn debug_enc_tiles(&self, lava: bool) -> Vec<(f32, f32)> {
        let want = if lava { T_LAVA } else { T_QUICK };
        let mut v = vec![];
        for (r, row) in self.cur_room().tiles.iter().enumerate() {
            for (c, &t) in row.iter().enumerate() {
                if t == want {
                    v.push(tile_center(c as i32, r as i32));
                }
            }
        }
        v
    }
    /// Spawn a roaming encounter (Goblin or Ogre) at this screen's centre, as on room entry.
    pub fn debug_enc_spawn(&mut self, kind: &str) -> u32 {
        let (cx, cy) = self.ecenter();
        let (k, id) = if kind == "Goblin" { (EK::Goblin, MINI_GOBLIN) } else { (EK::Ogre, MINI_OGRE) };
        let mut e = self.make_mini(k, id, cx, cy - 20.0);
        if k == EK::Goblin {
            e.timer = 900;
        } else if self.s.food < 20.0 {
            e.mode = 1;
        } else {
            e.touch = 0;
        }
        let i = e.id;
        self.enemies.push(e);
        i
    }
    pub fn debug_enc_bolt_tile(&mut self, c: i32, r: i32) {
        self.enc_bolt_tile(c, r);
    }
    pub fn debug_enc_freeze(&mut self, x: f32, y: f32) {
        self.enc_freeze_near(x, y);
    }
    pub fn debug_enc_center_tile(&self) -> (i32, i32) {
        self.rooms[self.room].center_tile()
    }
    pub fn debug_no_roamers(&mut self) {
        self.roamers = false;
    }
    pub fn debug_enc_clear(&mut self) {
        self.s.charms = 0;
        self.curse = 0;
        self.poison = 0;
        self.raccoon = None;
    }
}
