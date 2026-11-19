//! The spark (SPECS §4), a storm spirit's drop: right-click calls a lightning bolt down on the block looked at, up to
//! `RANGE` blocks away, and is used up (not in creative), with `COOLDOWN` ticks before the next. With no block in
//! range it fails and is kept. Any weather, any dimension; the bolt is a mod's (`strike_lightning`), so it never gives
//! birth to a spirit.
use mod_api::{ClipBlock, ClipFluid, Hand, HostSide, InteractionResult, ItemPart, ServerHost, UseItemEvent};

use crate::{BOLT_FACT, KIND};

pub const SPARK: &str = "storm_spirits:spark";
const RANGE: f64 = 32.0;
const COOLDOWN: i32 = 40;

pub struct Spark;

#[mod_api::sunrise_part]
impl ItemPart for Spark {
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        match host {
            HostSide::Server(host) => Some(if call(host, event.player, event.hand) {
                InteractionResult::Success
            } else {
                InteractionResult::Fail
            }),
            HostSide::Client(_) => Some(InteractionResult::Consume),
        }
    }
}

/// `player` calls lightning with the spark in `hand`: whether it struck.
fn call(host: &mut dyn ServerHost, player: u128, hand: Hand) -> bool {
    let Some(player) = host.player(player) else {
        return false;
    };
    if player.has_item_cooldown(&*host, SPARK).unwrap_or(false) {
        return false;
    }
    let (Some((eye, look)), Some(level)) = (creature::hand::eye_and_look(&*host, player), player.level(&*host)) else {
        return false;
    };
    let to = [0, 1, 2].map(|k| eye[k] + look[k] * RANGE);
    let Some(hit) = level
        .clip(&*host, eye, to, ClipBlock::Outline, ClipFluid::None)
        .filter(|h| !h.miss)
    else {
        return false;
    };
    let b = hit.block;
    let at = [f64::from(b[0]) + 0.5, f64::from(b[1]) + 1.0, f64::from(b[2]) + 0.5];
    if !level.strike_lightning(host, at, false) {
        return false;
    }
    player.set_item_cooldown(host, SPARK, COOLDOWN);
    creature::hand::use_one(host, player, hand);
    creature::bestiary::learn(host, player.uuid(), KIND, BOLT_FACT);
    true
}
