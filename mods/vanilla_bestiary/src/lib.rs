//! Vanilla Bestiary (SPECS.md): Minecraft's own creatures in the creature guide's bestiary, of the Sunrise Creatures
//! series.
//!
//! Each creature of `creatures.rs` is registered with the library's bestiary (`creature::bestiary`), with the biomes,
//! drops and breeding food the game's data gives (`game_data.rs`, from `cargo creature vanilla`). Its page shows the
//! creature itself, turning (the book draws a vanilla mob through `DrawContext::entity`). A player meets one as the
//! series' creatures, a wild one only: one hatched from a spawn egg, bred, let out of a bucket or tamed carries the tag
//! `NOT_WILD` (from `finalize_spawn`'s spawn reason, `entity_breed`, `entity_tame`). The facts are found out through the game's hooks: love mode (breeding), taming, an item used on it
//! (milk), shears or a brush (wool), a bucket, what it drops on its own (an egg), what it drops when a player kills it,
//! and one thing it does (a creeper exploding, an enderman taking a block, a zombie turning into a drowned).
mod creatures;
mod game_data;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use creature::bestiary::{Fact, Species};
use mod_api::{
    BlockEntityDrawContext, EntityBreedEvent, EntityChangeBlockEvent, EntityDropItemEvent, EntityEnterLoveModeEvent,
    EntityInteractEvent, EntityJoinLevelEvent, EntityLeaveLevelEvent, EntityTameEvent, EntityTickPostEvent,
    ExplosionPrimeEvent, FinalizeSpawnEvent, Flow, Hooks, Host, HostSide, InteractionResult, LivingConversionPostEvent,
    LootDropsEvent, PlayerBucketEntityEvent, Registrar, RegistrationContext, ServerHost, SpawnReason, Tag,
};

use creatures::{Breed, CREATURES, Creature, How, Seen};

/// What the game's data says of a creature (`game_data.rs`).
pub struct GameData {
    pub id: &'static str,
    /// The biomes it spawns in on its own.
    pub biomes: &'static [&'static str],
    /// The items its loot table can drop.
    pub drops: &'static [&'static str],
    /// The first item of its `<creature>_food` tag.
    pub food: Option<&'static str>,
}

/// On a creature that isn't wild: hatched from a spawn egg, bred or tamed (saved with it, as vanilla's `Tags`).
const NOT_WILD: &str = "vanilla_bestiary.not_wild";
const BREED: &str = "breed";
const TAME: &str = "tame";
const SPECIAL: &str = "special";
const KEYS: &str = "bestiary.vanilla_bestiary";

/// The creatures known not to be wild, by entity id (from their tag, read when they join the level).
static NOT_WILD_IDS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
/// Where each creeper is, to tell which one an explosion was.
static CREEPERS: Mutex<BTreeMap<u32, ([f64; 3], String)>> = Mutex::new(BTreeMap::new());

fn creature(kind: &str) -> Option<&'static Creature> {
    CREATURES.iter().find(|c| c.id == kind)
}

fn leak<T>(items: Vec<T>) -> &'static [T] {
    Box::leak(items.into_boxed_slice())
}

fn key(word: &str) -> &'static str {
    Box::leak(format!("{KEYS}.{word}").into_boxed_str())
}

/// The bestiary's page of `c`: its biomes (the game's and its own), its facts in the page's order (breeding, taming,
/// products, its special fact, then its drops, each an item alone).
fn species(c: &'static Creature) -> Species {
    let data = game_data::GAME_DATA.iter().find(|d| d.id == c.id);
    let name = c.id.trim_start_matches("minecraft:");
    let mut biomes: Vec<&'static str> = data.map_or(&[][..], |d| d.biomes).to_vec();
    for b in c.biomes {
        if !biomes.contains(b) {
            biomes.push(b);
        }
    }
    let mut facts = Vec::new();
    let food = match c.breed {
        Breed::Data => data.and_then(|d| d.food),
        Breed::With(item) => Some(item),
        Breed::No => None,
    };
    if let Some(item) = food {
        facts.push(Fact {
            id: BREED,
            item,
            key: key(BREED),
        });
    }
    if let Some(item) = c.tame {
        facts.push(Fact {
            id: TAME,
            item,
            key: key(TAME),
        });
    }
    for p in c.products {
        facts.push(Fact {
            id: Box::leak(format!("product.{}", p.fact).into_boxed_str()),
            item: p.item,
            key: key(p.fact),
        });
    }
    if let Some(s) = &c.special {
        facts.push(Fact {
            id: SPECIAL,
            item: s.item,
            key: key(&format!("special.{name}")),
        });
    }
    for item in data.map_or(&[][..], |d| d.drops) {
        facts.push(Fact {
            id: Box::leak(format!("drop.{item}").into_boxed_str()),
            item,
            key: "",
        });
    }
    Species {
        id: c.id,
        biomes: leak(biomes),
        wild: |id| !NOT_WILD_IDS.lock().unwrap().contains(&id),
        facts: leak(facts),
        icon: Box::leak(format!("{}_spawn_egg", c.id).into_boxed_str()),
        scheme: None,
        rarity: c.rarity,
        draw: no_drawing,
    }
}

