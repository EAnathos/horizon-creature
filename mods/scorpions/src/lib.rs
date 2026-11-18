//! Scorpions (SPECS.md): real-size scorpions of deserts, badlands and caves, of the Sunrise Creatures series.
//!
//! One entity (`scorpions:scorpion`), a copy of the wandering trader (its behaviour is all goals), with three kinds as
//! morphs (the library's colours, `SCHEME`: desert, badlands, a glowing cave kind), each a texture. It is drawn small
//! (a fifth of its built size) with its own model (`model_scorpion.rs`, from `sources/models/scorpion.bbmodel`).
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
//! Its posture and its fight run from the server tick (the library's `body`) and queue their strikes (`strike`), dealt
//! with their poison by the mod's `entity_tick_post`.
mod dagger;
mod fight;
mod model_scorpion;
mod posture;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use creature::bestiary::{Fact, Rarity, Species};
use creature::colours::{Colour, Look, Pattern, Scheme, Variety};
use creature::jar::Catchable;
use creature::territory::{self, Rule};
use mod_api::{
    ActiveEffect, BlockEntityDrawContext, ClientHost, EmissiveEvent, EntityInteractEvent, EntityPart,
    EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, HitHost, Hooks, HostSide, InteractionResult,
    LivingDeathEvent, LivingIncomingDamageEvent, RegisterEntityRenderersEvent, Registrar, RegistrationContext,
    ServerHost, SoundSource,
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
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.25,
    height: 0.15,
    eye_height: 0.25,
};
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
        // Its body's colour, for the book's swatch.
        swatch: Some(0xC9A86A),
    },
    Pattern {
        id: "badlands",
        texture: Some(TEXTURE[BADLANDS]),
        // Its body's colour, for the book's swatch.
        swatch: Some(0x8A4529),
    },
    Pattern {
        id: "cave",
        texture: Some(TEXTURE[CAVE]),
        // Its body's colour, for the book's swatch.
        swatch: Some(0x2B2D33),
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

/// Its sounds (`sources/sounds.toml`, to cut with `cargo creature sounds` once the clips are downloaded): a dry hiss as
/// it threatens; hurt and death are its entity type's. Silent until then.
const HISS: &str = "scorpions:entity.scorpion.hiss";

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
fn book_view(draw: &mut dyn BlockEntityDrawContext, look: Option<Look>) {
    draw.push();
    draw.translate([0.5, 0.0, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    let look = look.unwrap_or_default();
    creature::colours::draw(
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
/// threatens or fights, its tail low otherwise..
static MOODS: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());
/// Mob victims of the dagger's venom: ticks of Poison remaining. The pinned native API can damage any mob but exposes
/// potion-effect mutation only for players, so we reproduce Poison's health ticks and particles for mob targets.
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

/// A strike its behaviours made (`posture`, `fight`): (scorpion, target, the kind whose poison a sting gives).
static STRIKES: Mutex<Vec<(u32, u32, Option<usize>)>> = Mutex::new(Vec::new());

/// Damage of a pinch or a sting (half a heart more than a fist).
const STRIKE_DAMAGE: f32 = 1.5;

/// From a behaviour: `scorpion` strikes `target`, with a sting of its kind's poison when `sting` says so.
pub(crate) fn strike(scorpion: u32, target: u32, sting: Option<usize>) {
    STRIKES.lock().unwrap().push((scorpion, target, sting));
}

/// From `entity_tick_post`: the strikes `scorpion` made, dealt. A sting poisons: desert and badlands 5 s; the cave
/// kind 8 s and dazes (Nausea) 4 s (SPECS §2).
fn apply_strikes(host: &mut dyn ServerHost, scorpion: u32) {
    let mine: Vec<(u32, u32, Option<usize>)> = {
        let mut all = STRIKES.lock().unwrap();
        let (mine, rest) = all.drain(..).partition(|s| s.0 == scorpion);
        *all = rest;
        mine
    };
    for (_, target, sting) in mine {
        let (Some(me), Some(t)) = (host.entity(scorpion), host.entity(target)) else {
            continue;
        };
        if let Some(l) = host.living(me) {
            l.swing(host, mod_api::Hand::Main, true);
        }
        t.hurt(host, "minecraft:mob_attack", STRIKE_DAMAGE, Some(me), Some(me));
        if let Some(kind) = sting {
            let (poison_ticks, nausea) = if kind == CAVE { (160, 80) } else { (100, 0) };
            poison(host, target, "minecraft:poison", poison_ticks);
            if nausea > 0 {
                poison(host, target, "minecraft:nausea", nausea);
            }
        }
    }
}

/// Gives `target` (a mob or a player) effect `id` for `duration` ticks.
pub(crate) fn poison(host: &mut dyn ServerHost, target: u32, id: &str, duration: i32) {
    let (Some(effect), Some(living)) = (host.effect_key(id), host.entity(target).and_then(|e| host.living(e))) else {
        return;
    };
    living.add_effect(host, &ActiveEffect::new(effect, duration, 0, false, true, true));
}

/// Its goals: crawling about, unless it rears up or fights (its `body` behaviours, `busy`).
struct Goals;

#[mod_api::sunrise_part]
impl EntityPart for Goals {
    fn register_goals(&self, goals: &mut mod_api::GoalRegistrar<'_>) {
        goals.add_goal(3, creature::crawl::Crawl::new(&CRAWL_CFG));
    }
}

/// Whether a scorpion rears up or fights now (its crawl waits).
fn busy(id: u32) -> bool {
    reared(id) || fight::target(id).is_some()
}

/// Its own model in its kind's look, reared or not.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let look = creature::colours::look(entity.id).unwrap_or_default();
        let reared = reared(entity.id);
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let pose = |part: &str| pose(part, reared);
        let tint = creature::model::tint(mob);
        creature::colours::draw(draw, &model_scorpion::SCORPION, &SCHEME, look, &pose, tint);
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Scorpions;

#[mod_api::sunrise_mod]
impl Hooks for Scorpions {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(SCORPION, SIZE);
        creature::projectile::register(dagger::THROWN, dagger::SIZE);
        r.entity_part(SCORPION, Goals);
        let dagger = r.item_key(dagger::DAGGER);
        let _ = dagger::KEY.set(dagger);
        r.item_part(dagger, dagger::Dagger);
        creature::jar::register(&IN_JAR);
        creature::bestiary::register(&BESTIARY);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(SCORPION.into(), Arc::new(Draw));
        event.renderers.register(dagger::THROWN.into(), Arc::new(dagger::Draw));
    }

    /// Whoever hits a scorpion is fought back; a melee hit with an item is noted, for the dagger's poison.
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        let Ok(entity) = u32::try_from(event.entity) else {
            return Flow::Continue;
        };
        if event.kind == SCORPION
            && let Ok(by) = u32::try_from(event.source.causing)
        {
            fight::provoke(entity, by);
        }
        if event.source.damage_type == "minecraft:player_attack"
            && let Some(weapon) = event.weapon
        {
            DAGGER_HITS.lock().unwrap().push((entity, weapon.clone()));
        }
        Flow::Continue
    }

    /// Right-click: nothing (no trading screen, which its template would open), on both sides.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == SCORPION || event.kind == dagger::THROWN).then_some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: &mut dyn ServerHost, event: &mut EntityTickPostEvent<'_>) {
        poison_dagger_hits(host);
        if event.kind == dagger::THROWN {
            dagger::tick(host, event.entity);
            return;
        }
        if event.kind != SCORPION {
            return;
        }
        let id = event.entity;
        creature::body::tick(host, id, || {
            vec![Box::new(fight::Fight::default()), Box::new(posture::Posture::default())]
        });
        apply_strikes(host, id);
        creature::colours::tick(host, id, &SCHEME);
        creature::sounds::play_queued(host, id, SoundSource::Hostile);
        territory::tick(host, id);
        if creature::mob::despawn_far(host, event.dimension, id, territory::settled(id)) {
            forget(id);
        }
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == SCORPION {
            forget(event.entity);
        } else if event.kind == dagger::THROWN {
            dagger::forget(event.entity);
        }
        Flow::Continue
    }

    /// The cave kind's full-bright glow: the glow texture laid over the cave one (only cave scorpions draw it).
    fn emissive(&self, _: &dyn ClientHost, event: &mut EmissiveEvent<'_>) {
        event.layers.push((TEXTURE[CAVE], CAVE_GLOW));
    }
}

/// Melee hits with an item, noted by the damage hook (which can't give effects): the dagger's poison their target.
static DAGGER_HITS: Mutex<Vec<(u32, mod_api::ItemStack)>> = Mutex::new(Vec::new());

fn poison_dagger_hits(host: &mut dyn ServerHost) {
    let hits: Vec<(u32, mod_api::ItemStack)> = std::mem::take(&mut *DAGGER_HITS.lock().unwrap());
    for (entity, weapon) in hits {
        if creature::hand::is(&*host, &weapon, dagger::DAGGER) {
            dagger::poison(host, entity);
        }
    }
}

/// Crawling about (the library's small crawlers): a few blocks around where it first crawled, or over its sandy patch
/// when settled (the sand rule), its tail low; not while it rears up or fights.
const CRAWL_CFG: creature::crawl::Cfg = creature::crawl::Cfg {
    speed: 1.0,
    crawl: (40, 140),
    pause: (60, 220),
    wander: 2.5,
    check_every: 100,
    settled_area: territory::check_sand,
    on_pause: None,
    busy: Some(busy),
};

/// Forget a gone scorpion in every static that holds it.
fn forget(entity: u32) {
    territory::forget(entity);
    creature::body::forget(entity);
    fight::forget(entity);
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
    fn a_reared_scorpion_arches_its_tail_and_opens_its_claws() {
        assert!(pose("telson", true)[0] > pose("telson", false)[0]);
        assert_ne!(pose("claw_left_finger", true), [0.0; 3]);
        assert_eq!(pose("claw_left_finger", false), [0.0; 3]);
    }
}
