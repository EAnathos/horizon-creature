//! Scorpions (SPECS.md): real-size scorpions of deserts, badlands and caves, of the Sunrise Creatures series.
//!
//! One entity (`scorpions:scorpion`), a copy of the wandering trader (its behaviour is all goals), with three kinds as
//! morphs (the library's colours, `SCHEME`: desert, badlands, a glowing cave kind), each a texture. It is drawn small
//! (a fifth of its built size) with its own model (`model_scorpion.rs`, from `assets/models/scorpion.bbmodel`).
//!
//! - It crawls slowly on the ground (the library's small crawlers, `creature::crawl`), its tail low.
//! - **Defensive posture** (`posture.rs`): a player or a mob that is not ours coming within `POSTURE` blocks makes it
//!   rear up, claws open and tail arched, and back away; a sneaking player may come to `POSTURE_SNEAK`. Stepped on, it
//!   stings once (`posture.rs`).
//! - **It fights back** (`fight.rs`): hit, it chases whoever hit it and strikes, a pinch or a poisoned sting, calming
//!   after a while.
//! - Caught in the library's glass jar; released on sand it settles (the library's sand rule). Its drops (a venom sac,
//!   a stinger) are a loot table (content).
//!
//! Its poison is applied from the mod's `entity_tick_post` (the goals queue a sting, `STINGS`, since a goal can reach a
//! mob but not a player's effects; `Host::add_effect` needs a player, as the colours' single-player statics).
mod fight;
mod model_scorpion;
mod posture;

use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::bestiary::{Fact, Rarity, Species};
use creature::colours::{Colour, Look, Pattern, Scheme, Variety};
use creature::jar::Catchable;
use creature::territory::{self, Rule};
use horizon_client::mobs::goals::{Float, HurtByTarget};
use horizon_client::mod_api::{
    BlockEntityDraw, EmissiveEvent, EntityTickPostEvent, Flow, Hooks, Host, InteractEntityEvent, LivingDeathEvent,
    MobSubmitEvent, NativeEffect, ParticleOptions, ParticleType, RegisterGoalsEvent, Tag,
};

pub const SCORPION: &str = "scorpions:scorpion";

/// Its kinds, as indices into `SCHEME.patterns` (the morph saved and drawn); their textures.
const DESERT: usize = 0;
const BADLANDS: usize = 1;
const CAVE: usize = 2;
const TEXTURE: [&str; 3] = [
    "scorpions:entity/scorpion/desert",
    "scorpions:entity/scorpion/badlands",
    "scorpions:entity/scorpion/cave",
];
/// The cave kind glows (its edges full-bright, as scorpions glow under ultraviolet light): a glow texture laid over the
/// cave one (`emissive`).
const CAVE_GLOW: &str = "scorpions:entity/scorpion/cave_glow";

/// Its box (blocks, SPECS §1) and the scale its model is drawn at (a fifth, about 0.2 block long).
const SIZE: (f64, f64, f64) = (0.25, 0.15, 0.25);
const SCALE: f32 = 0.2;

/// Where each kind lives (SPECS §2), by biome. The cave kind's "under deserts and badlands, below y 40" is left to
/// balance: for now it is the cave biome's kind, the surface kinds their biomes', a desert scorpion everywhere else.
const DESERT_BIOMES: &[&str] = &["minecraft:desert"];
const BADLANDS_BIOMES: &[&str] = &[
    "minecraft:badlands",
    "minecraft:wooded_badlands",
    "minecraft:eroded_badlands",
];
const CAVE_BIOMES: &[&str] = &["minecraft:dripstone_caves"];

/// Every biome a scorpion spawns in (its bestiary page and `content/.../biome_modifier/`).
const BIOMES: &[&str] = &[
    "minecraft:desert",
    "minecraft:badlands",
    "minecraft:wooded_badlands",
    "minecraft:eroded_badlands",
    "minecraft:dripstone_caves",
];

