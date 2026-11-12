//! What every creature of the series needs from its mob, since each is a copy of a vanilla mob (the wandering trader):
//! its own size, silence (SPECS §3.2, §8.8), and for small wild ones, despawning far from players.
use std::sync::Arc;

use horizon_client::mod_api::{Cx, Goal, Host, Mob, Tag};

use crate::colours::Random;

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

/// Despawning, as vanilla's `Mob.checkDespawn` for ambient creatures: gone at once with no player within `GONE`
/// blocks, and one chance in `CHANCE` a tick with none within `NEAR`.
const GONE: f64 = 128.0;
const NEAR: f64 = 32.0;
const CHANCE: u64 = 800;

/// From the mod's `entity_tick_post`, for a small wild creature (a butterfly, a moth, a firefly): takes it out of the
/// world far from every player, unless `keep` (settled, say) or it has a name. Whether it was taken out.
pub fn despawn_far(host: &mut dyn Host, dimension: &str, entity: u32, keep: bool) -> bool {
    if keep {
        return false;
    }
    let Some(at) = host.entity(entity).and_then(|e| host.entity_position(e)) else {
        return false;
    };
    let nearest = host
        .server_players()
        .into_iter()
        .filter(|p| p.dimension == dimension)
        .map(|p| (0..3).map(|k| (p.position[k] - at[k]).powi(2)).sum::<f64>().sqrt())
        .fold(f64::MAX, f64::min);
    let gone = nearest > GONE || (nearest > NEAR && Random::new(entity).next() % CHANCE == 0);
    if !gone {
        return false;
    }
    // A name tag keeps it, as in vanilla.
    let named = host
        .entity(entity)
        .and_then(|e| host.entity_nbt(e))
        .is_some_and(|nbt| matches!(nbt.get(&"CustomName".into()), Some(t) if !matches!(t, Tag::End)));
    !named && host.remove_mob(dimension, entity)
}
