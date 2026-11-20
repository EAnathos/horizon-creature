//! Jellyfish (SPECS.md): stinging jellyfish drifting in blooms, of the Sunrise Creatures series.
//!
//! A jellyfish is a copy of the wandering trader drawn with its own model (`model.rs`, from
//! `sources/models/jellyfish.bbmodel`), one texture per kind: moon jelly, sea nettle, glow jelly, its kinds being its
//! looks (the library's `colours`, rolled by biome). It lives in water through the library's `aquatic` (never drowning,
//! beached out of water, caught in a water bucket, spawning in blooms) and swims by hand (`swim.rs`). What touches it is
//! stung, as its kind stings (the library's `touch`), unless they sit in a boat or wear a turtle shell. The glow jelly
//! glows: full-bright (`emissive`) with a soft cyan light around it (the library's `glow`).
mod model;
mod swim;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use creature::aquatic::{Aquatic, Spawning};
use creature::bestiary::{Fact, Species};
use creature::colours::{Colour, Look, Pattern, Scheme, Variety};
use mod_api::{
    BlockEntityDrawContext, ClientHost, EmissiveEvent, EntityInteractEvent, EntityRenderState, EntityRenderer,
    EntityTickPostEvent, EquipmentSlot, Flow, Hooks, HostSide, InteractionResult, LivingDeathEvent,
    RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost,
};

pub const KIND: &str = "jellyfish:jellyfish";
const EGG: &str = "jellyfish:jellyfish_spawn_egg";
const JELLY: &str = "jellyfish:jelly";
const GLOW_JELLY: &str = "jellyfish:glow_jelly";
const BUCKET: &str = "jellyfish:jellyfish_bucket";

/// Its kinds, as indices into `SCHEME.patterns` (the morph saved and drawn), and their textures.
const MOON: usize = 0;
const NETTLE: usize = 1;
const GLOW: usize = 2;
const TEXTURE: [&str; 3] = [
    "jellyfish:entity/jellyfish/moon_jelly",
    "jellyfish:entity/jellyfish/sea_nettle",
    "jellyfish:entity/jellyfish/glow_jelly",
];

/// Its box (blocks, SPECS §1) and the scale its model (21 texels tall, its bell 11 across) is drawn at.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.7,
    height: 0.9,
    eye_height: 0.7,
};
const SCALE: f32 = 0.75;

/// Where each kind lives (SPECS §2).
const OCEAN: &[&str] = &["minecraft:ocean", "minecraft:cold_ocean"];
const WARM: &[&str] = &["minecraft:lukewarm_ocean", "minecraft:warm_ocean"];
const DEEP: &[&str] = &[
    "minecraft:deep_ocean",
    "minecraft:deep_lukewarm_ocean",
    "minecraft:deep_cold_ocean",
];
const BIOMES: &[&str] = &[
    "minecraft:ocean",
    "minecraft:cold_ocean",
    "minecraft:lukewarm_ocean",
    "minecraft:warm_ocean",
    "minecraft:deep_ocean",
    "minecraft:deep_lukewarm_ocean",
    "minecraft:deep_cold_ocean",
];

