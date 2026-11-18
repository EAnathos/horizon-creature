//! Butterflies (SPECS.md): butterflies and moths of the Sunrise Creatures series.
//!
//! So far (SPECS §0): the butterfly and the moth exist (spawn eggs), each a copy of the wandering trader (its goals
//! taken off by the library) with its own tiny size, drawn with its own model (`model_butterfly.rs`, `model_moth.rs`,
//! from `sources/models/`). A butterfly gets a look the first time it is seen (the library's colours
//! and patterns, common/SPECS.md §5): a base colour, a pattern and a pattern colour, more or less likely per biome
//! (`SCHEME`). The moth has one look, painted in its texture (SPECS §2.1). On the ground a butterfly opens and closes
//! its wings slowly. They fly by day (butterflies) or by night (moths) and land on flowers (the library's `flutter`); released
//! from a glass jar on a flower they settle around it (the library's territory). Butterfly dust drops from them.
mod model_butterfly;
mod model_moth;
mod throw;

use creature::colours::{Colour, Pattern, Scheme, Variety};
use creature::jar::Catchable;
use std::sync::Arc;

use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, Hooks,
    HostSide, InteractionResult, LivingDeathEvent, RegisterEntityRenderersEvent, Registrar, RegistrationContext,
    ServerHost,
};

pub const BUTTERFLY: &str = "butterflies:butterfly";
pub const MOTH: &str = "butterflies:moth";
const MOTH_TEXTURE: &str = "butterflies:entity/moth/moth";

/// Their size: a tiny box, in place of the trader's 0.6 × 1.95 (the library answers `entity_size`), and the scale their
/// models are drawn at.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.5,
    height: 0.3,
    eye_height: 0.2,
};
const SCALE: f32 = 0.6;

/// The butterfly's colours (SPECS §2), for its wings and its patterns.
const WHITE: usize = 0;
const LEMON: usize = 1;
const ORANGE: usize = 2;
const RED: usize = 3;
const SKY_BLUE: usize = 4;
const VIOLET: usize = 5;
const BROWN: usize = 6;
const BLACK: usize = 7;

static COLOURS: [Colour; 8] = [
    Colour {
        id: "white",
        rgb: 0xF4F1E8,
    },
    Colour {
        id: "lemon",
        rgb: 0xF2DE4C,
    },
    Colour {
        id: "orange",
        rgb: 0xEE8A2A,
    },
    Colour {
        id: "red",
        rgb: 0xC8322C,
    },
    Colour {
        id: "sky_blue",
        rgb: 0x5CB4EC,
    },
    Colour {
        id: "violet",
        rgb: 0x8C5CC8,
    },
    Colour {
        id: "brown",
        rgb: 0x8A5A34,
    },
    Colour {
        id: "black",
        rgb: 0x2A2624,
    },
];

/// Its patterns (SPECS §2), each a texture over the base one.
static PATTERNS: [Pattern; 6] = [
    Pattern {
        id: "plain",
        texture: None,
        swatch: None,
    },
    Pattern {
        id: "eyespots",
        texture: Some("butterflies:entity/butterfly/pattern_eyespots"),
        swatch: None,
    },
    Pattern {
        id: "tips",
        texture: Some("butterflies:entity/butterfly/pattern_tips"),
        swatch: None,
    },
    Pattern {
        id: "veins",
        texture: Some("butterflies:entity/butterfly/pattern_veins"),
        swatch: None,
    },
    Pattern {
        id: "dots",
        texture: Some("butterflies:entity/butterfly/pattern_dots"),
        swatch: None,
    },
    Pattern {
        id: "tails",
        texture: Some("butterflies:entity/butterfly/pattern_tails"),
        swatch: None,
    },
];

/// Every pattern, plain a little more often.
const ANY_PATTERN: &[(usize, u32)] = &[(0, 3), (1, 2), (2, 2), (3, 2), (4, 2), (5, 1)];
/// Patterns are mostly black, sometimes brown or white.
const PATTERN_COLOURS: &[(usize, u32)] = &[(BLACK, 8), (BROWN, 2), (WHITE, 2), (ORANGE, 1)];

