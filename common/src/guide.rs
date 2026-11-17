//! The creature guide book (SPECS §7): `creature:guide`, given once to every player, the first time they join a world.
//!
//! "Given" is remembered in the player's persistent data (NeoForge's `Entity.getPersistentData()`,
//! `PlayerRef::persistent_data`), saved with the player. A full inventory leaves it unmarked: the book comes at the
//! next login. Right-click opens its bestiary (`bestiary`).
use mod_api::{HostSide, InteractionResult, ItemPart, PlayerRef, Registrar, ServerHost, Tag, UseItemEvent};

pub const ITEM: &str = "creature:guide";
/// The key in the player's persistent data.
const GIVEN: &str = "creature:guide_given";

/// From `player_logged_in`: gives `player` the guide if they never had it.
pub fn on_login(host: &mut dyn ServerHost, player: PlayerRef) {
    let Some(mut data) = player.persistent_data(host) else {
        return;
    };
    if data.contains_key(&GIVEN.into()) {
        return;
    }
    let Some(stack) = host.item_from_snbt(&format!("{{id:\"{ITEM}\",count:1}}")) else {
        return;
    };
    if player.give_stack(host, stack).is_err() {
        return;
    }
    data.insert(GIVEN.into(), Tag::Byte(1));
    let _ = player.set_persistent_data(host, data);
}

/// The guide item: right-click opens the bestiary (the server sends the player's pages, the client opens the book).
struct GuideItem;

#[mod_api::sunrise_part]
impl ItemPart for GuideItem {
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        if let HostSide::Server(host) = host {
            crate::bestiary::open(host, event.player);
        }
        Some(InteractionResult::Consume)
    }
}

/// From the library's `register`: the guide item's part.
pub fn declare(r: &mut dyn Registrar) {
    let item = r.item_key(ITEM);
    r.item_part(item, GuideItem);
}
