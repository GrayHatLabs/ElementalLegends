//! Side quest: the scholar's lost spellbook. Five pages are hidden in cave chests;
//! bringing all five to the scholar in the village teaches Arcane Blink (R1 / keyboard E),
//! a short teleport in the facing direction that passes through monsters, bullets and
//! over water, but not through walls.

use super::*;

pub(super) const PAGES: u32 = 5;
/// Caves (1-based) whose chests also hold a spellbook page.
pub(super) const PAGE_CAVES: [usize; 5] = [2, 3, 4, 6, 8];
/// Where the scholar stands in the village.
pub(super) const SCHOLAR: (f32, f32) = (160.0, (HUD + 11 * TS + 6) as f32);
const BLINK_DIST: f32 = 48.0;
const BLINK_MP: f32 = 6.0;
const BLINK_CD: i32 = 36;

impl Game {
    pub(super) fn pages_found(&self) -> u32 {
        self.s.pages.count_ones()
    }
    /// The spellbook page in cave k's chest, if it has one and it hasn't been taken.
    pub(super) fn take_page(&mut self, k: usize) -> Option<u32> {
        let slot = PAGE_CAVES.iter().position(|&c| c == k)?;
        if self.s.pages & (1 << slot) != 0 {
            return None;
        }
        self.s.pages |= 1 << slot;
        Some(self.pages_found())
    }
    pub(super) fn near_scholar(&self) -> bool {
        (self.pl.x - SCHOLAR.0).abs() < 14.0 && (self.pl.y - SCHOLAR.1).abs() < 14.0
    }
    /// Talking to the scholar (A while standing beside them, or on arrival).
    pub(super) fn talk_scholar(&mut self) {
        let n = self.pages_found();
        let text = if self.s.blink {
            "SCHOLAR: BLINK THROUGH DANGER, MAGE! PRESS R1 OR E.".to_string()
        } else if n >= PAGES {
            self.s.blink = true;
            self.sfx(Sfx::Fanfare);
            self.flash = 8;
            let (x, y) = (self.pl.x, self.pl.y);
            self.part(x, y, 0.0, 0.0, 24, rgb(0xd878fc), 1, PK::Glow(20.0));
            self.save();
            "SCHOLAR: MY BOOK IS WHOLE AGAIN! I TEACH YOU ARCANE BLINK: PRESS R1 OR E TO TELEPORT A SHORT WAY.".to_string()
        } else if n == 0 {
            "SCHOLAR: MY SPELLBOOK WAS TORN APART AND ITS FIVE PAGES SCATTERED INTO THE CAVES. BRING THEM BACK AND I WILL TEACH YOU A SPELL!".to_string()
        } else {
            format!("SCHOLAR: {} OF 5 PAGES! THE REST ARE STILL LOST IN THE CAVES.", n)
        };
        self.show_msg(text);
        if let Some(m) = self.msg.as_mut() {
            m.1 = 240;
        }
    }
    /// One line for the quest log (pause map) and the notice board.
    pub(super) fn quest_line(&self) -> String {
        if self.s.blink {
            "QUEST: SPELLBOOK RESTORED - BLINK LEARNED".to_string()
        } else {
            format!("QUEST: SPELLBOOK PAGES {}/{}", self.pages_found(), PAGES)
        }
    }

    /// Arcane Blink: teleport up to BLINK_DIST in the facing direction.
    pub(super) fn blink(&mut self) {
        let (x0, y0) = (self.pl.x, self.pl.y);
        if !self.s.blink {
            return;
        }
        if self.pl.cast > 0 || self.blink_cd > 0 {
            return;
        }
        if self.s.mp < BLINK_MP {
            self.float("NO MANA", x0 - 28.0, y0 - 18.0, rgb(0x747474));
            self.sfx(Sfx::Deny);
            return;
        }
        let (fx, fy) = (self.pl.fx, self.pl.fy);
        let l = fx.hypot(fy).max(0.01);
        let (ux, uy) = (fx / l, fy / l);
        // Trace forward: walls stop the blink, water and monsters don't. Land on the
        // farthest spot where the mage fits.
        let mut land = None;
        let mut d = 2.0;
        while d <= BLINK_DIST {
            let (x, y) = (x0 + ux * d, y0 + uy * d);
            if self.shot_blocked(x, y) {
                break;
            }
            if !self.box_solid(x, y, self.pl.w, self.pl.h) {
                land = Some((x, y));
            }
            d += 2.0;
        }
        let Some((x, y)) = land.filter(|&(x, y)| dist(x, y, x0, y0) > 8.0) else {
            self.float("BLOCKED", x0 - 28.0, y0 - 18.0, rgb(0x747474));
            return;
        };
        self.s.mp -= BLINK_MP;
        self.blink_cd = BLINK_CD;
        self.pl.inv = self.pl.inv.max(18);
        for i in 0..10 {
            let t = i as f32 / 9.0;
            let (px, py) = (x0 + (x - x0) * t, y0 + (y - y0) * t);
            self.part(px, py - 6.0, 0.0, -0.3, 14 + i, rgb(0xb040fc), 2, PK::Dot);
        }
        self.part(x0, y0 - 6.0, 0.0, 0.0, 12, rgb(0xd878fc), 1, PK::Burst(Elem::Storm, 10.0));
        self.part(x, y - 6.0, 0.0, 0.0, 12, rgb(0xd878fc), 1, PK::Burst(Elem::Storm, 10.0));
        self.pl.x = x;
        self.pl.y = y;
        self.sfx(Sfx::Zap);
    }
}
