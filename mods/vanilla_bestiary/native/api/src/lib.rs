//! Vanilla Bestiary (SPECS.md): Minecraft's own creatures in the creature guide's bestiary, of the Horizon Creature
//! series.
//!
//! Each creature of `creatures.rs` is registered with the library's bestiary (`creature::bestiary`), with the biomes,
//! drops and breeding food the game's data gives (`game_data.rs`, from `cargo creature vanilla`). Its page shows its
//! spawn egg turning where the series' creatures show their model (`creature::bestiary::draw_icon`). A player meets
//! one as the series' creatures, a wild one only: one hatched from a spawn egg, bred or tamed carries the tag
//! `NOT_WILD`. The facts are found out through the game's hooks: love mode (breeding), taming, an item used on it
//! (milk), shears or a brush (wool), a bucket, what it drops on its own (an egg), what it drops when a player kills it,
//! and one thing it does (a creeper exploding, an enderman taking a block, a zombie turning into a drowned).
mod creatures;
mod game_data;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use creature::bestiary::{Fact, Species};
use horizon_client::mod_api::{
    EntityBreedEvent, EntityChangeBlockEvent, EntityDropItemEvent, EntityEnterLoveModeEvent, EntityJoinLevelEvent,
    EntityLeaveLevelEvent, EntityTameEvent, EntityTickPostEvent, EntityTransformEvent, ExplosionPrimeEvent, Flow,
    Hooks, Host, InteractEntityEvent, LevelTickPostEvent, LootDropsEvent, PlayerBucketEntityEvent, Tag, UseItemEvent,
    UseItemOnEvent,
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
/// The creature a spawn egg used this tick hatches (`use_item_on`, `use_item`), until it joins the level.
static HATCHING: Mutex<Option<String>> = Mutex::new(None);
/// Where each creeper is, to tell which one an explosion was.
static CREEPERS: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());

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
        draw: creature::bestiary::draw_icon,
    }
}

pub fn hooks() -> Box<dyn Hooks> {
    for c in CREATURES {
        creature::bestiary::register(Box::leak(Box::new(species(c))));
    }
    Box::new(VanillaBestiary)
}

/// `players` found out `fact` about `kind`, when it is one of its facts.
fn learn(host: &mut dyn Host, players: &[u128], kind: &str, fact: &str) {
    let known = creature::bestiary::species_of(kind).is_some_and(|s| s.facts.iter().any(|f| f.id == fact));
    if !known {
        return;
    }
    for &player in players {
        creature::bestiary::learn(host, player, kind, fact);
    }
}

/// The players near entity `id`, in the level the hook runs in.
fn near(host: &dyn Host, id: u32) -> Vec<u128> {
    host.entity(id)
        .and_then(|e| host.entity_position(e))
        .map_or_else(Vec::new, |at| {
            creature::bestiary::players_near(host, host.dimension(), at)
        })
}

/// Creature `id` isn't wild: it no longer counts as met, now and once saved.
fn mark(host: &mut dyn Host, id: u32) {
    if let Some(e) = host.entity(id) {
        host.entity_add_tag(e, NOT_WILD);
    }
    NOT_WILD_IDS.lock().unwrap().insert(id);
}

/// Whether creature `id`, back from a save, isn't wild: tagged, or tamed before this mod was installed.
fn saved_not_wild(host: &mut dyn Host, id: u32) -> bool {
    let Some(e) = host.entity(id) else {
        return false;
    };
    if host
        .entity_tags(e)
        .is_some_and(|tags| tags.iter().any(|t| t == NOT_WILD))
    {
        return true;
    }
    let Some(nbt) = host.entity_nbt(e) else {
        return false;
    };
    nbt.contains_key(&"Owner".into()) || matches!(nbt.get(&"Tame".into()), Some(Tag::Byte(1)))
}

