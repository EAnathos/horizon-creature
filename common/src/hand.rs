//! The items in a player's hands, for items whose use the server's hooks handle: the glass jar, the guide, the mods'
//! throwables (butterfly dust, confetti).
use mod_api::{Hand, Host, ItemStack, LevelRef, PlayerRef, ServerHost};

/// The item id of `stack` (`creature:glass_jar`).
pub fn id(host: &dyn Host, stack: &ItemStack) -> String {
    stack.item().id(host)
}

/// Whether `stack` is the item `item`.
pub fn is(host: &dyn Host, stack: &ItemStack, item: &str) -> bool {
    id(host, stack) == item
}

/// What `player` holds in `hand`.
pub fn held(host: &dyn Host, player: PlayerRef, hand: Hand) -> Option<ItemStack> {
    host.living(player.entity())?.hand_item(host, hand)
}

/// Puts `stack` (or nothing) in `player`'s `hand`.
pub fn set_held(host: &mut dyn ServerHost, player: PlayerRef, hand: Hand, stack: Option<ItemStack>) -> bool {
    host.living(player.entity())
        .is_some_and(|l| l.set_hand_item(host, hand, stack).unwrap_or(false))
}

/// Uses up one of what `player` holds in `hand` (not in creative), from a use hook.
pub fn use_one(host: &mut dyn ServerHost, player: PlayerRef, hand: Hand) {
    if player.has_infinite_materials(host).unwrap_or(false) {
        return;
    }
    if let Some(stack) = held(host, player, hand) {
        set_held(host, player, hand, stack.shrink(1));
    }
}

/// Gives `stack` to `player`, or drops it at their feet when their inventory is full.
pub fn give(host: &mut dyn ServerHost, player: PlayerRef, stack: ItemStack) {
    if player.give_stack(host, stack.clone()).is_err() {
        let _ = player.drop_stack(host, stack, false);
    }
}

/// Drops `stack` as an item on the ground at `at`, with its components (a jar's creature, a dagger's wear).
pub fn drop_stack(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3], stack: ItemStack) {
    let _ = level.spawn_item(host, at[0], at[1], at[2], stack, 10);
}

/// A standing player's eye height.
const EYE_HEIGHT: f64 = 1.62;

/// Where `player`'s eyes are and the way they look (a unit vector, vanilla's `Entity.calculateViewVector`).
pub fn eye_and_look(host: &dyn Host, player: PlayerRef) -> Option<([f64; 3], [f64; 3])> {
    let e = player.entity();
    let at = e.position(host)?;
    let [yaw, pitch] = e.rotation(host)?;
    let (yaw, pitch) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    Some(([at[0], at[1] + EYE_HEIGHT, at[2]], look))
}
