//! Flying Fish (SPECS.md): shoals of flying fish that burst out of warm seas and glide over the waves, of the Sunrise
//! Creatures series.
//!
//! A flying fish is a copy of the wandering trader drawn with its own model (`model.rs`, from
//! `sources/models/flying_fish.bbmodel`), one texture per kind: blue wing, spotted wing, sunset wing, its kinds being
//! its looks (the library's `colours`, rolled by biome, one kind a shoal). It lives in water through the library's
//! `aquatic` (never drowning, beached out of water, caught in a water bucket, spawning in shoals), swims and glides by
//! hand (`swim.rs`), and is hunted by dolphins (the library's `prey`). A beached one is picked up as a raw flying fish.
//! Its drops are its loot table's.
mod model;
mod swim;

use std::sync::Arc;

use creature::aquatic::{Aquatic, Spawning};
use creature::bestiary::{Fact, Species};
use creature::colours::{Colour, Look, Pattern, Scheme, Variety};
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, Hooks,
    HostSide, InteractionResult, LivingDeathEvent, RegisterEntityRenderersEvent, Registrar, RegistrationContext,
    ServerHost, SoundSource,
};

pub const KIND: &str = "flying_fish:flying_fish";
const EGG: &str = "flying_fish:flying_fish_spawn_egg";
const RAW: &str = "flying_fish:raw_flying_fish";
const COOKED: &str = "flying_fish:cooked_flying_fish";
const BUCKET: &str = "flying_fish:flying_fish_bucket";

/// Its kinds, as indices into `SCHEME.patterns` (the morph saved and drawn), and their textures.
const BLUE: usize = 0;
const SPOTTED: usize = 1;
const SUNSET: usize = 2;
const TEXTURE: [&str; 3] = [
    "flying_fish:entity/flying_fish/blue_wing",
    "flying_fish:entity/flying_fish/spotted_wing",
    "flying_fish:entity/flying_fish/sunset_wing",
];

/// Its box (blocks, SPECS §1); its model (12 texels long) is drawn at full scale, about three quarters of a block.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.5,
    height: 0.3,
    eye_height: 0.2,
};

/// Where each kind lives (SPECS §2).
const LUKEWARM: &[&str] = &["minecraft:lukewarm_ocean", "minecraft:deep_lukewarm_ocean"];
const WARM: &[&str] = &["minecraft:warm_ocean"];
const BIOMES: &[&str] = &[
    "minecraft:lukewarm_ocean",
    "minecraft:deep_lukewarm_ocean",
    "minecraft:warm_ocean",
];

