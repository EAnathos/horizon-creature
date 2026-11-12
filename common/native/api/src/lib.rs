//! Horizon Creature, the library of the series (common/SPECS.md).
//!
//! The mods of the series reach it as the `creature` crate: the launcher adds it to their `[api]` package because
//! their `horizon-mixin.toml` lists it in `[dependencies]` (Horizon's docs/native-mods.md, "Dependencies between trait
//! mods"). What is here is shared by several mods; what only one mod needs stays in that mod.
//!
//! - `model`: draws a Blockbench model converted by `cargo creature model` (SPECS §3.1).
//! - `sounds`: a mod mob's own sounds, since Horizon prefixes mob sounds with `minecraft:` (SPECS §3.2).
//! - `colours`: colours and patterns, rolled per biome, saved, drawn as two tinted layers (SPECS §7).
//! - `hand`: the item in hand, for items the server's hooks use (the jar, the guide, throwables).
//! - `jar`: the glass jar, catching a small creature and releasing it, its look kept (SPECS §6.1).
//! - `mob`: a creature's own size and silence, its template's being a vanilla mob's (SPECS §3.2, §8.8).
//! - `territory`: where a released creature settles and stays (SPECS §6.2).
//! - `flutter`: how small fliers move (butterflies, moths, fireflies): short flights, landing, resting (SPECS §8).
//! - `guide`: the creature guide book, given at a player's first login (SPECS §9).
//! - `bestiary`: the book's pages, what each player met and found out (SPECS §9).
pub mod bestiary;
pub mod colours;
pub mod flutter;
mod guide;
pub mod hand;
pub mod jar;
pub mod mob;
pub mod model;
pub mod sounds;
pub mod territory;

use horizon_client::first_person::Hand;
use horizon_client::mod_api::BlockPos;
use horizon_client::mod_api::{
    BlockEntityEvent, BlockEntitySubmitEvent, BlockEntityUpdateTagEvent, Compound, DestroyBlockEvent,
    EntityTickPostEvent, ExplosionDetonateEvent, Flow, Hooks, Host, InteractEntityEvent, InteractionKeyEvent,
    LevelHost, LivingChangeTargetEvent, LivingDeathEvent, NetFromServerEvent, PistonMoveEvent, PlayerLoggedInEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ScreenRenderPostEvent, UseItemEvent, UseItemOnEvent,
};

pub fn hooks() -> Box<dyn Hooks> {
    hand::server_used(jar::ITEM);
    hand::server_used(guide::ITEM);
    Box::new(Creature)
}

/// The library's own hooks; the mods call its other functions from theirs.
struct Creature;

impl Hooks for Creature {
    /// A piston doesn't move or break a placed jar.
    fn piston_move(&self, level: &dyn LevelHost, event: &mut PistonMoveEvent<'_>) -> Flow {
        let jar = |p: &BlockPos| jar::is_jar_state(level.block_state([p.x, p.y, p.z]).0);
        if event.push.iter().chain(event.destroy).any(jar) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// A blast spares the placed jars.
    fn explosion_detonate(&self, host: &mut dyn Host, event: &mut ExplosionDetonateEvent<'_>) {
        jar::spare_from_blast(host, event.blocks);
    }

    /// Monsters leave our creatures alone: a target change onto one of them is cancelled.
    fn living_change_target(&self, event: &mut LivingChangeTargetEvent<'_>) -> Flow {
        match event.target_mob {
            Some(target) if mob::spared(event.entity, target) => Flow::Cancel,
            _ => Flow::Continue,
        }
    }

    fn living_death(&self, _: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        mob::forget(event.entity);
        Flow::Continue
    }

    fn player_logged_in(&self, host: &mut dyn Host, event: &mut PlayerLoggedInEvent<'_>) {
        jar::remember_states(host);
        guide::on_login(host, event.player);
    }

    /// The bestiary's 3D view (the creature shown, in the book's scene), and the creature in a placed jar.
    fn block_entity_submit(&self, host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
        bestiary::specimen(host, event);
        jar::draw_inside(host, event);
    }

    /// A placed jar has a block entity, holding its creature.
    fn block_entity(&self, _: &mut dyn Host, event: &mut BlockEntityEvent<'_>) -> Option<(String, Compound)> {
        jar::block_entity(event.block)
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
        jar::update_tag(event.entity)
    }

    /// A placed jar broken gives its filled jar back.
    fn destroy_block(&self, host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        jar::broken(host, event.pos);
        Flow::Continue
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

    /// The use key: with a jar, the guide or a mod's throwable in the main hand, the off hand isn't tried
    /// (`hand::holds_server_used`): the client can't tell that their use acted and would place an off-hand torch.
    fn interaction_key(&self, host: &mut dyn Host, event: &mut InteractionKeyEvent<'_>) -> Flow {
        if event.button == 1 && event.hand == Hand::Off && hand::holds_server_used(host) {
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