/// `players` found out `fact` about `kind`, when it is one of its facts.
fn learn(host: &mut dyn ServerHost, players: &[u128], kind: &str, fact: &str) {
    let known = creature::bestiary::species_of(kind).is_some_and(|s| s.facts.iter().any(|f| f.id == fact));
    if !known {
        return;
    }
    for &player in players {
        creature::bestiary::learn(host, player, kind, fact);
    }
}

/// The players near entity `id`.
fn near(host: &dyn ServerHost, id: u32) -> Vec<u128> {
    let Some(e) = host.entity(id) else {
        return Vec::new();
    };
    match (e.position(host), e.level(host).and_then(|l| l.dimension().id(host))) {
        (Some(at), Some(dimension)) => creature::bestiary::players_near(host, &dimension, at),
        _ => Vec::new(),
    }
}

/// The players near `at` in `dimension`.
fn near_at(host: &dyn ServerHost, dimension: &str, at: [f64; 3]) -> Vec<u128> {
    creature::bestiary::players_near(host, dimension, at)
}

/// Creature `id` isn't wild: it no longer counts as met, now and once saved.
fn mark(host: &mut dyn ServerHost, id: u32) {
    if let Some(e) = host.entity(id) {
        e.add_tag(host, NOT_WILD);
    }
    NOT_WILD_IDS.lock().unwrap().insert(id);
}

/// Whether creature `id`, back from a save, isn't wild: tagged, or tamed before this mod was installed.
fn saved_not_wild(host: &mut dyn ServerHost, id: u32) -> bool {
    let Some(e) = host.entity(id) else {
        return false;
    };
    if e.tags(host).is_some_and(|tags| tags.iter().any(|t| t == NOT_WILD)) {
        return true;
    }
    let Some(nbt) = e.nbt(host) else {
        return false;
    };
    nbt.contains_key(&"Owner".into()) || matches!(nbt.get(&"Tame".into()), Some(Tag::Byte(1)))
}

/// Whether a creature spawned for this reason isn't wild: hatched from a spawn egg, born of two parents, let out of a
/// bucket, from a spawner. One made by `/summon` still counts as wild.
fn tame_birth(reason: SpawnReason) -> bool {
    matches!(
        reason,
        SpawnReason::SpawnItemUse
            | SpawnReason::Breeding
            | SpawnReason::Bucket
            | SpawnReason::Spawner
            | SpawnReason::TrialSpawner
            | SpawnReason::Dispenser
    )
}

/// The products of `kind` that `how` gives.
fn products_by(kind: &str, how: impl Fn(&How) -> bool) -> Vec<String> {
    creature(kind).map_or_else(Vec::new, |c| {
        c.products
            .iter()
            .filter(|p| p.how.iter().any(&how))
            .map(|p| format!("product.{}", p.fact))
            .collect()
    })
}

