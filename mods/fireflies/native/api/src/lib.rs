//! Fireflies (SPECS.md): fireflies that light up the night, of the Horizon Creature series.
//!
//! A firefly is a copy of the wandering trader (its behaviour is all goals) with a tiny box, flying with the library's
//! `flutter`: out at night, drifting slowly and low, resting on the ground by day. It is drawn with its own model
//! (`model_firefly.rs`, from `assets/models/firefly.bbmodel`): its lantern pulses on its own rhythm (`lit`), lit with
//! a texture whose lantern is full-bright (`emissive`) and a soft visual light around it (`visuals`, up to 8). Caught
//! in the library's glass jar, released on a flower it settles there; a jar of fireflies on display gives light.
mod model_firefly;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use creature::bestiary::{Fact, Species};
use creature::jar::Catchable;
use horizon_client::mod_api::{
    BlockEntityDraw, EmissiveEvent, EntityTickPostEvent, Flow, Hooks, Host, InteractEntityEvent, LivingDeathEvent,
    MobSubmitEvent, NativeColoredLight, NativeMobRender, RegisterGoalsEvent, VisualsEvent,
};

pub const FIREFLY: &str = "fireflies:firefly";
const TEXTURE: &str = "fireflies:entity/firefly/firefly";
const LIT_TEXTURE: &str = "fireflies:entity/firefly/firefly_lit";
/// The lit texture's full-bright overlay: its lantern's pixels.
const GLOW: &str = "fireflies:entity/firefly/firefly_lit_glow";

/// Its box (blocks) and the scale its model is drawn at.
const SIZE: (f64, f64, f64) = (0.3, 0.2, 0.15);
const SCALE: f32 = 0.5;

/// How it flies (SPECS §2): at night, slow and low, long drifts and short rests; it doesn't seek flowers.
static FLIGHT: creature::flutter::Kind = creature::flutter::Kind {
    night: true,
    speed: 0.05,
    height: (0.5, 2.0),
    flight: (200, 600),
    rest: (60, 200),
    wander: 4.0,
    drawn_to_light: false,
    lands_on_flowers: false,
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
    draw: book_view,
};

/// The bestiary's 3D view: lit, its wings folded, about a block across.
fn book_view(host: &dyn Host, draw: &mut BlockEntityDraw, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.1, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    let folded = |part: &str| !part.starts_with("wing");
    creature::model::draw_shown(
        host,
        draw,
        &model_firefly::FIREFLY,
        LIT_TEXTURE,
        &|_| [0.0; 3],
        &folded,
        creature::model::WHITE,
    );
    draw.pop();
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

/// Where the flying fireflies are, for their visual lights (the client's `visuals` has no positions of its own): kept
/// by the server's tick, read by the drawing (single player; arismoko/horizon#2740).
static FLYING_AT: Mutex<BTreeMap<u32, ([f64; 3], Instant)>> = Mutex::new(BTreeMap::new());
/// At most this many visual lights (Horizon's budget per mod is 8), their colour, reach and strength.
const LIGHTS: usize = 8;
const LIGHT_RGB: u32 = 0xD6FF6E;
const LIGHT_RADIUS: f32 = 3.0;
const LIGHT_INTENSITY: f32 = 0.8;

pub fn hooks() -> Box<dyn Hooks> {
    creature::jar::register(&IN_JAR);
    creature::bestiary::register(&BESTIARY);
    Box::new(Fireflies)
}

struct Fireflies;

/// A group's rotation this frame: flying, the wing cases lift and the wings beat fast.
fn pose(name: &str, mob: &NativeMobRender<'_>, flying: bool) -> [f32; 3] {
    if !flying {
        return [0.0; 3];
    }
    let beat = 25.0 + 30.0 * (mob.age * 2.2).sin();
    match name {
        "elytra_left" => [-20.0, 0.0, 35.0],
        "elytra_right" => [-20.0, 0.0, -35.0],
        "wing_left" => [0.0, 0.0, beat],
        "wing_right" => [0.0, 0.0, -beat],
        _ => [0.0; 3],
    }
}

impl Hooks for Fireflies {
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id == FIREFLY {
            creature::flutter::register(event.mob, SIZE, &FLIGHT);
        }
    }

    /// Right-click: nothing (no trading screen, which its template would open).
    fn interact_entity(&self, _: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if event.kind == FIREFLY {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind != FIREFLY {
            return;
        }
        creature::flutter::tick_post(host, event.dimension, event.entity);
        if creature::flutter::flying(event.entity)
            && let Some(at) = host.entity(event.entity).and_then(|e| host.entity_position(e))
        {
            FLYING_AT.lock().unwrap().insert(event.entity, (at, Instant::now()));
        }
    }

    fn living_death(&self, _: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == FIREFLY {
            creature::flutter::gone(event.entity);
            FLYING_AT.lock().unwrap().remove(&event.entity);
        }
        Flow::Continue
    }

    /// The lit texture's lantern is full-bright.
    fn emissive(&self, _: &dyn Host, event: &mut EmissiveEvent<'_>) {
        event.layers.push((LIT_TEXTURE, GLOW));
    }

    /// A soft yellow-green light around the lit fireflies flying (seen only, it lights nothing in the world).
    fn visuals(&self, _: &dyn Host, event: &mut VisualsEvent<'_>) {
        let mut at = FLYING_AT.lock().unwrap();
        at.retain(|_, (_, seen)| seen.elapsed().as_secs() < 2);
        for (id, (p, _)) in at.iter().filter(|(id, _)| lit(**id)).take(LIGHTS) {
            let _ = id;
            event.lights.push(NativeColoredLight {
                at: [p[0], p[1] + 0.1, p[2]],
                rgb: LIGHT_RGB,
                radius: LIGHT_RADIUS,
                intensity: LIGHT_INTENSITY,
            });
        }
    }

    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != FIREFLY {
            return Flow::Continue;
        }
        let flying = creature::flutter::flying(mob.id);
        let texture = if flying && lit(mob.id) { LIT_TEXTURE } else { TEXTURE };
        let shown = |part: &str| flying || !part.starts_with("wing");
        event.draw.push();
        creature::model::death_tilt(&mut *event.draw, mob);
        event.draw.scale([SCALE; 3]);
        creature::model::draw_shown(
            host,
            &mut *event.draw,
            &model_firefly::FIREFLY,
            texture,
            &|part| pose(part, mob, flying),
            &shown,
            creature::model::tint(mob),
        );
        event.draw.pop();
        Flow::Cancel
    }
}
