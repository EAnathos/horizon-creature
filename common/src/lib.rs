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
//!
//! Being ported to Sunrise (docs/SUNRISE-PORT.md), not compiled yet: `bestiary`, `jar`, `friend`, `flutter`, `hand`,
//! `items`, `guide`, `projectile`, `recipes`.
pub mod colours;
pub mod crawl;
pub mod mob;
pub mod model;
pub mod sounds;
pub mod territory;

use mod_api::{
    EntityJoinLevelEvent, EntitySizeEvent, Flow, Hooks, Host, LivingChangeTargetEvent, Registrar, RegistrationContext,
    ServerHost,
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
        territory::declare(r);
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