/// The creatures bigger than the book's view takes by default, as vanilla's `EntityType` sizes them (width, height in blocks): the
/// book's 3D view fits them (`creature::bestiary::view_size`). The others take its default.
const BIG: &[(&str, f32, f32)] = &[
    ("minecraft:ender_dragon", 16.0, 8.0),
    ("minecraft:ghast", 4.0, 4.0),
    ("minecraft:wither", 0.9, 3.5),
    ("minecraft:enderman", 0.6, 2.9),
    ("minecraft:warden", 0.9, 2.9),
    ("minecraft:iron_golem", 1.4, 2.7),
    ("minecraft:creaking", 0.9, 2.7),
    ("minecraft:wither_skeleton", 0.7, 2.4),
    ("minecraft:camel", 1.7, 2.375),
    ("minecraft:ravager", 1.95, 2.2),
    ("minecraft:elder_guardian", 1.9975, 1.9975),
    ("minecraft:llama", 0.9, 1.87),
    ("minecraft:trader_llama", 0.9, 1.87),
    ("minecraft:sniffer", 1.9, 1.75),
    ("minecraft:strider", 0.9, 1.7),
    ("minecraft:horse", 1.3965, 1.6),
    ("minecraft:skeleton_horse", 1.3965, 1.6),
    ("minecraft:zombie_horse", 1.3965, 1.6),
    ("minecraft:donkey", 1.3965, 1.5),
    ("minecraft:polar_bear", 1.4, 1.4),
    ("minecraft:hoglin", 1.3965, 1.4),
    ("minecraft:zoglin", 1.3965, 1.4),
    ("minecraft:panda", 1.3, 1.25),
    // Humanoids about two blocks tall, their heads and hats over their boxes.
    ("minecraft:witch", 0.6, 1.95),
    ("minecraft:villager", 0.6, 1.95),
    ("minecraft:zombie_villager", 0.6, 1.95),
    ("minecraft:wandering_trader", 0.6, 1.95),
    ("minecraft:pillager", 0.6, 1.95),
    ("minecraft:vindicator", 0.6, 1.95),
    ("minecraft:evoker", 0.6, 1.95),
    ("minecraft:zombie", 0.6, 1.95),
    ("minecraft:husk", 0.6, 1.95),
    ("minecraft:drowned", 0.6, 1.95),
    ("minecraft:skeleton", 0.6, 1.99),
    ("minecraft:stray", 0.6, 1.99),
    ("minecraft:bogged", 0.6, 1.99),
    ("minecraft:piglin", 0.6, 1.95),
    ("minecraft:piglin_brute", 0.6, 1.95),
    ("minecraft:zombified_piglin", 0.6, 1.95),
    ("minecraft:blaze", 0.6, 1.8),
    ("minecraft:breeze", 0.6, 1.77),
    ("minecraft:creeper", 0.6, 1.7),
];

