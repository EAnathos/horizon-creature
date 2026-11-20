//! Sunrise Creatures, the library of the series (common/SPECS.md).
//!
//! The mods of the series reach it as the `creature` crate: the launcher adds it to their crate because their
//! `sunrise-mod.toml` lists it in `[dependencies]` (Sunrise's modding book, "Dependencies between mods"). What is here
//! is shared by several mods; what only one mod needs stays in that mod.
//!
//! - `model`: draws a Blockbench model converted by `cargo creature model` (SPECS §3.1).
//! - `sounds`: what a mod mob's goals play besides its entity type's sounds (SPECS §3.2).
//! - `colours`: colours and patterns, rolled per biome, saved, drawn as two tinted layers (SPECS §5).
//! - `mob`: a creature's own size and goals, its template's being a vanilla mob's (SPECS §3.2, §6.8).
//! - `territory`: where a released creature settles and stays (SPECS §4.2).
//! - `crawl`: how small crawlers move (worms, scorpions): a slow crawl on the ground, wandering or over a patch (SPECS §3.6).
//! - `body`: behaviours that move a creature by hand from its server tick (flight, following, carrying).
//! - `flutter`: how small fliers move (butterflies, moths, fireflies): short flights, landing, resting (SPECS §3.5).
//! - `friend`: a creature befriended by a player, following them or staying (SPECS §6.2).
//! - `hand`: the items in a player's hands, for items the server's hooks use (the jar, the guide, throwables).
//! - `jar`: the glass jar, catching a small creature and releasing it, its look kept (SPECS §4.1).
//! - `projectile`: thrown things (the cyclops's boulder, the scorpion dagger), flown and ended (SPECS §3.9).
//! - `guide`: the creature guide book, given at a player's first login (SPECS §7).
//! - `bestiary`: the book's pages, what each player met and found out (SPECS §7).
//! - `particles`: the vanilla particles the mods send, their keys asked for at registration.
//! - `items`: the book's Items tab, a page per item of the series, with the recipes of `recipes.rs` (SPECS §7.2).
pub mod anger;
pub mod bestiary;
pub mod body;
pub mod colours;
pub mod crawl;
pub mod flutter;
pub mod friend;
pub mod glow;
mod guide;
pub mod hand;
pub mod items;
pub mod jar;
pub mod mob;
pub mod model;
pub mod particles;
pub mod projectile;
mod recipes;
pub mod sounds;
pub mod territory;

