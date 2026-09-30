//! The village: merchant's shop (see `Game::shop`), the inn and the notice board.

use super::*;

/// Gold for a night at the inn (full life and magic).
pub(super) const INN_PRICE: i32 = 10;
/// Where the innkeeper stands, in front of the inn door.
pub(super) const INNKEEPER: (f32, f32) = (64.0, (HUD + 11 * TS + 6) as f32);
/// The spot in front of the notice board.
pub(super) const BOARD: (f32, f32) = ((BOARD_TILE.0 as i32 * TS + 8) as f32, (HUD + BOARD_TILE.1 as i32 * TS + 8) as f32);
/// Prompt latches in `shop_armed` for the inn and the board (0..4 are the shop pedestals).
const ARM_INN: usize = 4;
const ARM_BOARD: usize = 5;

impl Game {
    pub(super) fn near_innkeeper(&self) -> bool {
        (self.pl.x - INNKEEPER.0).abs() < 14.0 && (self.pl.y - INNKEEPER.1).abs() < 14.0
    }
    pub(super) fn near_board(&self) -> bool {
        (self.pl.x - BOARD.0).abs() < 12.0 && (6.0..24.0).contains(&(self.pl.y - BOARD.1))
    }
    /// The mage is talking to someone in the village (A talks instead of casting).
    pub(super) fn village_talk_spot(&self) -> bool {
        self.overworld() && self.room == self.shop_room && (self.near_innkeeper() || self.near_board())
    }

    /// Inn and notice board interactions; called every frame on the village screen.
    pub(super) fn village(&mut self) {
        if self.near_innkeeper() {
            if self.shop_armed[ARM_INN] {
                self.shop_armed[ARM_INN] = false;
                self.show_msg(format!("INNKEEPER: A WARM BED AND A HOT MEAL FOR {} GOLD. PRESS A TO REST.", INN_PRICE));
            }
            if self.p(Btn::Fire) {
                self.rest_at_inn();
            }
        } else {
            self.shop_armed[ARM_INN] = true;
        }
        if self.near_board() {
            if self.shop_armed[ARM_BOARD] || self.p(Btn::Fire) {
                self.shop_armed[ARM_BOARD] = false;
                let text = self.notice_text();
                self.show_msg(text);
                if let Some(m) = self.msg.as_mut() {
                    m.1 = 240;
                }
            }
        } else {
            self.shop_armed[ARM_BOARD] = true;
        }
    }
    fn rest_at_inn(&mut self) {
        if self.s.hp >= self.s.max_hp && self.s.mp >= self.s.max_mp as f32 && self.poison <= 0 {
            self.show_msg("INNKEEPER: YOU LOOK WELL RESTED ALREADY, MAGE.");
            self.sfx(Sfx::Deny);
            return;
        }
        if self.s.gold < INN_PRICE {
            self.show_msg("INNKEEPER: SORRY, MAGE. A ROOM COSTS 10 GOLD.");
            self.sfx(Sfx::Deny);
            return;
        }
        self.s.gold -= INN_PRICE;
        self.s.hp = self.s.max_hp;
        self.s.mp = self.s.max_mp as f32;
        if self.poison > 0 {
            self.cure_poison();
        }
        self.flash = 6;
        self.show_msg("YOU REST AT THE INN. LIFE AND MAGIC RESTORED.");
        self.sfx(Sfx::Heal);
        self.save();
    }
    /// What the notice board says (grows as side quests are added).
    pub(super) fn notice_text(&self) -> String {
        let runes = (1..=5).filter(|&i| self.s.cleared[i]).count();
        if runes < 5 {
            format!("NOTICE: FIVE LAIRS HOLD THE RUNES OF THE MONOLITH. RUNES FOUND: {} OF 5. BEWARE THE WILDS AT NIGHT.", runes)
        } else {
            "NOTICE: ALL FIVE RUNES SHINE! THE DARK TOWER'S SEAL IS BROKEN.".to_string()
        }
    }
}
