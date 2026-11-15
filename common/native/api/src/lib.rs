//! Horizon Creature, the library of the series (common/SPECS.md).
//!
//! The mods of the series reach it as the `creature` crate: the launcher adds it to their `[api]` package because
//! their `horizon-mixin.toml` lists it in `[dependencies]` (Horizon's docs/native-mods.md, "Dependencies between trait
//! mods"). What is here is shared by several mods; what only one mod needs stays in that mod.
//!
//! - `model`: draws a Blockbench model converted by `cargo creature model` (SPECS §3.1).
//! - `sounds`: a mod mob's own sounds, since Horizon prefixes mob sounds with `minecraft:` (SPECS §3.2).
//! - `colours`: colours and patterns, rolled per biome, saved, drawn as two tinted layers (SPECS §5).
//! - `hand`: the item in hand, for items the server's hooks use (the jar, the guide, throwables).
//! - `jar`: the glass jar, catching a small creature and releasing it, its look kept (SPECS §4.1).
//! - `mob`: a creature's own size and silence, its template's being a vanilla mob's (SPECS §3.2, §6.8).
//! - `territory`: where a released creature settles and stays (SPECS §4.2).
//! - `flutter`: how small fliers move (butterflies, moths, fireflies): short flights, landing, resting (SPECS §3.5).
//! - `crawl`: how small crawlers move (worms, scorpions): a slow crawl on the ground, wandering or over a patch (SPECS §3.6).
//! - `friend`: a creature befriended by a player, following them or staying (SPECS §6.2).
//! - `guide`: the creature guide book, given at a player's first login (SPECS §7).
//! - `bestiary`: the book's pages, what each player met and found out (SPECS §7).
//! - `items`: the book's Items tab, a page per item of the series, with the recipes of `recipes.rs` (SPECS §7.2).
pub mod bestiary;
pub mod colours;
pub mod crawl;
pub mod flutter;
pub mod friend;
mod guide;
pub mod hand;
pub mod items;
pub mod jar;
pub mod mob;
pub mod model;
mod recipes;
pub mod sounds;
pub mod territory;

use horizon_client::first_person::Hand;
use horizon_client::mod_api::BlockPos;
use horizon_client::mod_api::{
    BlockEntityEvent, BlockEntitySubmitEvent, BlockEntityUpdateTagEvent, Compound, DestroyBlockEvent,
    EntityLeaveLevelEvent, EntityTickPostEvent, ExplosionDetonateEvent, Flow, Hooks, Host, InteractEntityEvent,
    InteractionKeyEvent, LevelHost, LevelTickPostEvent, LivingChangeTargetEvent, LivingDeathEvent, NetFromServerEvent,
    PerformCommandEvent, PistonMoveEvent, PlayerLoggedInEvent, RegisterCommandsEvent, ScreenMouseButtonPressedPreEvent,
    ScreenMouseScrolledPreEvent, ScreenRenderPostEvent, TabCompleteV2Event, UseItemEvent, UseItemOnEvent,
};

/// The library's items in the guide's Items tab (SPECS §7.2).
static GUIDE_PAGE: items::ItemPage = items::ItemPage {
    item: guide::ITEM,
    dropped_by: &[],
    given_by: &[items::Use {
        icon: "minecraft:compass",
        key: "bestiary.creature.item.guide.given",
    }],
    uses: &[items::Use {
        icon: guide::ITEM,
        key: "bestiary.creature.item.guide.use",
    }],
};
static JAR_PAGE: items::ItemPage = items::ItemPage {
    item: jar::ITEM,
    dropped_by: &[],
    given_by: &[],
    uses: &[
        items::Use {
            icon: jar::ITEM,
            key: "bestiary.creature.item.glass_jar.catch",
        },
        items::Use {
            icon: "minecraft:poppy",
            key: "bestiary.creature.item.glass_jar.release",
        },
        items::Use {
            icon: "minecraft:oak_planks",
            key: "bestiary.creature.item.glass_jar.place",
        },
    ],
};

/// From a mod's `mods` hook: its row in Options > Mods, for its native half. Horizon lists a content half alone (its
/// items and creatures) when the native one has no row, and switching that row leaves the mod's code running; this row
/// stands for both: switched off, the mod's hooks stop at once (and the library hides its pages, `on`), its content
/// follows on the next start.
pub fn list_mod(event: &mut horizon_client::mod_api::ModsEvent<'_>, id: &str, title: &str, description: &str) {
    event.mods.push(horizon_client::mod_api::NativeMod {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        keys: Vec::new(),
    });
}

/// The mod a species or an item (`butterflies:moth`) belongs to: its namespace, Minecraft's own creatures the Vanilla
/// Bestiary's.
pub fn owner(id: &str) -> &str {
    match id.split_once(':').map_or(id, |(ns, _)| ns) {
        "minecraft" => "vanilla_bestiary",
        ns => ns,
    }
}

/// Whether the mod `id` belongs to is on: one switched off in Options > Mods has its pages hidden and meets nothing.
pub fn on(host: &dyn Host, id: &str) -> bool {
    host.on(owner(id))
}

pub fn hooks() -> Box<dyn Hooks> {
    hand::server_used(jar::ITEM);
    hand::server_used(guide::ITEM);
    bestiary::register_item(&GUIDE_PAGE);
    bestiary::register_item(&JAR_PAGE);
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

    fn entity_leave_level(&self, _: &mut dyn Host, event: &mut EntityLeaveLevelEvent<'_>) {
        bestiary::forget(event.entity);
    }

    /// Small fliers come out near players.
    fn level_tick_post(&self, host: &mut dyn Host, event: &mut LevelTickPostEvent<'_>) {
        flutter::spawn(host, event.dimension);
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

    /// The operators' `/bestiary`, to check its pages.
    fn register_commands(&self, _: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        bestiary::commands(event.commands);
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        if bestiary::perform(host, event) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn tab_complete_v2(&self, _: &mut dyn Host, event: &mut TabCompleteV2Event<'_>) {
        bestiary::complete(event);
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