/// Where each colour is likely (SPECS §2): bright in flower forests and meadows, white and lemon in plains, brown in
/// forests, blue and violet in jungles; a bit of everything elsewhere.
static VARIETIES: [Variety; 5] = [
    Variety {
        biomes: &[
            "minecraft:flower_forest",
            "minecraft:meadow",
            "minecraft:sunflower_plains",
            "minecraft:cherry_grove",
        ],
        base: &[
            (ORANGE, 3),
            (RED, 3),
            (LEMON, 2),
            (SKY_BLUE, 2),
            (VIOLET, 2),
            (WHITE, 1),
        ],
        patterns: ANY_PATTERN,
        pattern_colours: PATTERN_COLOURS,
    },
    Variety {
        biomes: &["minecraft:plains"],
        base: &[(WHITE, 4), (LEMON, 4), (ORANGE, 1)],
        patterns: ANY_PATTERN,
        pattern_colours: PATTERN_COLOURS,
    },
    Variety {
        biomes: &[
            "minecraft:forest",
            "minecraft:birch_forest",
            "minecraft:old_growth_birch_forest",
            "minecraft:dark_forest",
            "minecraft:taiga",
        ],
        base: &[(BROWN, 4), (ORANGE, 2), (WHITE, 1), (BLACK, 1)],
        patterns: ANY_PATTERN,
        pattern_colours: &[(BLACK, 6), (WHITE, 3), (ORANGE, 1)],
    },
    Variety {
        biomes: &["minecraft:jungle", "minecraft:sparse_jungle", "minecraft:bamboo_jungle"],
        base: &[(SKY_BLUE, 4), (VIOLET, 3), (RED, 1), (BLACK, 1)],
        patterns: ANY_PATTERN,
        pattern_colours: &[(BLACK, 6), (WHITE, 2), (LEMON, 1)],
    },
    Variety {
        biomes: &[],
        base: &[(WHITE, 2), (LEMON, 2), (ORANGE, 2), (BROWN, 2), (RED, 1), (SKY_BLUE, 1)],
        patterns: ANY_PATTERN,
        pattern_colours: PATTERN_COLOURS,
    },
];

/// How they fly (the library's `flutter`, SPECS §2, §2.1): a butterfly by day, landing on flowers; a moth at night,
/// drawn to light.
static BUTTERFLY_FLIGHT: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Day,
    speed: 0.16,
    height: (1.0, 3.5),
    flight: (80, 240),
    rest: (100, 600),
    wander: 6.0,
    drawn_to_light: false,
    lands_on_flowers: true,
    takeoff: Some("butterflies:entity.butterfly.flutter"),
};
static MOTH_FLIGHT: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Night,
    drawn_to_light: true,
    takeoff: Some("butterflies:entity.moth.flutter"),
    ..BUTTERFLY_FLIGHT
};

pub static SCHEME: Scheme = Scheme {
    colours: &COLOURS,
    patterns: &PATTERNS,
    base_texture: "butterflies:entity/butterfly/base",
    varieties: &VARIETIES,
};

/// Both are caught in the library's glass jar (common/SPECS.md §4.1), each with its own jar icon.
static IN_JAR: [Catchable; 2] = [
    Catchable {
        kind: BUTTERFLY,
        model: "butterflies:butterfly_jar",
        settles: Some(creature::territory::Rule::FlowerPatch),
        glows: false,
    },
    Catchable {
        kind: MOTH,
        model: "butterflies:moth_jar",
        settles: Some(creature::territory::Rule::FlowerPatch),
        glows: false,
    },
];

/// Their pages in the bestiary (common/SPECS.md §7): met wild (not settled) in the biomes they spawn in; caught in a jar.
const JAR_FACTS: &[creature::bestiary::Fact] = &[
    creature::bestiary::Fact {
        id: creature::jar::JAR_FACT,
        item: creature::jar::ITEM,
        key: "bestiary.butterflies.jar",
    },
    creature::bestiary::Fact {
        id: creature::jar::FLOWER_FACT,
        item: "minecraft:poppy",
        key: "bestiary.butterflies.flower",
    },
];