static COLOURS: [Colour; 1] = [Colour {
    id: "plain",
    rgb: 0xFFFFFF,
}];
static PATTERNS: [Pattern; 3] = [
    Pattern {
        id: "blue_wing",
        texture: Some(TEXTURE[BLUE]),
        swatch: Some(0x8CC8F0),
    },
    Pattern {
        id: "spotted_wing",
        texture: Some(TEXTURE[SPOTTED]),
        swatch: Some(0x7A8040),
    },
    Pattern {
        id: "sunset_wing",
        texture: Some(TEXTURE[SUNSET]),
        swatch: Some(0xF08A30),
    },
];
static VARIETIES: [Variety; 3] = [
    Variety {
        biomes: LUKEWARM,
        base: &[(0, 1)],
        patterns: &[(BLUE, 1), (SPOTTED, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: WARM,
        base: &[(0, 1)],
        patterns: &[(BLUE, 9), (SUNSET, 1)],
        pattern_colours: &[(0, 1)],
    },
    // Everywhere else (one let out of a bucket in a river, say): a blue wing.
    Variety {
        biomes: &[],
        base: &[(0, 1)],
        patterns: &[(BLUE, 1)],
        pattern_colours: &[(0, 1)],
    },
];
static SCHEME: Scheme = Scheme {
    colours: &COLOURS,
    patterns: &PATTERNS,
    base_texture: TEXTURE[BLUE],
    varieties: &VARIETIES,
};

/// In water through the library (SPECS §3): dried out after 30 s on land, caught in its bucket.
static AQUATIC: Aquatic = Aquatic {
    kind: KIND,
    bucket: BUCKET,
    dries: 600,
    flop: Some(FLOP),
};

/// Its sounds (`sources/sounds.toml`, cut from CC0 freesound recordings): its splash out of and into the water, the buzz
/// of its fins gliding and its tail beating on a wave (`swim.rs`), its flop on land; hurt and death are its entity
/// type's.
pub(crate) const SPLASH: &str = "flying_fish:entity.flying_fish.splash";
pub(crate) const GLIDE: &str = "flying_fish:entity.flying_fish.glide";
pub(crate) const BEAT: &str = "flying_fish:entity.flying_fish.beat";
const FLOP: &str = "flying_fish:entity.flying_fish.flop";

/// Shoals of 4 to 10 in its oceans, just under the surface (SPECS §3).
static SPAWNING: Spawning = Spawning {
    kind: KIND,
    scheme: Some(&SCHEME),
    biomes: BIOMES,
    group: (4, 10),
    cap: 12,
    chance: 3,
    depth: (0.5, 2.5),
};

const GLIDE_FACT: &str = "glide";
const SHOAL_FACT: &str = "shoal";
const DOLPHIN_FACT: &str = "dolphin";
const ROD_FACT: &str = "rod";

/// Dolphins eat them (SPECS §3): one within 12 blocks swims after it.
static PREY: creature::prey::Prey = creature::prey::Prey {
    kind: KIND,
    hunters: &[creature::prey::Hunter {
        kind: "minecraft:dolphin",
        range: 12.0,
        speed: 1.5,
    }],
    fact: Some(DOLPHIN_FACT),
};

/// A dolphin that caught one is fed, as by a player (SPECS §3): it leads the players near to treasure.
fn fed(host: &mut dyn ServerHost, dolphin: u32) {
    let mut nbt = mod_api::Compound::new();
    nbt.insert("GotFish".into(), mod_api::Tag::Byte(1));
    if let Some(d) = host.entity(dolphin) {
        d.merge_nbt(host, nbt);
    }
}

/// Its page (SPECS §4): met as its kinds (morphs), its facts shown as they are learnt.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: BIOMES,
    wild: |_| true,
    facts: &[
        Fact {
            id: GLIDE_FACT,
            item: "minecraft:feather",
            key: "bestiary.flying_fish.glide",
        },
        Fact {
            id: SHOAL_FACT,
            item: RAW,
            key: "bestiary.flying_fish.shoal",
        },
        Fact {
            id: DOLPHIN_FACT,
            item: "minecraft:heart_of_the_sea",
            key: "bestiary.flying_fish.dolphin",
        },
        Fact {
            id: ROD_FACT,
            item: "minecraft:fishing_rod",
            key: "bestiary.flying_fish.rod",
        },
        Fact {
            id: creature::aquatic::BUCKET_FACT,
            item: BUCKET,
            key: "bestiary.flying_fish.bucket",
        },
    ],
    icon: EGG,
    scheme: Some(&SCHEME),
    rarity: creature::bestiary::Rarity::Common,
    // Never in a glass jar: it needs water.
    draw: |_, _| {},
    draw_scale: 1.0,
    draw_lift: 0.0,
};

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static RAW_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: RAW,
    dropped_by: &[KIND],
    given_by: &[],
    uses: &[
        creature::items::Use {
            icon: "minecraft:cod",
            key: "bestiary.flying_fish.item.raw_flying_fish.use",
        },
        creature::items::Use {
            icon: EGG,
            key: "bestiary.flying_fish.item.raw_flying_fish.beached",
        },
        creature::items::Use {
            icon: "minecraft:fishing_rod",
            key: "bestiary.flying_fish.item.raw_flying_fish.fished",
        },
    ],
};
static COOKED_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: COOKED,
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:cooked_cod",
        key: "bestiary.flying_fish.item.cooked_flying_fish.use",
    }],
};
static BUCKET_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: BUCKET,
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:water_bucket",
        key: "bestiary.flying_fish.item.flying_fish_bucket.use",
    }],
};

/// How often it looks out for dolphins and boats (ticks), and how near they frighten it (blocks).
const LOOK_EVERY: i32 = 10;
const THREAT: f64 = 6.0;
/// How often the facts seen are looked for (ticks).
const FACTS_EVERY: i32 = 20;

/// Whether `kind` frightens a flying fish: a dolphin or a boat.
fn threat(kind: &str) -> bool {
    kind == "minecraft:dolphin" || kind.ends_with("boat") || kind.ends_with("raft")
}