static COLOURS: [Colour; 1] = [Colour {
    id: "plain",
    rgb: 0xFFFFFF,
}];
static PATTERNS: [Pattern; 3] = [
    Pattern {
        id: "moon_jelly",
        texture: Some(TEXTURE[MOON]),
        swatch: Some(0xC9B4F0),
    },
    Pattern {
        id: "sea_nettle",
        texture: Some(TEXTURE[NETTLE]),
        swatch: Some(0xE0A040),
    },
    Pattern {
        id: "glow_jelly",
        texture: Some(TEXTURE[GLOW]),
        swatch: Some(0x2050C0),
    },
];
static VARIETIES: [Variety; 4] = [
    Variety {
        biomes: OCEAN,
        base: &[(0, 1)],
        patterns: &[(MOON, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: WARM,
        base: &[(0, 1)],
        patterns: &[(MOON, 1), (NETTLE, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: DEEP,
        base: &[(0, 1)],
        patterns: &[(GLOW, 1)],
        pattern_colours: &[(0, 1)],
    },
    // Everywhere else (one let out of a bucket in a river, say): a moon jelly.
    Variety {
        biomes: &[],
        base: &[(0, 1)],
        patterns: &[(MOON, 1)],
        pattern_colours: &[(0, 1)],
    },
];
static SCHEME: Scheme = Scheme {
    colours: &COLOURS,
    patterns: &PATTERNS,
    base_texture: TEXTURE[MOON],
    varieties: &VARIETIES,
};

/// The kind of jellyfish `id` (a moon jelly until its look is known).
fn kind(id: u32) -> usize {
    creature::colours::look(id).map_or(MOON, |l| l.pattern)
}

/// In water through the library (SPECS §3): dried out after a minute on land, caught in its bucket.
static AQUATIC: Aquatic = Aquatic {
    kind: KIND,
    bucket: BUCKET,
    dries: 1200,
};

/// Blooms of 3 to 8 in its oceans, 2 to 6 blocks under the surface (SPECS §3).
static SPAWNING: Spawning = Spawning {
    kind: KIND,
    biomes: BIOMES,
    group: (3, 8),
    cap: 10,
    chance: 3,
    depth: (2.0, 6.0),
};

const BLOOM_FACT: &str = "bloom";
const STING_FACT: &str = "sting";
const GLOW_FACT: &str = "glow";

/// Its page (SPECS §4): met as its kinds (morphs), its facts shown as they are learnt.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: BIOMES,
    wild: |_| true,
    facts: &[
        Fact {
            id: BLOOM_FACT,
            item: "minecraft:kelp",
            key: "bestiary.jellyfish.bloom",
        },
        Fact {
            id: STING_FACT,
            item: JELLY,
            key: "bestiary.jellyfish.sting",
        },
        Fact {
            id: GLOW_FACT,
            item: GLOW_JELLY,
            key: "bestiary.jellyfish.glow",
        },
        Fact {
            id: creature::aquatic::BUCKET_FACT,
            item: BUCKET,
            key: "bestiary.jellyfish.bucket",
        },
    ],
    icon: EGG,
    scheme: Some(&SCHEME),
    rarity: creature::bestiary::Rarity::Common,
    // Never in a glass jar (SPECS §6): it needs water.
    draw: |_, _| {},
    draw_scale: 1.0,
    draw_lift: 0.0,
};

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static JELLY_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: JELLY,
    dropped_by: &[KIND],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:slime_ball",
        key: "bestiary.jellyfish.item.jelly.use",
    }],
};
static GLOW_JELLY_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: GLOW_JELLY,
    dropped_by: &[KIND],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:glow_ink_sac",
        key: "bestiary.jellyfish.item.glow_jelly.use",
    }],
};
static BUCKET_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: BUCKET,
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:water_bucket",
        key: "bestiary.jellyfish.item.jellyfish_bucket.use",
    }],
};

/// The glow jelly's light: soft cyan, seen only.
static LIGHT: creature::glow::Light = creature::glow::Light {
    rgb: 0x5FE6F5,
    radius: 4.0,
    intensity: 0.8,
    lift: 0.5,
    shown: |_| true,
};

/// What touches it is stung at most once a second (SPECS §3), within this reach of its bell (blocks).
const STING_EVERY: i32 = 20;
const STING_REACH: f64 = 0.45;

/// Where each jellyfish is, for its blooms (single player).
static AT: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());
/// A bloom: this many within `BLOOM_REACH` blocks of one another.
const BLOOM: usize = 5;
const BLOOM_REACH: f64 = 6.0;
/// How often the facts seen are looked for (ticks).
const FACTS_EVERY: i32 = 40;

/// `target` stung by jellyfish `id`, as its kind stings (SPECS §2): unless they sit in a boat or wear a turtle shell.
fn sting(host: &mut dyn ServerHost, id: u32, target: u32) {
    let Some(t) = host.entity(target) else {
        return;
    };
    let in_boat = t.vehicle(&*host).and_then(|v| v.kind(&*host)).is_some_and(|k| {
        let k = k.id(&*host);
        k.ends_with("boat") || k.ends_with("raft")
    });
    let shelled = host
        .living(t)
        .and_then(|l| l.equipment(&*host, EquipmentSlot::Head))
        .is_some_and(|s| creature::hand::is(&*host, &s, "minecraft:turtle_helmet"));
    if in_boat || shelled {
        return;
    }
    let me = host.entity(id);
    match kind(id) {
        NETTLE => {
            t.hurt(host, "minecraft:mob_attack", 2.0, me, me);
            creature::mob::give_effect(host, target, "minecraft:poison", 80);
        }
        GLOW => {
            t.hurt(host, "minecraft:mob_attack", 1.0, me, me);
            creature::mob::give_effect(host, target, "minecraft:glowing", 200);
        }
        _ => {
            t.hurt(host, "minecraft:mob_attack", 1.0, me, me);
        }
    }
    if let Some(player) = creature::mob::player_of(&*host, target) {
        creature::bestiary::learn(host, player, KIND, STING_FACT);
    }
}

/// From `entity_tick_post`, for each jellyfish: its life in water, its look, its stings, its light, the facts seen,
/// and despawning far from players.
fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    let wet = creature::aquatic::tick(host, id, KIND);
    creature::colours::tick(host, id, &SCHEME);
    let glows = kind(id) == GLOW;
    creature::body::tick(host, id, || vec![Box::new(swim::Swim::new(glows))]);
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    AT.lock().unwrap().insert(id, at);
    let middle = [at[0], at[1] + 0.6, at[2]];
    for target in creature::touch::due(&*host, level, middle, KIND, id, STING_REACH, STING_EVERY) {
        sting(host, id, target);
    }
    if glows && wet {
        creature::glow::shine(&*host, dimension, id, LIGHT);
    } else {
        creature::glow::dim(id);
    }
    if e.tick_count(&*host).unwrap_or(0) % FACTS_EVERY == 0 {
        facts(host, dimension, id, at, glows);
    }
    if creature::mob::despawn_far(host, dimension, id, false) {
        forget(id);
    }
}

