//! The village shop interior: walk through the cottage door into a shop room with the
//! merchant behind the counter and the goods on stands; the south door leads back out.
//! It is a one-room "dungeon" numbered SHOP_N, so room scrolling, exits and the camera
//! work as everywhere else.
use super::dungeon::*;
use super::*;

/// Dungeon number of the shop interior (lairs are 1..=6, caves follow).
pub(super) const SHOP_N: usize = 0;
/// Tileset theme for the interior (wooden floor, plaster walls).
pub(super) const SHOP_THEME: usize = 10;
/// Where the merchant stands, behind the counter.
pub(super) const MERCHANT: (f32, f32) = (128.0, HUDF + 44.0);
/// Counter tiles (row 3) and furniture spots.
const COUNTER_COLS: std::ops::RangeInclusive<usize> = 5..=10;
const SHELVES: [(usize, usize); 2] = [(2, 1), (13, 1)];
const BARRELS: [(usize, usize); 2] = [(1, 11), (14, 11)];

pub(super) fn build_shop(themes: &[Theme]) -> Dungeon {
    let mut has = [false; 7];
    has[R_ENTRY] = true;
    let theme = SHOP_THEME.min(themes.len() - 1);
    let mut rooms = Vec::new();
    for i in 0..7 {
        let mut r = Room::new(i, GRID[i].0 as usize, GRID[i].1 as usize, 9900 + i as u32);
        r.doors = [false, i == R_ENTRY, false, false];
        r.visited = i == R_ENTRY;
        // Marked as the shop so the camera keeps the counter in view (see cam_target).
        r.special = SP_SHOP;
        r.frame();
        if i == R_ENTRY {
            for c in COUNTER_COLS {
                r.tiles[3][c] = T_DECOR;
            }
            for &(c, y) in SHELVES.iter().chain(BARRELS.iter()) {
                r.tiles[y][c] = T_DECOR;
                if y == 1 {
                    r.tiles[2][c] = T_DECOR;
                }
            }
        }
        render(&mut r, &themes[theme]);
        rooms.push(r);
    }
    Dungeon {
        n: SHOP_N, rooms, objs: vec![vec![]; 7], puz: [None; 7], has, larder: vec![vec![]; 7], hub_combat: false, cur: R_ENTRY,
        dirty: false, sealed: false, push_t: 0, crack_hits: vec![], seen: [false; 7], warned: false, cave: false, theme,
        spawns: vec![vec![]; 7], random: vec![None; 7], chal: R_ENTRY, treasure: R_ENTRY,
    }
}

impl Game {
    pub(super) fn in_shop(&self) -> bool {
        self.dungeon.as_ref().map_or(false, |d| d.n == SHOP_N)
    }
    pub(super) fn enter_shop_room(&mut self) {
        if let Some(d) = self.dungeon.as_mut() {
            d.seen[R_ENTRY] = true;
        }
        self.show_msg("MERCHANT: WELCOME, MAGE! STAND BY AN ITEM AND PRESS A TO BUY.");
        if let Some(m) = self.msg.as_mut() {
            m.1 = 170;
        }
    }

    /// Counter, shelves, barrels, rug and merchant (the stands are drawn by draw_shop_stands).
    pub(super) fn draw_shop_interior(&self, scr: &mut Screen) {
        let rug = (128.0, (HUD + 9 * TS + 8) as f32);
        if !self.obj_hd(scr, "obj_rug", rug.0, rug.1 + 16.0) {
            scr.fill(rug.0 as i32 - 40, rug.1 as i32 - 14, 80, 30, rgb(0x881810));
            scr.frame_rect(rug.0 as i32 - 38, rug.1 as i32 - 12, 76, 26, rgb(0xd8a040));
            scr.frame_rect(rug.0 as i32 - 34, rug.1 as i32 - 8, 68, 18, rgb(0x581008));
        }
        for &(c, r) in &SHELVES {
            let (x, y) = ((c as i32 * TS + 8) as f32, (HUD + (r as i32 + 2) * TS) as f32);
            if !self.obj_hd(scr, "obj_shelf", x, y) {
                scr.fill(x as i32 - 8, y as i32 - 30, 16, 30, rgb(0x5c3410));
                for k in 0..3 {
                    let sy = y as i32 - 26 + k * 9;
                    scr.fill(x as i32 - 7, sy + 6, 14, 1, rgb(0x8c5020));
                    scr.fill(x as i32 - 5, sy + 2, 3, 4, [rgb(0x3cbcfc), rgb(0x58d854), rgb(0xd82800)][k as usize]);
                    scr.fill(x as i32 + 1, sy + 1, 3, 5, rgb(0xd8c8a0));
                }
            }
        }
        for &(c, r) in &BARRELS {
            let (x, y) = ((c as i32 * TS + 8) as f32, (HUD + (r as i32 + 1) * TS) as f32);
            if !self.obj_hd(scr, "obj_barrel", x, y) {
                scr.disc(x as i32, y as i32 - 7, 7, rgb(0x8c5020));
                scr.fill(x as i32 - 7, y as i32 - 9, 14, 1, rgb(0x3c2410));
                scr.fill(x as i32 - 7, y as i32 - 4, 14, 1, rgb(0x3c2410));
            }
        }
        // The merchant stands behind the counter.
        let (mx, my) = MERCHANT;
        match self.art.sheet("npc_merchant").and_then(|sh| sh.anim("idle_down").map(|a| (sh.cell, a))) {
            Some((cell, a)) => scr.spr_hd_anchor(a.at(self.frame as u32), mx, my + 7.0, cell.0 / 2, cell.1 - 2, false, Tint::None),
            None => scr.spr(&self.spr.mage_d[4][0].img, mx, my, false),
        }
        let (cx, cy) = (128.0, (HUD + 4 * TS) as f32);
        if !self.obj_hd(scr, "obj_shop_counter", cx, cy) {
            scr.fill(cx as i32 - 48, cy as i32 - 14, 96, 14, rgb(0x8c5020));
            scr.fill(cx as i32 - 48, cy as i32 - 14, 96, 2, rgb(0xb87838));
            scr.fill(cx as i32 - 48, cy as i32 - 2, 96, 2, rgb(0x5c3410));
        }
    }
}