/// The three kinds, one morph each: a plain (untinted) base, a pattern (the kind's full texture) per biome group.
static COLOURS: [Colour; 1] = [Colour {
    id: "plain",
    rgb: 0xFFFFFF,
}];
static PATTERNS: [Pattern; 3] = [
    Pattern {
        id: "desert",
        texture: Some(TEXTURE[DESERT]),
    },
    Pattern {
        id: "badlands",
        texture: Some(TEXTURE[BADLANDS]),
    },
    Pattern {
        id: "cave",
        texture: Some(TEXTURE[CAVE]),
    },
];
static VARIETIES: [Variety; 4] = [
    Variety {
        biomes: DESERT_BIOMES,
        base: &[(0, 1)],
        patterns: &[(DESERT, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: BADLANDS_BIOMES,
        base: &[(0, 1)],
        patterns: &[(BADLANDS, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: CAVE_BIOMES,
        base: &[(0, 1)],
        patterns: &[(CAVE, 1)],
        pattern_colours: &[(0, 1)],
    },
    // Everywhere else: a desert scorpion.
    Variety {
        biomes: &[],
        base: &[(0, 1)],
        patterns: &[(DESERT, 1)],
        pattern_colours: &[(0, 1)],
    },
];
static SCHEME: Scheme = Scheme {
    colours: &COLOURS,
    patterns: &PATTERNS,
    base_texture: TEXTURE[DESERT],
    varieties: &VARIETIES,
};

/// The kind of a scorpion from its saved look (the pattern index), desert when it has none yet.
fn kind(id: u32) -> usize {
    creature::colours::look(id).map_or(DESERT, |l| l.pattern)
}

/// Caught in the library's glass jar, settling on a sandy patch (the sand rule, SPECS §6).
static IN_JAR: Catchable = Catchable {
    kind: SCORPION,
    model: "scorpions:scorpion_jar",
    settles: Some(Rule::Sand),
    glows: false,
};

/// Its sounds (`assets/sounds.toml`, to cut with `cargo creature sounds` once the clips are downloaded): a dry hiss as
/// it threatens, hurt and death. Silent until then; the ids are harmless when no `sounds.json` names them.
const HISS: &str = "scorpions:entity.scorpion.hiss";
const VOICE: creature::sounds::Voice = creature::sounds::Voice {
    hurt: "scorpions:entity.scorpion.hurt",
    death: "scorpions:entity.scorpion.death",
};

/// Its page (SPECS §4): met as its kinds (morphs), its facts shown as they are learnt.
static BESTIARY: Species = Species {
    id: SCORPION,
    biomes: BIOMES,
    wild: |id| !territory::settled(id),
    facts: &[
        Fact {
            id: POSTURE_FACT,
            item: "minecraft:spider_eye",
            key: "bestiary.scorpions.posture",
        },
        Fact {
            id: STUNG_FACT,
            item: "scorpions:venom_sac",
            key: "bestiary.scorpions.stung",
        },
        Fact {
            id: GLOW_FACT,
            item: "minecraft:glow_ink_sac",
            key: "bestiary.scorpions.glow",
        },
        Fact {
            id: creature::jar::JAR_FACT,
            item: creature::jar::ITEM,
            key: "bestiary.scorpions.jar",
        },
        Fact {
            id: creature::jar::SAND_FACT,
            item: "minecraft:sand",
            key: "bestiary.scorpions.settled",
        },
    ],
    icon: "scorpions:scorpion_spawn_egg",
    scheme: Some(&SCHEME),
    rarity: Rarity::Uncommon,
    draw: book_view,
};

/// Bestiary facts learnt from the scorpion's own behaviour (the jar's two are the library's).
pub const POSTURE_FACT: &str = "posture";
pub const STUNG_FACT: &str = "stung";
pub const GLOW_FACT: &str = "glow";

/// The bestiary's 3D view: at rest, scaled to about a block, in the morph picked.
fn book_view(host: &dyn Host, draw: &mut BlockEntityDraw, look: Option<Look>) {
    draw.push();
    draw.translate([0.5, 0.0, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    let look = look.unwrap_or_default();
    creature::colours::draw(
        host,
        draw,
        &model_scorpion::SCORPION,
        &SCHEME,
        look,
        &|_| [0.0; 3],
        creature::model::WHITE,
    );
    draw.pop();
}
const BOOK_SCALE: f32 = 2.4;

/// How a scorpion holds itself for the drawing, by its mood (`MOODS`): reared (tail arched high, claws open) when it
/// threatens or fights, its tail low otherwise. Single player, as the colours' render statics (arismoko/horizon#2740).
static MOODS: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());
/// Mob victims of the dagger's venom: ticks of Poison remaining. The pinned native API can damage any mob but exposes
/// potion-effect mutation only for players, so we reproduce Poison's health ticks and particles for mob targets.
static VENOM: Mutex<BTreeMap<u32, u8>> = Mutex::new(BTreeMap::new());
const VENOM_TICKS: u8 = 60;
const VENOM_INTERVAL: u8 = 25;

fn set_reared(id: u32, reared: bool) {
    MOODS.lock().unwrap().insert(id, reared);
}
fn reared(id: u32) -> bool {
    MOODS.lock().unwrap().get(&id).copied().unwrap_or(false)
}

/// The pose of a part (degrees): the tail curls higher over the back and the claw fingers open when reared; the tail
/// lies low otherwise. The model is built with the tail curled, so a small extra lift reads as the defensive arch.
fn pose(part: &str, reared: bool) -> [f32; 3] {
    let tail = part.strip_prefix("tail_").and_then(|n| n.parse::<f32>().ok());
    if let Some(i) = tail {
        // Each segment turns up a little; more when reared, lowered when calm.
        let up = if reared { 10.0 } else { -6.0 };
        return [up - i * 0.5, 0.0, 0.0];
    }
    if part == "telson" {
        return [if reared { 14.0 } else { -8.0 }, 0.0, 0.0];
    }
    if reared && (part == "claw_left_finger" || part == "claw_right_finger") {
        // The moving finger opens.
        let open = if part.ends_with("left_finger") { 22.0 } else { -22.0 };
        return [0.0, open, 0.0];
    }
    [0.0; 3]
}

/// A sting's poison to apply (`STINGS`): the stinging scorpion's kind. A goal reaches a mob but not a player's effects
/// (`Host::add_effect` needs a player), so the goal queues the kind and `entity_tick_post` poisons the player stung.
/// Single player, as the colours' render statics (arismoko/horizon#2740): the poison lands on the one player.
pub(crate) static STINGS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

pub(crate) fn queue_sting(kind: usize) {
    STINGS.lock().unwrap().push(kind);
}

/// From `entity_tick_post`: the poison of each queued sting, on the player. Desert and badlands poison 5 s; the cave
/// kind poisons 8 s and dazes (Nausea) 4 s (SPECS §2).
fn apply_stings(host: &mut dyn Host) {
    let stings: Vec<usize> = std::mem::take(&mut *STINGS.lock().unwrap());
    for kind in stings {
        let (poison, nausea) = if kind == CAVE { (160, 80) } else { (100, 0) };
        for player in host.server_players() {
            host.add_effect(player.uuid, &poison_effect("minecraft:poison", poison));
            if nausea > 0 {
                host.add_effect(player.uuid, &poison_effect("minecraft:nausea", nausea));
            }
        }
    }
}

fn poison_effect(id: &str, duration: i32) -> NativeEffect {
    NativeEffect {
        id: id.into(),
        amplifier: 0,
        duration,
        ambient: false,
        particles: true,
    }
}

/// The pinned API has no mob-effect setter. Keep a short poison timer for dagger-hit mobs, inflict Poison's periodic
/// one-health damage without killing them, and show the familiar green motes. Vanilla Potion effects remain used for
/// player victims through `Host::add_effect` where that hook is available.
fn venom_tick(host: &mut dyn Host, event: &EntityTickPostEvent<'_>) {
    let (remaining, damage) = {
        let mut venom = VENOM.lock().unwrap();
        let Some(left) = venom.get_mut(&event.entity) else { return };
        *left = left.saturating_sub(1);
        let remaining = *left;
        let damage = remaining > 0 && remaining % VENOM_INTERVAL == 0;
        if remaining == 0 {
            venom.remove(&event.entity);
        }
        (remaining, damage)
    };

    if damage
        && let Some(entity) = host.entity(event.entity)
        && let Some(nbt) = host.entity_nbt(entity)
        && let Some(Tag::Float(bits)) = nbt.get(&"Health".into())
    {
        let health = f32::from_bits(*bits);
        if health > 1.0 {
            host.set_mob_health(event.dimension, event.entity, (health - 1.0).max(1.0));
        }
    }

    if remaining > 0 && remaining % 4 == 0
        && let Some(entity) = host.entity(event.entity)
        && let (Some(at), Some(level)) = (host.entity_position(entity), host.entity_level(entity))
    {
        let mut motes = ParticleOptions::simple(ParticleType::Dust);
        motes.color = [0.25, 0.8, 0.15, 1.0];
        motes.scale = 0.7;
        level.send_particles(host, &motes, None, at, [0.25, 0.35, 0.25], 0.0, 1);
    }
}

pub fn hooks() -> Box<dyn Hooks> {
    creature::jar::register(&IN_JAR);
    creature::bestiary::register(&BESTIARY);
    Box::new(Scorpions)
}

struct Scorpions;

impl Hooks for Scorpions {
    /// A dagger hit starts three seconds of venom on a mob. The native damage hook is reached only for successful,
    /// non-invulnerable player melee hits and includes the weapon snapshot.
    fn living_incoming_damage_v2(&self, _: &mut dyn Host, event: &mut horizon_client::mod_api::LivingIncomingDamageV2Event<'_>) -> Flow {
        if event.damage_type == "minecraft:player_attack"
            && event.weapon.as_ref().is_some_and(|weapon| weapon.item.as_ref() == "scorpions:scorpion_dagger")
            && let Ok(entity) = u32::try_from(event.entity)
        {
            VENOM.lock().unwrap().insert(entity, VENOM_TICKS);
        }
        Flow::Continue
    }
    /// Its row in Options > Mods, which switches it off and on at once (`creature::list_mod`).
    fn mods(&self, _: &mut dyn Host, event: &mut horizon_client::mod_api::ModsEvent<'_>) {
        creature::list_mod(
            event,
            "scorpions",
            concat!("Scorpions ", env!("CARGO_PKG_VERSION")),
            "Real-size scorpions of deserts, badlands and caves. Needs Sunrise Creatures.",
        );
    }

    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id != SCORPION {
            return;
        }
        creature::mob::ours(event.mob, SIZE.0, SIZE.1, SIZE.2);
        event.mob.ai.goals.add(0, Float);
        event.mob.ai.goals.add(1, fight::Fight::default());
        event.mob.ai.goals.add(2, posture::Posture::default());
        event.mob.ai.goals.add(3, creature::crawl::Crawl::new(&CRAWL_CFG));
        // It fights back whoever hits it (SPECS §3): the vanilla "hurt by" target goal sets its target.
        event.mob.ai.targets.add(1, HurtByTarget::new());
    }

    /// Right-click: nothing (no trading screen, which its template would open).
    fn interact_entity(&self, _: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if event.kind == SCORPION {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        venom_tick(host, event);
        if event.kind != SCORPION {
            return;
        }
        creature::colours::tick(host, event.entity, &SCHEME);
        creature::sounds::tick_post(host, event, &VOICE, creature::sounds::NEUTRAL);
        territory::tick(host, event.entity);
        apply_stings(host);
        if creature::mob::despawn_far(host, event.dimension, event.entity, territory::settled(event.entity)) {
            forget(event.entity);
        }
    }

    fn living_death(&self, _: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        VENOM.lock().unwrap().remove(&event.entity);
        if creature::mob::kind(event.kind) == SCORPION {
            forget(event.entity);
        }
        Flow::Continue
    }

    /// The cave kind's full-bright glow: the glow texture laid over the cave one (only cave scorpions draw it).
    fn emissive(&self, _: &dyn Host, event: &mut EmissiveEvent<'_>) {
        event.layers.push((TEXTURE[CAVE], CAVE_GLOW));
    }

    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != SCORPION {
            return Flow::Continue;
        }
        let look = creature::colours::look(mob.id).unwrap_or_default();
        let reared = reared(mob.id);
        event.draw.push();
        creature::model::death_tilt(&mut *event.draw, mob);
        event.draw.scale([SCALE; 3]);
        creature::colours::draw(
            host,
            &mut *event.draw,
            &model_scorpion::SCORPION,
            &SCHEME,
            look,
            &|part| pose(part, reared),
            creature::model::tint(mob),
        );
        event.draw.pop();
        Flow::Cancel
    }
}

/// Crawling about (the library's small crawlers): a few blocks around where it first crawled, or over its sandy patch
/// when settled (the sand rule), its tail low.
const CRAWL_CFG: creature::crawl::Cfg = creature::crawl::Cfg {
    speed: 0.012,
    crawl: (40, 140),
    pause: (60, 220),
    wander: 2.5,
    check_every: 100,
    settled_area: territory::check_sand,
    on_pause: None,
};

/// Forget a gone scorpion in every static that holds it.
fn forget(entity: u32) {
    territory::forget(entity);
    MOODS.lock().unwrap().remove(&entity);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_is_its_own_morph_and_the_cave_one_is_known() {
        assert_eq!(SCHEME.patterns.len(), 3);
        assert_eq!(PATTERNS[CAVE].id, "cave");
        assert_eq!(TEXTURE[CAVE], "scorpions:entity/scorpion/cave");
        // A desert scorpion when nothing is saved yet.
        assert_eq!(kind(0), DESERT);
    }

    #[test]
    fn the_cave_kind_poisons_longer_and_dazes() {
        // Stings queued for a cave and a desert scorpion; the mood pose arches the tail when reared.
        assert!(pose("telson", true)[0] > pose("telson", false)[0]);
        assert_ne!(pose("claw_left_finger", true), [0.0; 3]);
        assert_eq!(pose("claw_left_finger", false), [0.0; 3]);
    }
}
