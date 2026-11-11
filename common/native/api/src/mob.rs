//! What every creature of the series needs from its mob, since each is a copy of a vanilla mob (the wandering trader):
//! its own size, and silence (SPECS §3.2, §8.8).
use std::sync::Arc;

use horizon_client::mod_api::{Cx, Goal, Mob};

/// Gives `mob` its own box (blocks) in place of its template's: a mob's box comes from its type, so it gets a copy of
/// its type with this size (workaround for horizon-rust/horizon#6276; an `entity_size` hook exists on the fork only).
/// From the mod's `register_goals`.
pub fn resize(mob: &mut Mob, width: f64, height: f64, eye_height: f64) {
    if mob.kind.width == width && mob.kind.height == height {
        return;
    }
    let mut kind = (*mob.kind).clone();
    kind.width = width;
    kind.height = height;
    kind.eye_height = eye_height;
    mob.kind = Arc::new(kind);
}

/// Keeps a creature muted (its template's sounds aren't its own; it plays its own through `sounds`), every tick: a
/// save read after `register_goals` sets `Silent` back. Add it at priority 0; it takes no control.
#[derive(Clone, Debug)]
pub struct Muted;

impl Goal for Muted {
    fn flags(&self) -> u8 {
        0
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        m.silent = true;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
