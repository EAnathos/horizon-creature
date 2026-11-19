//! Creatures that fight back (the scorpions, the storm spirits): hit, a creature of a kind that fights back (`register`)
//! takes whoever hit it as its target, until it lets them go (`pursued`: dead, gone from its level, a creative player
//! or out of reach). The library's damage hook notes who hit it; the mod's own behaviour does the fighting.
//!
//! Single player: the hooks and the behaviours share the mob's id (as the rest of the series).
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use crate::body::{Body, World};

/// The kinds that fight back.
static KINDS: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
/// Each angered creature's target: who hit it last (an entity id).
static TARGETS: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

/// From a mod's `register`: creatures of `kind` fight back when hit.
pub fn register(kind: &'static str) {
    KINDS.lock().unwrap().insert(kind);
}

/// From the library's damage hook: `entity`, of `kind`, was hit by `by`.
pub(crate) fn hit(kind: &str, entity: u32, by: u32) {
    if entity != by && KINDS.lock().unwrap().contains(kind) {
        TARGETS.lock().unwrap().insert(entity, by);
    }
}

/// Whom `entity` fights, if anyone.
pub fn target(entity: u32) -> Option<u32> {
    TARGETS.lock().unwrap().get(&entity).copied()
}

/// `entity` calms down (or is gone).
pub fn forget(entity: u32) {
    TARGETS.lock().unwrap().remove(&entity);
}

/// From a behaviour: the creature's target and where it is, while it is still one to fight (alive, in its level, not a
/// creative player, within `give_up` blocks across); otherwise the creature lets it go.
pub fn pursued(m: &Body, cx: &World<'_>, give_up: f64) -> Option<(u32, [f64; 3])> {
    let who = target(m.id)?;
    let host = cx.host();
    let at = host
        .entity(who)
        .filter(|e| e.level(host) == Some(cx.ground.level))
        .filter(|&e| host.living(e).and_then(|l| l.alive(host)).unwrap_or(false))
        .and_then(|e| e.position(host));
    // The nearest player is the one a behaviour knows: a creative one isn't fought.
    let creative = cx.player.as_ref().is_some_and(|p| p.entity_id == who && p.creative);
    match at {
        Some(at) if !creative && (at[0] - m.position.x).hypot(at[2] - m.position.z) <= give_up => Some((who, at)),
        _ => {
            forget(m.id);
            None
        }
    }
}