/// The bestiary's 3D view: the model at rest, scaled to about a block across; the butterfly in the morph picked, its
/// wings a little open.
fn butterfly_view(draw: &mut dyn BlockEntityDrawContext, look: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.1, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    let open = |part: &str| match part {
        "wing_left" => [0.0, 0.0, 20.0],
        "wing_right" => [0.0, 0.0, -20.0],
        _ => [0.0; 3],
    };
    let look = look.unwrap_or_default();
    creature::colours::draw(
        draw,
        &model_butterfly::BUTTERFLY,
        &SCHEME,
        look,
        &open,
        creature::model::WHITE,
    );
    draw.pop();
}

fn moth_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.1, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    creature::model::draw(draw, &model_moth::MOTH, MOTH_TEXTURE, &|_| [0.0; 3]);
    draw.pop();
}
const BOOK_SCALE: f32 = 1.2;

fn wild(id: u32) -> bool {
    !creature::territory::settled(id)
}

static BESTIARY: [creature::bestiary::Species; 2] = [
    creature::bestiary::Species {
        id: BUTTERFLY,
        biomes: BUTTERFLY_BIOMES,
        wild,
        facts: JAR_FACTS,
        icon: "butterflies:butterfly_spawn_egg",
        scheme: Some(&SCHEME),
        rarity: creature::bestiary::Rarity::Common,
        draw: butterfly_view,
    },
    creature::bestiary::Species {
        id: MOTH,
        biomes: MOTH_BIOMES,
        wild,
        facts: JAR_FACTS,
        icon: "butterflies:moth_spawn_egg",
        scheme: None,
        rarity: creature::bestiary::Rarity::Common,
        draw: moth_view,
    },
];

/// Where they spawn (`content/data/butterflies/neoforge/biome_modifier/`).
const BUTTERFLY_BIOMES: &[&str] = &[
    "minecraft:flower_forest",
    "minecraft:meadow",
    "minecraft:sunflower_plains",
    "minecraft:cherry_grove",
    "minecraft:plains",
    "minecraft:forest",
    "minecraft:birch_forest",
    "minecraft:jungle",
    "minecraft:sparse_jungle",
];
const MOTH_BIOMES: &[&str] = &[
    "minecraft:forest",
    "minecraft:birch_forest",
    "minecraft:dark_forest",
    "minecraft:taiga",
    "minecraft:plains",
    "minecraft:meadow",
];

/// They come out near players (the library's spawning: SPECS §2), by day for butterflies, at night for moths; butterflies
/// on every try (each 10 s), up to 18 near a player.
static SPAWNING: [creature::flutter::Spawning; 2] = [
    creature::flutter::Spawning {
        kind: BUTTERFLY,
        flight: &BUTTERFLY_FLIGHT,
        biomes: BUTTERFLY_BIOMES,
        group: (1, 3),
        cap: 18,
        chance: 1,
    },
    creature::flutter::Spawning {
        kind: MOTH,
        flight: &MOTH_FLIGHT,
        biomes: MOTH_BIOMES,
        group: (1, 3),
        cap: 5,
        chance: 3,
    },
];

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static DUST_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "butterflies:butterfly_dust",
    dropped_by: &["butterflies:butterfly", "butterflies:moth"],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "butterflies:butterfly_dust",
        key: "bestiary.butterflies.item.dust.throw",
    }],
};
static CONFETTI_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "butterflies:confetti",
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "butterflies:confetti",
        key: "bestiary.butterflies.item.confetti.pop",
    }],
};

#[derive(Default)]
struct Butterflies;

fn ours(kind: &str) -> bool {
    kind == BUTTERFLY || kind == MOTH
}