use mod_api::{
    BreakBlockEvent, BuildCreativeModeTabContentsEvent, ClientHost, CreativeTabVisibility, EntityInteractEvent,
    EntityJoinLevelEvent, EntityLeaveLevelEvent, EntitySizeEvent, EntityTickListenedEvent, EntityTickPostEvent,
    ExplosionDetonateEvent, Flow, HitHost, Hooks, Host, HostSide, InteractionResult, LevelTickPostEvent,
    LivingChangeTargetEvent, LivingIncomingDamageEvent, PlayerLoggedInEvent, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent,
    ScreenRenderPostEvent, ServerHost, TabCompleteEvent, VisualsEvent,
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

/// The mod a species or an item (`butterflies:moth`) belongs to: its namespace, Minecraft's own creatures the Vanilla
/// Bestiary's.
pub fn owner(id: &str) -> &str {
    match id.split_once(':').map_or(id, |(ns, _)| ns) {
        "minecraft" => "vanilla_bestiary",
        ns => ns,
    }
}

/// The library's own hooks; the mods call its other functions from theirs.
#[derive(Default)]
struct Creature;

#[mod_api::sunrise_mod]
impl Hooks for Creature {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        colours::declare(r);
        particles::declare(r);
        territory::declare(r);
        friend::declare(r);
        jar::declare(r);
        guide::declare(r);
        bestiary::declare(r);
        bestiary::register_item(&GUIDE_PAGE);
        bestiary::register_item(&JAR_PAGE);
    }

    /// A blast spares the placed jars.
    fn explosion_detonate(&self, host: &mut dyn ServerHost, event: &mut ExplosionDetonateEvent<'_>) {
        if let Some(level) = host.level(event.dimension) {
            jar::spare_from_blast(territory::Ground::new(&*host, level), event.blocks);
        }
    }

    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        if let Some(player) = host.player(event.player) {
            guide::on_login(host, player);
        }
    }

    /// A placed jar broken gives its filled jar back.
    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        jar::broken(host, event.pos);
        Flow::Continue
    }

    /// The game tells entities' ticks (`entity_tick_post`) only to a mod that asks: the series' creatures live in them
    /// (their behaviours, `body`; the bestiary's meetings).
    fn entity_tick_listened(&self, host: HostSide, event: &mut EntityTickListenedEvent) {
        event.listened |= matches!(host, HostSide::Server(_));
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        mob::each_ticked(host, event, |host, event| {
            bestiary::tick(host, event.dimension, event.entity, event.kind);
        });
    }

    fn entity_leave_level(&self, _: &mut dyn ServerHost, event: &mut EntityLeaveLevelEvent<'_>) {
        bestiary::forget(event.entity);
        glow::dim(event.entity);
        anger::forget(event.entity);
    }

    /// Small fliers come out near players.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        if let HostSide::Server(host) = host {
            flutter::spawn(host, event.level);
        }
    }

    /// An empty glass jar catches a registered creature (before the mods' own right-click). Both sides answer, so the
    /// client neither trades with it nor tries the off hand.
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        match host {
            HostSide::Server(host) => jar::catch(host, event).then_some(InteractionResult::Success),
            HostSide::Client(host) => jar::catches(&*host, event).then_some(InteractionResult::Success),
        }
    }

    /// The operators' `/bestiary`, to check its pages.
    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        bestiary::commands(event.commands);
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        if bestiary::perform(host, event) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn tab_complete(&self, _: &mut dyn ServerHost, event: &mut TabCompleteEvent<'_>) {
        bestiary::complete(event);
    }

    /// A creature that fights back takes whoever hit it as its target (`anger`).
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        if let (Ok(entity), Ok(by)) = (u32::try_from(event.entity), u32::try_from(event.source.causing)) {
            anger::hit(event.kind, entity, by);
        }
        Flow::Continue
    }

    /// The glowing creatures' lights (`glow`).
    fn visuals(&self, _: &dyn ClientHost, event: &mut VisualsEvent<'_>) {
        glow::visuals(event);
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        bestiary::draw(host, event);
    }

    /// The wheel over the book's lists scrolls them.
    fn screen_mouse_scrolled_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
        if bestiary::scrolled(host, event.screen, event.scroll[1]) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        if bestiary::clicked(host, event) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// The series' tab shows the glass jar only when a mod uses it (common/SPECS.md §3.3).
    fn build_creative_mode_tab_contents(&self, host: HostSide, event: &mut BuildCreativeModeTabContentsEvent<'_>) {
        if event.tab != "creature:creature" || jar::used() {
            return;
        }
        let host: &dyn Host = match &host {
            HostSide::Server(h) => &**h,
            HostSide::Client(h) => &**h,
        };
        let _ = event
            .edit
            .remove_if(host, CreativeTabVisibility::Both, |s| hand::is(host, s, jar::ITEM));
    }

    /// Our creatures' own boxes.
    fn entity_size(&self, host: &dyn Host, event: &mut EntitySizeEvent<'_>) {
        mob::size(host, event);
    }

    /// Our creatures lose their template's goals for their mod's own.
    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if mob::is_ours(event.kind)
            && let Some(m) = host.entity(event.entity).and_then(|e| host.mob(e))
        {
            mob::clear_template_goals(host, m);
        }
        Flow::Continue
    }

    /// Monsters leave our creatures alone: a target change onto one of them is cancelled.
    fn living_change_target(&self, host: &dyn Host, event: &mut LivingChangeTargetEvent<'_>) -> Flow {
        let Some(target) = *event.new_target else {
            return Flow::Continue;
        };
        let kind = |e: mod_api::EntityRef| e.kind(host).map(|k| k.id(host));
        match (kind(event.mob.entity()), kind(target.entity())) {
            (Some(entity), Some(target)) if mob::spared(&entity, &target) => Flow::Cancel,
            _ => Flow::Continue,
        }
    }
}