/// From `entity_tick_post`, for each flying fish: its life in water, its look, its swim and glides, the dolphins and
/// boats frightening it, the facts seen, and despawning far from players.
fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    if creature::prey::tick(host, dimension, id, KIND) {
        forget(id);
        return;
    }
    creature::aquatic::tick(host, id, KIND);
    creature::colours::tick(host, id, &SCHEME);
    creature::body::tick(host, id, || vec![Box::new(swim::Fish::default())]);
    creature::sounds::play_queued(host, id, SoundSource::Neutral);
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    let age = e.tick_count(&*host).unwrap_or(0);
    if (age + id as i32) % LOOK_EVERY == 0 {
        let from = [at[0] - THREAT, at[1] - THREAT, at[2] - THREAT];
        let to = [at[0] + THREAT, at[1] + THREAT, at[2] + THREAT];
        let near = level
            .entities_in(&*host, from, to)
            .unwrap_or_default()
            .into_iter()
            .filter(|t| t.kind(&*host).is_some_and(|k| threat(&k.id(&*host))))
            .find_map(|t| t.position(&*host));
        if let Some(from) = near {
            swim::scare(at, from);
        }
    }
    if swim::gliding(id) {
        bumps(host, id, at);
    }
    if age % FACTS_EVERY == 0 {
        facts(host, dimension, id, at);
    }
    if creature::mob::despawn_far(host, dimension, id, false) {
        forget(id);
    }
}

/// How near a glider runs into a player or a mob (blocks from its middle).
const BUMP: f64 = 0.5;

/// Gliding flying fish `id` at `at` running into a player or a mob (not one of its own, nor a spectator): it bounces off.
fn bumps(host: &mut dyn ServerHost, id: u32, at: [f64; 3]) {
    let Some(level) = host.entity(id).and_then(|e| e.level(&*host)) else {
        return;
    };
    let middle = [at[0], at[1] + 0.15, at[2]];
    let from = [middle[0] - BUMP, middle[1] - BUMP, middle[2] - BUMP];
    let to = [middle[0] + BUMP, middle[1] + BUMP, middle[2] + BUMP];
    let hit = level
        .entities_in(&*host, from, to)
        .unwrap_or_default()
        .into_iter()
        .filter(|t| t.id() != id && host.living(*t).is_some())
        .filter(|t| t.kind(&*host).is_none_or(|k| k.id(&*host) != KIND))
        .filter(|t| {
            creature::mob::player_of(&*host, t.id())
                .and_then(|p| host.player(p))
                .is_none_or(|p| !p.is_spectator(&*host).unwrap_or(false))
        })
        .find_map(|t| t.position(&*host));
    if let Some(there) = hit {
        swim::bump(id, at, there);
    }
}

/// Fishing with a rod in its oceans (SPECS §5): a cod caught there is a raw flying fish two times in five, about as often
/// as a salmon. The player learns the rod fact.
fn fished(host: &mut dyn ServerHost, event: &mut mod_api::ItemFishedEvent<'_>) {
    let Some(level) = host.player(event.player).and_then(|p| p.entity().level(&*host)) else {
        return;
    };
    let p = event.fished.position;
    let biome = level.biome_at(&*host, [p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32]);
    if !biome.is_some_and(|b| BIOMES.contains(&b.as_str())) {
        return;
    }
    let mut roll = creature::colours::Random::new(event.fished.hook ^ 0xF15F);
    let mut caught = false;
    for stack in event.drops.iter_mut() {
        if creature::hand::is(&*host, stack, "minecraft:cod") && roll.next() % 5 < 2 {
            let count = stack.count();
            if let Some(fish) = host.item_from_snbt(&format!("{{id:\"{RAW}\",count:{count}}}")) {
                *stack = fish;
                caught = true;
            }
        }
    }
    if caught {
        creature::bestiary::learn(host, event.player, KIND, ROD_FACT);
    }
}

/// The facts the players near flying fish `id` see: a long glide, a shoal taking off.
fn facts(host: &mut dyn ServerHost, dimension: &str, id: u32, at: [f64; 3]) {
    let mut seen = Vec::new();
    if swim::glided(id) {
        seen.push((at, GLIDE_FACT));
    }
    for there in swim::took_off() {
        seen.push((there, SHOAL_FACT));
    }
    for (there, fact) in seen {
        for player in creature::bestiary::players_near(&*host, dimension, there) {
            creature::bestiary::learn(host, player, KIND, fact);
        }
    }
}

fn forget(id: u32) {
    creature::body::forget(id);
    creature::aquatic::forget(id);
    swim::forget(id);
}

