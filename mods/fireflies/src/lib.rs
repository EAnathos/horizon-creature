//! Fireflies (SPECS.md): fireflies that light up the night, of the Sunrise Creatures series.
//!
//! A firefly is a copy of the wandering trader (its behaviour is all goals) with a tiny box, flying with the library's
//! `flutter`: out at night, drifting slowly and low, resting on the ground by day. It is drawn with its own model
//! (`model_firefly.rs`, from `sources/models/firefly.bbmodel`): its lantern pulses on its own rhythm (`lit`), lit with
//! a texture whose lantern is full-bright (`emissive`) and a soft visual light around it (`visuals`, up to 8). Caught
//! in the library's glass jar, released on a flower it settles there; a jar of fireflies on display gives light.
mod model_firefly;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use creature::bestiary::{Fact, Species};
use creature::jar::Catchable;
use mod_api::{
    BlockEntityDrawContext, ClientHost, EmissiveEvent, EntityInteractEvent, EntityRenderState, EntityRenderer,
    EntityTickPostEvent, Flow, Hooks, HostSide, InteractionResult, LivingDeathEvent, RegisterEntityRenderersEvent,
    Registrar, RegistrationContext, ServerHost,
};

pub const FIREFLY: &str = "fireflies:firefly";
const TEXTURE: &str = "fireflies:entity/firefly/firefly";
const LIT_TEXTURE: &str = "fireflies:entity/firefly/firefly_lit";
/// The lit texture's full-bright overlay: its lantern's pixels.
const GLOW: &str = "fireflies:entity/firefly/firefly_lit_glow";

/// Its box (blocks) and the scale its model is drawn at.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.3,
    height: 0.2,
    eye_height: 0.15,
};
const SCALE: f32 = 0.5;

/// How it flies (SPECS §2): at night, slow and low, long drifts and short rests; it doesn't seek flowers.
static FLIGHT: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Night,
    speed: 0.05,
    height: (0.5, 2.0),
    flight: (200, 600),
    rest: (60, 200),
    wander: 4.0,
    drawn_to_light: false,
    lands_on_flowers: false,
    takeoff: Some("fireflies:entity.firefly.buzz"),
};

/// Caught in the library's glass jar, settling on a flower, glowing in a jar on display (SPECS §3).
static IN_JAR: Catchable = Catchable {
    kind: FIREFLY,
    model: "fireflies:firefly_jar",
    settles: Some(creature::territory::Rule::FlowerPatch),
    glows: true,
};

/// Its page in the bestiary: met wild in the biomes it spawns in; caught in a jar, settling on a flower.
static BESTIARY: Species = Species {
    id: FIREFLY,
    biomes: &[
        "minecraft:swamp",
        "minecraft:mangrove_swamp",
        "minecraft:river",
        "minecraft:plains",
        "minecraft:meadow",
        "minecraft:forest",
        "minecraft:birch_forest",
        "minecraft:flower_forest",
    ],
    wild: |id| !creature::territory::settled(id),
    facts: &[
        Fact {
            id: creature::jar::JAR_FACT,
            item: creature::jar::ITEM,
            key: "bestiary.fireflies.jar",
        },
        Fact {
            id: creature::jar::FLOWER_FACT,
            item: "minecraft:poppy",
            key: "bestiary.fireflies.flower",
        },
    ],
    icon: "fireflies:firefly_spawn_egg",
    scheme: None,
    rarity: creature::bestiary::Rarity::Uncommon,
    draw: book_view,
    draw_scale: BOOK_SCALE,
    draw_lift: 0.1,
};

/// Its drawing in a placed jar: lit, its wings folded, about a block across.
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    let folded = |part: &str| !part.starts_with("wing");
    creature::model::draw_shown(
        draw,
        &model_firefly::FIREFLY,
        LIT_TEXTURE,
        &|_| [0.0; 3],
        &folded,
        creature::model::WHITE,
    );
}
const BOOK_SCALE: f32 = 1.6;

/// Whether firefly `id`'s lantern is lit now: each pulses on its own rhythm, lit about a second out of three.
fn lit(id: u32) -> bool {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let period = 2600 + u128::from(id.wrapping_mul(137) % 1200);
    (millis + u128::from(id.wrapping_mul(911))) % period < 900
}

/// The yellow-green light around a lit firefly flying (seen only, it lights nothing in the world).
static LIGHT: creature::glow::Light = creature::glow::Light {
    rgb: 0xD6FF6E,
    radius: 3.0,
    intensity: 0.8,
    lift: 0.1,
    shown: lit,
};

/// They come out near players at night, in small groups (the library's spawning: SPECS §2).
static SPAWNING: creature::flutter::Spawning = creature::flutter::Spawning {
    kind: FIREFLY,
    flight: &FLIGHT,
    biomes: BESTIARY.biomes,
    group: (3, 6),
    cap: 12,
    chance: 3,
};

#[derive(Default)]
struct Fireflies;

/// A group's rotation this frame: flying, the wing cases lift and the wings beat fast.
fn pose(name: &str, age: f32, flying: bool) -> [f32; 3] {
    if !flying {
        return [0.0; 3];
    }
    let beat = 25.0 + 30.0 * (age * 2.2).sin();
    match name {
        "elytra_left" => [-20.0, 0.0, 35.0],
        "elytra_right" => [-20.0, 0.0, -35.0],
        "wing_left" => [0.0, 0.0, beat],
        "wing_right" => [0.0, 0.0, -beat],
        _ => [0.0; 3],
    }
}

/// Its own model, its lantern pulsing in flight.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let flying = creature::flutter::flying(entity.id);
        let texture = if flying && lit(entity.id) { LIT_TEXTURE } else { TEXTURE };
        let shown = |part: &str| flying || !part.starts_with("wing");
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let pose = |part: &str| pose(part, entity.age, flying);
        creature::model::draw_shown(
            draw,
            &model_firefly::FIREFLY,
            texture,
            &pose,
            &shown,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

#[mod_api::sunrise_mod]
impl Hooks for Fireflies {
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {
        creature::flutter::register(FIREFLY, SIZE);
        creature::flutter::spawns(&SPAWNING);
        creature::jar::register(&IN_JAR);
        creature::bestiary::register(&BESTIARY);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(FIREFLY.into(), Arc::new(Draw));
    }

    /// Right-click: nothing (no trading screen, which its template would open), on both sides.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == FIREFLY).then_some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != FIREFLY {
                return;
            }
            creature::flutter::tick_post(host, event.dimension, event.kind, event.entity, || {
                vec![Box::new(creature::flutter::Flutter::new(&FLIGHT))]
            });
            if creature::flutter::flying(event.entity) {
                creature::glow::shine(host, event.dimension, event.entity, LIGHT);
            } else {
                creature::glow::dim(event.entity);
            }
        });
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == FIREFLY {
            creature::flutter::gone(event.entity);
            creature::glow::dim(event.entity);
        }
        Flow::Continue
    }

    /// The lit texture's lantern is full-bright.
    fn emissive(&self, _: &dyn ClientHost, event: &mut EmissiveEvent<'_>) {
        event.layers.push((LIT_TEXTURE, GLOW));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn they_spawn_where_their_bestiary_says_at_night() {
        assert_eq!(SPAWNING.biomes, BESTIARY.biomes);
        assert_eq!(FLIGHT.hours, creature::flutter::Hours::Night);
        assert!(SPAWNING.group.0 <= SPAWNING.group.1);
    }
}