/// A group's rotation for this frame (degrees): on the ground a butterfly opens and closes its wings slowly, basking;
/// a moth rests with its wings flat.
fn pose(name: &str, id: u32, age: f32, butterfly: bool) -> [f32; 3] {
    let open = if creature::flutter::flying(id) {
        // Flying: quick beats, the wings meeting above it.
        if butterfly {
            30.0 + 50.0 * (age * 1.3).sin()
        } else {
            15.0 + 35.0 * (age * 1.6).sin()
        }
    } else if butterfly {
        // Resting: a butterfly opens and closes its wings slowly, a moth keeps them flat.
        35.0 + 35.0 * (age * 0.08).sin()
    } else {
        return [0.0; 3];
    };
    match name {
        "wing_left" => [0.0, 0.0, open],
        "wing_right" => [0.0, 0.0, -open],
        _ => [0.0; 3],
    }
}

/// Their own models: a butterfly in its look, a moth.
struct Draw {
    butterfly: bool,
}

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let (id, age, butterfly) = (entity.id, entity.age, self.butterfly);
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let tint = creature::model::tint(mob);
        if butterfly {
            let look = creature::colours::look(id).unwrap_or_default();
            let pose = |part: &str| pose(part, id, age, true);
            creature::colours::draw(draw, &model_butterfly::BUTTERFLY, &SCHEME, look, &pose, tint);
        } else {
            let pose = |part: &str| pose(part, id, age, false);
            creature::model::draw_shown(draw, &model_moth::MOTH, MOTH_TEXTURE, &pose, &|_| true, tint);
        }
        draw.pop();
        Ok(())
    }
}

#[mod_api::sunrise_mod]
impl Hooks for Butterflies {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::flutter::register(BUTTERFLY, SIZE);
        creature::flutter::register(MOTH, SIZE);
        creature::bestiary::register_item(&DUST_PAGE);
        creature::bestiary::register_item(&CONFETTI_PAGE);
        for s in &SPAWNING {
            creature::flutter::spawns(s);
        }
        for item in [throw::DUST, throw::CONFETTI] {
            let key = r.item_key(item);
            r.item_part(key, throw::Thrown);
        }
        for c in &IN_JAR {
            creature::jar::register(c);
        }
        for species in &BESTIARY {
            creature::bestiary::register(species);
        }
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event
            .renderers
            .register(BUTTERFLY.into(), Arc::new(Draw { butterfly: true }));
        event
            .renderers
            .register(MOTH.into(), Arc::new(Draw { butterfly: false }));
    }

    /// Right-click: nothing (no trading screen, which their template would open), on both sides.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        ours(event.kind).then_some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: &mut dyn ServerHost, event: &mut EntityTickPostEvent<'_>) {
        if event.kind == BUTTERFLY {
            creature::colours::tick(host, event.entity, &SCHEME);
        }
        if ours(event.kind) {
            let flight = if event.kind == MOTH {
                &MOTH_FLIGHT
            } else {
                &BUTTERFLY_FLIGHT
            };
            creature::flutter::tick_post(host, event.dimension, event.kind, event.entity, || {
                vec![Box::new(creature::flutter::Flutter::new(flight))]
            });
        }
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if ours(event.kind) {
            creature::flutter::gone(event.entity);
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creature::colours::{Random, from_key, key, roll};

    #[test]
    fn every_look_rolled_is_one_of_the_scheme_and_goes_through_its_key() {
        let biomes = [
            None,
            Some("minecraft:plains"),
            Some("minecraft:jungle"),
            Some("minecraft:flower_forest"),
        ];
        let mut random = Random::new(7);
        for biome in biomes {
            for _ in 0..200 {
                let look = roll(&SCHEME, biome, &mut random);
                assert!(look.base < SCHEME.colours.len() && look.pattern_colour < SCHEME.colours.len());
                assert!(look.pattern < SCHEME.patterns.len());
                assert_eq!(from_key(&SCHEME, &key(&SCHEME, look)), Some(look));
            }
        }
    }

    #[test]
    fn its_spawning_biomes_are_its_bestiary_s() {
        assert_eq!(SPAWNING[0].biomes, BUTTERFLY_BIOMES);
        assert_eq!(SPAWNING[1].biomes, MOTH_BIOMES);
    }
}
