//! Horizon Creature, the library of the series (common/SPECS.md).
//!
//! The mods of the series reach it as the `creature` crate: the launcher adds it to their `[api]` package because
//! their `horizon-mixin.toml` lists it in `[dependencies]` (Horizon's docs/native-mods.md, "Dependencies between trait
//! mods"). What is here is shared by several mods; what only one mod needs stays in that mod.
//!
//! - `model`: draws a Blockbench model converted by `cargo creature model` (SPECS §3.1).
//! - `sounds`: a mod mob's own sounds, since Horizon prefixes mob sounds with `minecraft:` (SPECS §3.2).
//! - `colours`: colours and patterns, rolled per biome, saved, drawn as two tinted layers (SPECS §7).
//! - `jar`: the glass jar, catching a small creature and releasing it, its look kept (SPECS §6.1).
//! - `mob`: a creature's own size and silence, its template's being a vanilla mob's (SPECS §3.2, §8.8).
//! - `territory`: where a released creature settles and stays (SPECS §6.2).
//! - `guide`: the creature guide book, given at a player's first login (SPECS §9).
//! - `bestiary`: the book's pages, what each player met and found out (SPECS §9).
pub mod bestiary;
pub mod colours;
mod guide;
pub mod jar;
pub mod mob;
pub mod model;
pub mod sounds;
pub mod territory;

use horizon_client::first_person::Hand;
use horizon_client::mod_api::{
    BlockEntitySubmitEvent, EntityTickPostEvent, Flow, Hooks, Host, InteractEntityEvent, InteractionKeyEvent,
    NetFromServerEvent, PlayerLoggedInEvent, ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent,
    ScreenRenderPostEvent, UseItemEvent, UseItemOnEvent,
};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Creature)
}

/// The library's own hooks; the mods call its other functions from theirs.
struct Creature;

impl Hooks for Creature {
    fn player_logged_in(&self, host: &mut dyn Host, event: &mut PlayerLoggedInEvent<'_>) {
        guide::on_login(host, event.player);
    }

    /// The bestiary's 3D view: the creature shown, drawn in the book's scene.
    fn block_entity_submit(&self, host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
        bestiary::specimen(host, event);
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        bestiary::tick(host, event.dimension, event.entity, event.kind);
    }

    /// Right-click with the guide: the bestiary opens.
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        if &*event.stack.item != guide::ITEM {
            return Flow::Continue;
        }
        bestiary::open(host);
        Flow::Cancel
    }

    /// An empty glass jar catches a registered creature (before the mods' own right-click).
    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if jar::catch(host, event) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// A filled glass jar releases its creature on the block clicked.
    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        if jar::release(host, event) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// The use key: with a jar in the main hand, the off hand isn't tried (`jar::holds_jar`).
    fn interaction_key(&self, host: &mut dyn Host, event: &mut InteractionKeyEvent<'_>) -> Flow {
        if event.button == 1 && event.hand == Hand::Off && jar::holds_jar(host) {
            event.swing_hand = false;
            return Flow::Cancel;
        }
        Flow::Continue
    }

    fn net_from_server(&self, host: &mut dyn Host, event: &mut NetFromServerEvent<'_>) {
        bestiary::received(host, event.channel, event.data);
    }

    fn screen_render_post(&self, host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
        bestiary::draw(host, event);
    }

    /// The wheel over the book's morph swatches scrolls them.
    fn screen_mouse_scrolled_pre(&self, host: &dyn Host, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
        if bestiary::scrolled(host, event.screen, event.scroll[1]) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        if bestiary::clicked(host, event) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
}
