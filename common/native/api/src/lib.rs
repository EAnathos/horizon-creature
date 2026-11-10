//! Horizon Creature, the library of the series (common/SPECS.md).
//!
//! The mods of the series reach it as the `creature` crate: the launcher adds it to their `[api]` package because
//! their `horizon-mixin.toml` lists it in `[dependencies]` (Horizon's docs/native-mods.md, "Dependencies between trait
//! mods"). What is here is shared by several mods; what only one mod needs stays in that mod.
//!
//! - `model`: draws a Blockbench model converted by `cargo creature model` (SPECS §3.1).
//! - `sounds`: a mod mob's own sounds, since Horizon prefixes mob sounds with `minecraft:` (SPECS §3.2).
//! - `guide`: the creature guide book, given at a player's first login (SPECS §9).
//! - `bestiary`: the book's pages, what each player met and found out (SPECS §9).
pub mod bestiary;
mod guide;
pub mod model;
pub mod sounds;

use horizon_client::mod_api::{
    EntityTickPostEvent, Flow, Hooks, Host, NetFromServerEvent, PlayerLoggedInEvent, ScreenMouseButtonPressedPreEvent,
    ScreenRenderPostEvent, UseItemEvent,
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

    fn net_from_server(&self, host: &mut dyn Host, event: &mut NetFromServerEvent<'_>) {
        bestiary::received(host, event.channel, event.data);
    }

    fn screen_render_post(&self, host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
        bestiary::draw(host, event);
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