/// The creature a spawn egg in hand hatches (`minecraft:cow_spawn_egg` → `minecraft:cow`).
fn hatched_by(item: &str) -> Option<String> {
    item.strip_suffix("_spawn_egg")
        .filter(|kind| creature(kind).is_some())
        .map(str::to_owned)
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

struct VanillaBestiary;

impl Hooks for VanillaBestiary {
    /// Its row in Options > Mods, which switches it off and on at once (`creature::list_mod`).
    fn mods(&self, _: &mut dyn Host, event: &mut horizon_client::mod_api::ModsEvent<'_>) {
        creature::list_mod(
            event,
            "vanilla_bestiary",
            concat!("Vanilla Bestiary ", env!("CARGO_PKG_VERSION")),
            "Minecraft's own creatures in the creature guide's bestiary. Needs Horizon Creature.",
        );
    }

    /// A spawn egg in hand: the creature it hatches won't be wild.
    fn use_item_on(&self, _: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        *HATCHING.lock().unwrap() = hatched_by(&event.stack.item);
        Flow::Continue
    }

    /// A spawn egg used on water (a fish's).
    fn use_item(&self, _: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        *HATCHING.lock().unwrap() = hatched_by(&event.stack.item);
        Flow::Continue
    }

    fn level_tick_post(&self, _: &mut dyn Host, _: &mut LevelTickPostEvent<'_>) {
        HATCHING.lock().unwrap().take();
    }

    /// A creature hatched from a spawn egg is marked; one back from a save is known by its tag.
    fn entity_join_level(&self, host: &mut dyn Host, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if creature(event.kind).is_none() {
            return Flow::Continue;
        }
        if event.loaded_from_disk {
            if saved_not_wild(host, event.entity) {
                NOT_WILD_IDS.lock().unwrap().insert(event.entity);
            }
        } else if HATCHING.lock().unwrap().as_deref() == Some(event.kind) {
            HATCHING.lock().unwrap().take();
            mark(host, event.entity);
        }
        Flow::Continue
    }

    fn entity_leave_level(&self, _: &mut dyn Host, event: &mut EntityLeaveLevelEvent<'_>) {
        NOT_WILD_IDS.lock().unwrap().remove(&event.entity);
        CREEPERS.lock().unwrap().remove(&event.entity);
    }

    /// Fed what breeds it: the player who fed it found out what it eats.
    fn entity_enter_love_mode(&self, host: &mut dyn Host, event: &mut EntityEnterLoveModeEvent<'_>) {
        if let Some(player) = event.player {
            learn(host, &[player], event.kind, BREED);
        }
    }

    /// A baby born of two parents isn't wild.
    fn entity_breed(&self, host: &mut dyn Host, event: &mut EntityBreedEvent) {
        if let Some(child) = event.child {
            mark(host, child);
        }
    }

    fn entity_tame(&self, host: &mut dyn Host, event: &mut EntityTameEvent<'_>) {
        mark(host, event.entity);
        if let Some(owner) = event.owner {
            learn(host, &[owner], event.kind, TAME);
        }
    }

    /// An item used on it (a bucket on a cow, a bowl on a mooshroom).
    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        let (Some(stack), Some(player)) = (event.stack, host.acting_player()) else {
            return Flow::Continue;
        };
        for fact in products_by(event.kind, |h| matches!(h, How::Used(item) if *item == &*stack.item)) {
            learn(host, &[player.uuid], event.kind, &fact);
        }
        Flow::Continue
    }

    fn player_bucket_entity(&self, host: &mut dyn Host, event: &mut PlayerBucketEntityEvent<'_>) {
        for fact in products_by(event.entity, |h| matches!(h, How::Bucket)) {
            learn(host, &[event.player], event.entity, &fact);
        }
    }

    /// Shears or a brush on it (its products), or a player's kill (its drops, each item once).
    fn loot_drops(&self, host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        let (Some((_, kind)), Some(player)) = (event.context.entity, event.context.player) else {
            return;
        };
        if event.context.used {
            for fact in products_by(kind, |h| matches!(h, How::Tool)) {
                learn(host, &[player], kind, &fact);
            }
            return;
        }
        let items: BTreeSet<String> = event.drops.iter().map(|s| s.item.to_string()).collect();
        for item in items {
            learn(host, &[player], kind, &format!("drop.{item}"));
        }
    }

    /// What it drops on its own (an egg, a scute), or a piglin's barter, seen by the players near.
    fn entity_drop_item(&self, host: &mut dyn Host, event: &mut EntityDropItemEvent<'_>) {
        let Some(kind) = host.entity(event.entity).and_then(|e| host.entity_kind(e)) else {
            return;
        };
        let Some(c) = creature(&kind) else {
            return;
        };
        let players = creature::bestiary::players_near(host, host.dimension(), event.pos);
        for fact in products_by(&kind, |h| matches!(h, How::Drops(items) if items.contains(&event.item))) {
            learn(host, &players, &kind, &fact);
        }
        if matches!(c.special, Some(ref s) if matches!(s.seen, Seen::DropsItems)) {
            learn(host, &players, &kind, SPECIAL);
        }
    }

    /// Where the creepers are, for `explosion_prime`.
    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind != "minecraft:creeper" {
            return;
        }
        if let Some(at) = host.entity(event.entity).and_then(|e| host.entity_position(e)) {
            CREEPERS.lock().unwrap().insert(event.entity, at);
        }
    }

    /// An explosion where a creeper stood: the players near saw it blow up.
    fn explosion_prime(&self, host: &mut dyn Host, event: &mut ExplosionPrimeEvent) {
        let c = event.center;
        let creeper = CREEPERS
            .lock()
            .unwrap()
            .values()
            .any(|at| (0..3).map(|k| (at[k] - c[k]).powi(2)).sum::<f64>() < 4.0);
        if creeper {
            let players = creature::bestiary::players_near(host, host.dimension(), c);
            learn(host, &players, "minecraft:creeper", SPECIAL);
        }
    }

    /// An enderman taking or placing a block.
    fn entity_change_block(&self, host: &mut dyn Host, event: &mut EntityChangeBlockEvent<'_>) {
        let seen = creature(event.kind)
            .is_some_and(|c| matches!(c.special, Some(ref s) if matches!(s.seen, Seen::ChangesBlock)));
        if seen {
            let players = near(host, event.entity);
            learn(host, &players, event.kind, SPECIAL);
        }
    }

    /// A creature turning into another (a zombie into a drowned, a zombie villager cured).
    fn entity_transform(&self, host: &mut dyn Host, event: &mut EntityTransformEvent<'_>) {
        let seen = creature(event.kind).is_some_and(
            |c| matches!(c.special, Some(ref s) if matches!(s.seen, Seen::Becomes(k) if k == event.transformed_kind)),
        );
        if seen {
            let players = near(host, event.transformed);
            learn(host, &players, event.kind, SPECIAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spawn_egg_hatches_its_creature() {
        assert_eq!(hatched_by("minecraft:cow_spawn_egg").as_deref(), Some("minecraft:cow"));
        assert_eq!(hatched_by("minecraft:stick"), None);
        assert_eq!(hatched_by("minecraft:not_a_creature_spawn_egg"), None);
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