/// The facts the players near jellyfish `id` see: a bloom, a glow jelly at night.
fn facts(host: &mut dyn ServerHost, dimension: &str, id: u32, at: [f64; 3], glows: bool) {
    let players = creature::bestiary::players_near(&*host, dimension, at);
    if players.is_empty() {
        return;
    }
    let bloom = AT
        .lock()
        .unwrap()
        .values()
        .filter(|p| creature::body::distance(**p, at) < BLOOM_REACH)
        .count()
        >= BLOOM;
    let night = host
        .level(dimension)
        .and_then(|l| l.day_time(&*host))
        .is_some_and(|t| t.rem_euclid(24_000) >= 12_500);
    for player in players {
        if bloom {
            creature::bestiary::learn(host, player, KIND, BLOOM_FACT);
        }
        if glows && night {
            creature::bestiary::learn(host, player, KIND, GLOW_FACT);
        }
    }
    let _ = id;
}

/// Its drops (SPECS §3): 0 to 2 jelly, glow jelly from a glow jelly.
fn drops(host: &mut dyn ServerHost, id: u32) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    let count = creature::colours::Random::new(id ^ 0x5EED).next() % 3;
    if count == 0 {
        return;
    }
    let item = if kind(id) == GLOW { GLOW_JELLY } else { JELLY };
    if let Some(stack) = host.item_from_snbt(&format!("{{id:\"{item}\",count:{count}}}")) {
        creature::hand::drop_stack(host, level, at, stack);
    }
}

fn forget(id: u32) {
    creature::body::forget(id);
    creature::aquatic::forget(id);
    creature::glow::dim(id);
    swim::forget(id);
    AT.lock().unwrap().remove(&id);
}

/// How it holds itself this frame: its tentacles and arms swaying, flared out while it pulses.
fn pose(name: &str, age: f32, pulse: f32) -> [f32; 3] {
    let n: f32 = name
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split('_')
        .next()
        .and_then(|d| d.parse().ok())
        .unwrap_or(0.0);
    let sway = |k: f32| 8.0 * (age * 0.08 + n * 0.9 + k).sin();
    let flare = 30.0 * pulse;
    if name.starts_with("tentacle_") {
        // Around the rim: tentacle n points out at n twelfths of a turn.
        let angle = n / 12.0 * std::f32::consts::TAU;
        let (dx, dz) = (angle.cos(), angle.sin());
        let bend = if name.ends_with("_end") || name.ends_with("_mid") {
            0.5
        } else {
            1.0
        };
        [dz * flare * bend + sway(0.0), 0.0, -dx * flare * bend + sway(1.3)]
    } else if name.starts_with("arm_") {
        [sway(0.4) * 0.6, 0.0, sway(2.1) * 0.6]
    } else {
        [0.0; 3]
    }
}

/// Its own model, in its kind's texture.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let look: Look = creature::colours::look(entity.id).unwrap_or_default();
        let texture = TEXTURE.get(look.pattern).copied().unwrap_or(TEXTURE[MOON]);
        let pulse = swim::pulse(entity.id);
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let pose = |part: &str| pose(part, entity.age, pulse);
        creature::model::draw(draw, &model::JELLYFISH, texture, &pose);
        let _ = mob;
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Jellyfish;

#[mod_api::sunrise_mod]
impl Hooks for Jellyfish {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(KIND, SIZE);
        creature::aquatic::register(&AQUATIC);
        creature::aquatic::spawns(&SPAWNING);
        let bucket = r.item_key(BUCKET);
        r.item_part(bucket, creature::aquatic::Bucket);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&JELLY_PAGE);
        creature::bestiary::register_item(&GLOW_JELLY_PAGE);
        creature::bestiary::register_item(&BUCKET_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Right-click: nothing (no trading screen, which its template would open), on both sides; a water bucket is the
    /// library's (it catches it).
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == KIND).then_some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind == KIND {
                tick(host, event.dimension, event.entity);
            }
        });
    }

    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            drops(host, event.entity);
            forget(event.entity);
        }
        Flow::Continue
    }

    /// The glow jelly glows: its whole texture is full-bright.
    fn emissive(&self, _: &dyn ClientHost, event: &mut EmissiveEvent<'_>) {
        event.layers.push((TEXTURE[GLOW], TEXTURE[GLOW]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_have_their_textures() {
        assert_eq!(SCHEME.patterns.len(), TEXTURE.len());
        for (k, p) in SCHEME.patterns.iter().enumerate() {
            assert_eq!(p.texture, Some(TEXTURE[k]));
        }
    }

    #[test]
    fn a_tentacle_flares_only_while_it_pulses() {
        let still = pose("tentacle_3", 0.0, 0.0);
        let flared = pose("tentacle_3", 0.0, 1.0);
        assert_ne!(still, flared);
        assert_eq!(pose("bell", 10.0, 1.0), [0.0; 3]);
    }
}