/// In a placed jar a vanilla creature draws nothing (none is caught; the book draws it itself).
fn no_drawing(_: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {}

#[derive(Default)]
struct VanillaBestiary;

#[mod_api::sunrise_mod]
impl Hooks for VanillaBestiary {
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {
        for c in CREATURES {
            creature::bestiary::register(Box::leak(Box::new(species(c))));
        }
        for &(kind, width, height) in BIG {
            creature::bestiary::view_size(kind, width, height);
        }
    }

    /// One born tame (a spawn egg, a bucket, two parents, a spawner) is tagged before it spawns, so it never counts as
    /// met.
    fn finalize_spawn(&self, _: &dyn Host, event: &mut FinalizeSpawnEvent<'_>) -> Flow {
        if creature(event.kind).is_some() && tame_birth(event.reason) {
            let _ = event.edits.add_tag(event.mob, NOT_WILD);
        }
        Flow::Continue
    }

    /// One back from a save, or just born tame, is known by its tag.
    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if creature(event.kind).is_some() && saved_not_wild(host, event.entity) {
            NOT_WILD_IDS.lock().unwrap().insert(event.entity);
        }
        Flow::Continue
    }

    fn entity_leave_level(&self, _: &mut dyn ServerHost, event: &mut EntityLeaveLevelEvent<'_>) {
        NOT_WILD_IDS.lock().unwrap().remove(&event.entity);
        CREEPERS.lock().unwrap().remove(&event.entity);
    }

    /// Fed what breeds it: the player who fed it found out what it eats.
    fn entity_enter_love_mode(&self, host: &mut dyn ServerHost, event: &mut EntityEnterLoveModeEvent<'_>) {
        if let Some(player) = event.player {
            learn(host, &[player], event.kind, BREED);
        }
    }

    /// A baby born of two parents isn't wild.
    fn entity_breed(&self, host: &mut dyn ServerHost, event: &mut EntityBreedEvent) {
        if let Some(child) = event.child {
            mark(host, child);
        }
    }

    fn entity_tame(&self, host: &mut dyn ServerHost, event: &mut EntityTameEvent<'_>) {
        mark(host, event.entity);
        if let Some(owner) = event.owner {
            learn(host, &[owner], event.kind, TAME);
        }
    }

    /// An item used on it (a bucket on a cow, a bowl on a mooshroom).
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        let (Some(stack), Some(player)) = (event.stack, host.players().acting_player()) else {
            return None;
        };
        let item = creature::hand::id(&*host, stack);
        for fact in products_by(event.kind, |h| matches!(h, How::Used(i) if *i == item)) {
            learn(host, &[player.uuid()], event.kind, &fact);
        }
        None
    }

    fn player_bucket_entity(&self, host: &mut dyn ServerHost, event: &mut PlayerBucketEntityEvent<'_>) {
        for fact in products_by(event.entity, |h| matches!(h, How::Bucket)) {
            learn(host, &[event.player], event.entity, &fact);
        }
    }

    /// Shears or a brush on it (its products), or a player's kill (its drops, each item once).
    fn loot_drops(&self, host: &mut dyn ServerHost, event: &mut LootDropsEvent<'_>) {
        let (Some((_, kind)), Some(player)) = (event.context.entity, event.context.player) else {
            return;
        };
        if event.context.used {
            for fact in products_by(kind, |h| matches!(h, How::Tool)) {
                learn(host, &[player], kind, &fact);
            }
            return;
        }
        let items: BTreeSet<String> = event.drops.iter().map(|s| creature::hand::id(&*host, s)).collect();
        for item in items {
            learn(host, &[player], kind, &format!("drop.{item}"));
        }
    }

    /// What it drops on its own (an egg, a scute), or a piglin's barter, seen by the players near.
    fn entity_drop_item(&self, host: &mut dyn ServerHost, event: &mut EntityDropItemEvent<'_>) {
        let Some(e) = host.entity(event.entity) else {
            return;
        };
        let Some(kind) = e.kind(host).map(|k| k.id(host)) else {
            return;
        };
        let Some(c) = creature(&kind) else {
            return;
        };
        let Some(dimension) = e.level(host).and_then(|l| l.dimension().id(host)) else {
            return;
        };
        let players = near_at(host, &dimension, event.pos);
        for fact in products_by(&kind, |h| matches!(h, How::Drops(items) if items.contains(&event.item))) {
            learn(host, &players, &kind, &fact);
        }
        if matches!(c.special, Some(ref s) if matches!(s.seen, Seen::DropsItems)) {
            learn(host, &players, &kind, SPECIAL);
        }
    }

    /// Where the creepers are, for `explosion_prime`.
    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != "minecraft:creeper" {
                return;
            }
            if let Some(at) = host.entity(event.entity).and_then(|e| e.position(host)) {
                CREEPERS
                    .lock()
                    .unwrap()
                    .insert(event.entity, (at, event.dimension.to_owned()));
            }
        });
    }

    /// An explosion where a creeper stood: the players near saw it blow up.
    fn explosion_prime(&self, host: &mut dyn ServerHost, event: &mut ExplosionPrimeEvent) {
        let c = event.center;
        let creeper = CREEPERS
            .lock()
            .unwrap()
            .values()
            .find(|(at, _)| (0..3).map(|k| (at[k] - c[k]).powi(2)).sum::<f64>() < 4.0)
            .map(|(_, dimension)| dimension.clone());
        if let Some(dimension) = creeper {
            let players = near_at(host, &dimension, c);
            learn(host, &players, "minecraft:creeper", SPECIAL);
        }
    }

    /// An enderman taking or placing a block.
    fn entity_change_block(&self, host: &mut dyn ServerHost, event: &mut EntityChangeBlockEvent<'_>) {
        let seen = creature(event.kind)
            .is_some_and(|c| matches!(c.special, Some(ref s) if matches!(s.seen, Seen::ChangesBlock)));
        if seen {
            let players = near(host, event.entity);
            learn(host, &players, event.kind, SPECIAL);
        }
    }

    /// A creature turning into another (a zombie into a drowned, a zombie villager cured).
    fn living_conversion_post(&self, host: &mut dyn ServerHost, event: &mut LivingConversionPostEvent<'_>) {
        let seen = creature(event.kind).is_some_and(
            |c| matches!(c.special, Some(ref s) if matches!(s.seen, Seen::Becomes(k) if k == event.converted_kind)),
        );
        if seen {
            let players = near(host, event.converted);
            learn(host, &players, event.kind, SPECIAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_creature_born_tame_is_not_wild_but_a_summoned_one_is() {
        assert!(tame_birth(SpawnReason::SpawnItemUse));
        assert!(tame_birth(SpawnReason::Breeding));
        assert!(!tame_birth(SpawnReason::Natural));
        assert!(!tame_birth(SpawnReason::Command));
    }

    #[test]
    fn every_creature_is_vanilla_once_with_its_products_named() {
        let mut ids = std::collections::BTreeSet::new();
        for c in CREATURES {
            assert!(c.id.starts_with("minecraft:"), "{}", c.id);
            assert!(ids.insert(c.id), "{} twice", c.id);
            for p in c.products {
                assert!(!p.fact.is_empty() && p.item.contains(':'), "{}: {}", c.id, p.item);
            }
        }
        assert!(CREATURES.len() >= 80);
    }
}
