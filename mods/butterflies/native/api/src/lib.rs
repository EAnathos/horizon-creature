//! Butterflies (SPECS.md): butterflies and moths of the Horizon Creature series.
//!
//! So far (SPECS §0): the butterfly and the moth exist (spawn eggs), each a copy of the wandering trader (its
//! behaviour is all goals, cleared here) with its own tiny size, drawn with its own model (`model_butterfly.rs`,
//! `model_moth.rs`, from `assets/models/`). A butterfly gets a look the first time it is seen (the library's colours
//! and patterns, common/SPECS.md §5): a base colour, a pattern and a pattern colour, more or less likely per biome
//! (`SCHEME`). The moth has one look, painted in its texture (SPECS §2.1). On the ground a butterfly opens and closes
//! its wings slowly. They fly by day (butterflies) or by night (moths) and land on flowers (the library's `flutter`); released
//! from a glass jar on a flower they settle around it (the library's territory). Butterfly dust drops from them.
mod model_butterfly;
mod model_moth;
mod throw;

use creature::colours::{Colour, Pattern, Scheme, Variety};
use creature::jar::Catchable;
use horizon_client::mod_api::{
    BlockEntityDraw, EntityTickPostEvent, Flow, Hooks, Host, InteractEntityEvent, LivingDeathEvent, MobSubmitEvent,
    NativeMobRender, RegisterGoalsEvent, UseItemEvent,
};

pub const BUTTERFLY: &str = "butterflies:butterfly";
pub const MOTH: &str = "butterflies:moth";
const MOTH_TEXTURE: &str = "butterflies:entity/moth/moth";

/// Their size: a tiny box, in place of the trader's 0.6 × 1.95 (a copy of its type, as the vulture; workaround for
/// horizon-rust/horizon#6276), and the scale their models are drawn at.
const WIDTH: f64 = 0.5;
const HEIGHT: f64 = 0.3;
const EYE_HEIGHT: f64 = 0.2;
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
    },
    Pattern {
        id: "eyespots",
        texture: Some("butterflies:entity/butterfly/pattern_eyespots"),
    },
    Pattern {
        id: "tips",
        texture: Some("butterflies:entity/butterfly/pattern_tips"),
    },
    Pattern {
        id: "veins",
        texture: Some("butterflies:entity/butterfly/pattern_veins"),
    },
    Pattern {
        id: "dots",
        texture: Some("butterflies:entity/butterfly/pattern_dots"),
    },
    Pattern {
        id: "tails",
        texture: Some("butterflies:entity/butterfly/pattern_tails"),
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
    night: false,
    speed: 0.16,
    height: (1.0, 3.5),
    flight: (80, 240),
    rest: (100, 600),
    wander: 6.0,
    drawn_to_light: false,
    lands_on_flowers: true,
};
static MOTH_FLIGHT: creature::flutter::Kind = creature::flutter::Kind {
    night: true,
    drawn_to_light: true,
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
fn butterfly_view(host: &dyn Host, draw: &mut BlockEntityDraw, look: Option<creature::colours::Look>) {
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
        host,
        draw,
        &model_butterfly::BUTTERFLY,
        &SCHEME,
        look,
        &open,
        creature::model::WHITE,
    );
    draw.pop();
}

fn moth_view(host: &dyn Host, draw: &mut BlockEntityDraw, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.1, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    creature::model::draw(host, draw, &model_moth::MOTH, MOTH_TEXTURE, &|_| [0.0; 3]);
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

pub fn hooks() -> Box<dyn Hooks> {
    creature::hand::server_used(throw::DUST);
    creature::hand::server_used(throw::CONFETTI);
    for c in &IN_JAR {
        creature::jar::register(c);
    }
    for species in &BESTIARY {
        creature::bestiary::register(species);
    }
    Box::new(Butterflies)
}

struct Butterflies;

fn ours(kind: &str) -> bool {
    kind == BUTTERFLY || kind == MOTH
}

/// A group's rotation for this frame (degrees): on the ground a butterfly opens and closes its wings slowly, basking;
/// a moth rests with its wings flat.
fn pose(name: &str, mob: &NativeMobRender<'_>, butterfly: bool) -> [f32; 3] {
    let open = if creature::flutter::flying(mob.id) {
        // Flying: quick beats, the wings meeting above it.
        if butterfly {
            30.0 + 50.0 * (mob.age * 1.3).sin()
        } else {
            15.0 + 35.0 * (mob.age * 1.6).sin()
        }
    } else if butterfly {
        // Resting: a butterfly opens and closes its wings slowly, a moth keeps them flat.
        35.0 + 35.0 * (mob.age * 0.08).sin()
    } else {
        return [0.0; 3];
    };
    match name {
        "wing_left" => [0.0, 0.0, open],
        "wing_right" => [0.0, 0.0, -open],
        _ => [0.0; 3],
    }
}

impl Hooks for Butterflies {
    /// Right-click with butterfly dust or confetti: thrown (`throw.rs`).
    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        if throw::thrown(host, event.stack) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if !ours(&event.mob.kind.id) {
            return;
        }
        let kind = if event.mob.kind.id == MOTH {
            &MOTH_FLIGHT
        } else {
            &BUTTERFLY_FLIGHT
        };
        creature::flutter::register(event.mob, (WIDTH, HEIGHT, EYE_HEIGHT), kind);
    }

    /// Right-click: nothing (no trading screen, which their template would open).
    fn interact_entity(&self, _: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if ours(event.kind) { Flow::Cancel } else { Flow::Continue }
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind == BUTTERFLY {
            creature::colours::tick(host, event.entity, &SCHEME);
        }
        if ours(event.kind) {
            creature::flutter::tick_post(host, event.dimension, event.entity);
        }
    }

    fn living_death(&self, _: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        if ours(event.kind) {
            creature::flutter::gone(event.entity);
        }
        Flow::Continue
    }

    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        let butterfly = mob.kind == BUTTERFLY;
        if !butterfly && mob.kind != MOTH {
            return Flow::Continue;
        }
        event.draw.push();
        creature::model::death_tilt(&mut *event.draw, mob);
        event.draw.scale([SCALE; 3]);
        let tint = creature::model::tint(mob);
        if butterfly {
            let look = creature::colours::look(mob.id).unwrap_or_default();
            creature::colours::draw(
                host,
                &mut *event.draw,
                &model_butterfly::BUTTERFLY,
                &SCHEME,
                look,
                &|part| pose(part, mob, true),
                tint,
            );
        } else {
            creature::model::draw_shown(
                host,
                &mut *event.draw,
                &model_moth::MOTH,
                MOTH_TEXTURE,
                &|part| pose(part, mob, false),
                &|_| true,
                tint,
            );
        }
        event.draw.pop();
        Flow::Cancel
    }
}
