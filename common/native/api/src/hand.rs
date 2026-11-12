//! The item in the acting player's hand, for items whose use only the server's hooks handle: the glass jar, the guide,
//! the mods' throwables (butterfly dust, confetti).
use std::sync::Mutex;

use horizon_client::items::ItemStack;
use horizon_client::mod_api::Host;

/// The items the server's hooks use (`server_used`).
static SERVER_USED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// From a module's `hooks()`: right-clicking `item` acts through the server's hooks. With it in the main hand, the
/// off hand's turn is then skipped (`holds_server_used`, from the library's `interaction_key`): the client can't tell
/// that the use acted and would go on to the off hand and place its torch (shown, then undone by the server, and the
/// torch spent); vanilla stops at the hand that acted.
pub fn server_used(item: &'static str) {
    let mut used = SERVER_USED.lock().unwrap();
    if !used.contains(&item) {
        used.push(item);
    }
}

/// Whether the main hand holds an item `server_used` registered.
pub fn holds_server_used(host: &dyn Host) -> bool {
    host.held()
        .is_some_and(|(item, _, _)| SERVER_USED.lock().unwrap().iter().any(|i| *i == item))
}

/// Uses up one of `stack` from the acting player's hand (not in creative), from a `use_item` hook.
pub fn use_one(host: &mut dyn Host, stack: &ItemStack) {
    let Some(player) = host.acting_player() else {
        return;
    };
    if player.game_mode == 1 {
        return;
    }
    if let Some(s) = slot_of(host, stack).and_then(|slot| host.carried_stack(slot)) {
        s.count = s.count.saturating_sub(1);
    }
}

/// The acting player's slot holding `stack` (the hotbar and inventory, then the off hand).
pub fn slot_of(host: &mut dyn Host, stack: &ItemStack) -> Option<i32> {
    // The selected hotbar slot, when it holds this stack: Horizon doesn't say which slot is selected, but `held` reads
    // it, so each hotbar slot holding the same stack is marked a moment (its count changed, then put back) to see which
    // one `held` sees. Without that, a jar in another slot looking the same was filled in place of the held one.
    for s in 0..9 {
        let Some(count) = host.carried_stack(s).filter(|c| **c == *stack).map(|c| c.count) else {
            continue;
        };
        if let Some(c) = host.carried_stack(s) {
            c.count = count + MARK;
        }
        let selected = host.held().is_some_and(|(_, n, _)| n == count + MARK);
        if let Some(c) = host.carried_stack(s) {
            c.count = count;
        }
        if selected {
            return Some(s);
        }
    }
    (0..36)
        .chain([-106])
        .find(|&s| host.carried_stack(s).is_some_and(|c| *c == *stack))
}

/// A count no stack has, to tell the selected slot (`slot_of`).
const MARK: u32 = 1000;
