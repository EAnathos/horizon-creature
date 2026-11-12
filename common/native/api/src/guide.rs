//! The creature guide book (SPECS §9): `creature:guide`, given once to every player, the first time they join a world.
//!
//! "Given" is remembered in the player's persistent data (NeoForge's `Entity.getPersistentData()`, Horizon's
//! `Host::player_persistent_data`), saved with the player. A full inventory leaves it unmarked: the book comes at the
//! next login. Right-click opens its bestiary (`bestiary.rs`).
use horizon_client::mod_api::{Host, Tag};

pub const ITEM: &str = "creature:guide";
/// The key in the player's persistent data.
const GIVEN: &str = "creature:guide_given";

/// From `player_logged_in`: gives `player` the guide if they never had it. Not called while the book is switched
/// off (SPECS §9).
#[allow(dead_code)]
pub fn on_login(host: &mut dyn Host, player: u128) {
    let Some(mut data) = host.player_persistent_data(player) else {
        return;
    };
    if data.get(GIVEN).is_some() {
        return;
    }
    let Some(stack) = host.item_from_snbt(&format!("{{id:\"{ITEM}\",count:1}}")) else {
        return;
    };
    if host.give_stack(player, stack).is_some() {
        return;
    }
    if let Tag::Compound(map) = &mut data {
        map.insert(GIVEN.into(), Tag::Byte(1));
        host.set_player_persistent_data(player, data);
    }
}
