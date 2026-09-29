//! Reusable elemental status effects shared by enemies and bosses:
//! burning (damage over time), chilled (slowed), frozen (encased) and stunned.
//! Pure state; the game applies damage and plays effects from the events.

/// Burning lasts 4 s; re-igniting refreshes the duration but never stacks damage.
pub const BURN_TIME: i32 = 240;
pub const BURN_TICK: i32 = 30;
pub const BURN_DMG: f32 = 0.6;
/// Burn flames fade out over this many final frames.
pub const BURN_FADE: i32 = 40;
/// First ice hit: chilled (slowed) for 4 s.
pub const CHILL_TIME: i32 = 240;
/// Movement multiplier while chilled (55% speed = 45% slow).
pub const CHILL_SLOW: f32 = 0.55;
/// Second ice hit while chilled: frozen solid for 2.5 s.
pub const FREEZE_TIME: i32 = 150;
/// Bosses thaw faster and cannot be refrozen right away.
pub const BOSS_FREEZE_TIME: i32 = 60;
pub const BOSS_FREEZE_COOLDOWN: i32 = 360;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Status {
    pub burn: i32,
    burn_tick: i32,
    pub chill: i32,
    pub freeze: i32,
    pub freeze_max: i32,
    freeze_cd: i32,
    pub stun: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IceHit {
    Chilled,
    Froze,
    /// Already frozen, or a boss still on freeze cooldown: only re-chills.
    NoEffect,
}

/// What happened during one `tick`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tick {
    /// Fire damage to apply this frame (before elemental multipliers).
    pub burn_dmg: f32,
    /// The ice encasing just ended: play the shatter animation.
    pub thawed: bool,
}

impl Status {
    /// Set on fire. Returns true if the target wasn't already burning.
    pub fn ignite(&mut self) -> bool {
        let fresh = self.burn <= 0;
        self.burn = BURN_TIME;
        if fresh {
            self.burn_tick = BURN_TICK;
        }
        // Fire melts a light chill.
        self.chill = 0;
        fresh
    }

    /// Two-stage ice: the first hit chills, a hit on a chilled target freezes it.
    pub fn ice_hit(&mut self, boss: bool) -> IceHit {
        if self.freeze > 0 {
            return IceHit::NoEffect;
        }
        self.burn = 0;
        if self.chill > 0 && self.freeze_cd <= 0 {
            self.freeze_now(if boss { BOSS_FREEZE_TIME } else { FREEZE_TIME });
            if boss {
                self.freeze_cd = BOSS_FREEZE_COOLDOWN;
            }
            IceHit::Froze
        } else {
            self.chill = CHILL_TIME;
            if self.freeze_cd > 0 {
                IceHit::NoEffect
            } else {
                IceHit::Chilled
            }
        }
    }

    /// Freeze immediately (Frost Nova).
    pub fn freeze_now(&mut self, frames: i32) {
        self.freeze = frames;
        self.freeze_max = frames;
        self.chill = 0;
        self.burn = 0;
    }

    /// Smash the ice early (heavy hits). Returns true if the target was frozen.
    pub fn break_freeze(&mut self) -> bool {
        if self.freeze > 0 {
            self.freeze = 0;
            true
        } else {
            false
        }
    }

    pub fn stun(&mut self, frames: i32) {
        self.stun = self.stun.max(frames);
    }

    pub fn tick(&mut self) -> Tick {
        let mut out = Tick::default();
        if self.burn > 0 {
            self.burn -= 1;
            self.burn_tick -= 1;
            if self.burn_tick <= 0 {
                self.burn_tick = BURN_TICK;
                out.burn_dmg = BURN_DMG;
            }
        }
        if self.freeze > 0 {
            self.freeze -= 1;
            if self.freeze == 0 {
                out.thawed = true;
            }
        }
        if self.chill > 0 {
            self.chill -= 1;
        }
        if self.stun > 0 {
            self.stun -= 1;
        }
        if self.freeze_cd > 0 {
            self.freeze_cd -= 1;
        }
        out
    }

    pub fn frozen(&self) -> bool {
        self.freeze > 0
    }
    pub fn chilled(&self) -> bool {
        self.chill > 0 && self.freeze == 0
    }
    pub fn burning(&self) -> bool {
        self.burn > 0
    }
    /// Can't move or attack.
    pub fn immobile(&self) -> bool {
        self.freeze > 0 || self.stun > 0
    }
    pub fn speed(&self) -> f32 {
        if self.immobile() {
            0.0
        } else if self.chill > 0 {
            CHILL_SLOW
        } else {
            1.0
        }
    }
    /// 0..1 size of burning flames; shrinks as the burn runs out.
    pub fn flame_scale(&self) -> f32 {
        (self.burn as f32 / BURN_FADE as f32).min(1.0)
    }
    /// 0..1 progress of the ice-encasing animation (first 12 frames of a freeze).
    pub fn encase(&self) -> f32 {
        if self.freeze <= 0 {
            0.0
        } else {
            ((self.freeze_max - self.freeze) as f32 / 12.0).min(1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burn_deals_damage_over_time_and_expires() {
        let mut s = Status::default();
        assert!(s.ignite());
        let total: f32 = (0..BURN_TIME + 10).map(|_| s.tick().burn_dmg).sum();
        assert!((total - BURN_DMG * (BURN_TIME / BURN_TICK) as f32).abs() < 1e-4);
        assert!(!s.burning());
    }

    #[test]
    fn reignite_refreshes_without_stacking() {
        let mut s = Status::default();
        s.ignite();
        let mut dmg = 0.0;
        for i in 0..600 {
            if i % 20 == 0 {
                assert!(!s.ignite() || i == 0);
            }
            dmg += s.tick().burn_dmg;
        }
        // One tick every BURN_TICK frames, no matter how often it is refreshed.
        assert!(dmg <= BURN_DMG * (600 / BURN_TICK) as f32 + 1e-4);
        assert!(s.burning());
    }

    #[test]
    fn two_ice_hits_freeze_then_thaw() {
        let mut s = Status::default();
        assert_eq!(s.ice_hit(false), IceHit::Chilled);
        assert!(s.chilled());
        assert!((s.speed() - CHILL_SLOW).abs() < 1e-6);
        assert_eq!(s.ice_hit(false), IceHit::Froze);
        assert!(s.frozen() && s.immobile());
        let mut thawed = false;
        for _ in 0..FREEZE_TIME {
            thawed |= s.tick().thawed;
        }
        assert!(thawed);
        assert!(!s.frozen());
        assert_eq!(s.speed(), 1.0);
    }

    #[test]
    fn bosses_freeze_briefly_with_cooldown() {
        let mut s = Status::default();
        s.ice_hit(true);
        assert_eq!(s.ice_hit(true), IceHit::Froze);
        assert_eq!(s.freeze, BOSS_FREEZE_TIME);
        for _ in 0..BOSS_FREEZE_TIME {
            s.tick();
        }
        s.ice_hit(true);
        assert_eq!(s.ice_hit(true), IceHit::NoEffect);
    }

    #[test]
    fn heavy_hit_breaks_freeze() {
        let mut s = Status::default();
        s.freeze_now(100);
        assert!(s.break_freeze());
        assert!(!s.frozen());
    }
}