/// A beached flying fish picked up by hand (SPECS §3): it is gone, a raw flying fish in the player's hands.
fn pick_up(host: &mut dyn ServerHost, id: u32) -> bool {
    let Some(e) = host.entity(id) else {
        return false;
    };
    if e.is_in_water(&*host).unwrap_or(true) || swim::gliding(id) {
        return false;
    }
    let (Some(player), Some(at), Some(level)) = (host.players().acting_player(), e.position(&*host), e.level(&*host))
    else {
        return false;
    };
    let Some(fish) = host.item_from_snbt(&format!("{{id:\"{RAW}\",count:1}}")) else {
        return false;
    };
    if !e.discard(host) {
        return false;
    }
    creature::colours::forget(id);
    forget(id);
    creature::hand::give(host, player, fish);
    level.play_sound(
        host,
        "minecraft:entity.item.pickup",
        at,
        SoundSource::Players,
        0.3,
        1.4,
        None,
    );
    true
}

/// How it holds itself this frame: fins folded along its body as it swims, its tail beating; fins spread to glide,
/// its tail's lower lobe quivering.
fn pose(name: &str, age: f32, gliding: bool) -> [f32; 3] {
    let beat = |speed: f32, k: f32| (age * speed + k).sin();
    match (name, gliding) {
        ("fin_left", false) => [0.0, 70.0, -25.0],
        ("fin_right", false) => [0.0, -70.0, 25.0],
        ("pelvic_left", false) => [0.0, 50.0, -15.0],
        ("pelvic_right", false) => [0.0, -50.0, 15.0],
        ("fin_left", true) => [0.0, 0.0, 3.0 * beat(1.7, 0.0)],
        ("fin_right", true) => [0.0, 0.0, -3.0 * beat(1.7, 0.0)],
        ("tail", false) => [0.0, 15.0 * beat(0.6, 0.0), 0.0],
        ("tail_fin", false) => [0.0, 15.0 * beat(0.6, -0.8), 0.0],
        ("tail_fin", true) => [0.0, 6.0 * beat(2.5, 0.0), 0.0],
        _ => [0.0; 3],
    }
}

/// Its own model, in its kind's texture, pitched as it climbs or dives.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let look: Look = creature::colours::look(entity.id).unwrap_or_default();
        let texture = TEXTURE.get(look.pattern).copied().unwrap_or(TEXTURE[BLUE]);
        let gliding = swim::gliding(entity.id);
        draw.push();
        creature::model::living_pose(draw, entity);
        if let Some(m) = mob {
            draw.rotate_about([1.0, 0.0, 0.0], m.x_rot, [0.0, 0.125, 0.0]);
        }
        let pose = |part: &str| pose(part, entity.age, gliding);
        // Tinted as the game tints a mob: red while it is hurt.
        let all = |_: &str| true;
        creature::model::draw_shown(
            draw,
            &model::FLYING_FISH,
            texture,
            &pose,
            &all,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct FlyingFish;

#[mod_api::sunrise_mod]
impl Hooks for FlyingFish {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(KIND, SIZE);
        creature::aquatic::register(&AQUATIC);
        creature::aquatic::spawns(&SPAWNING);
        creature::prey::register(&PREY);
        creature::prey::when_caught(KIND, fed);
        let bucket = r.item_key(BUCKET);
        r.item_part(bucket, creature::aquatic::Bucket);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&RAW_PAGE);
        creature::bestiary::register_item(&COOKED_PAGE);
        creature::bestiary::register_item(&BUCKET_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Right-click: a beached one is picked up (on the server); never a trading screen, which its template would
    /// open, on both sides. A water bucket is the library's (its hook runs first: it catches it).
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != KIND {
            return None;
        }
        if let HostSide::Server(host) = host
            && let Ok(id) = u32::try_from(event.entity)
        {
            pick_up(host, id);
        }
        Some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind == KIND {
                tick(host, event.dimension, event.entity);
            }
        });
    }

    fn item_fished(&self, host: &mut dyn ServerHost, event: &mut mod_api::ItemFishedEvent<'_>) -> Flow {
        fished(host, event);
        Flow::Continue
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            forget(event.entity);
        }
        Flow::Continue
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
    fn its_fins_fold_to_swim_and_spread_to_glide() {
        assert_ne!(pose("fin_left", 0.0, false), pose("fin_left", 0.0, true));
        assert_eq!(pose("body", 3.0, true), [0.0; 3]);
        assert!(threat("minecraft:dolphin") && threat("minecraft:oak_boat") && !threat("minecraft:cod"));
    }
}
