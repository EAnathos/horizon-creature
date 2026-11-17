//! Thrown by hand (SPECS §4): right-click with butterfly dust throws a shimmering cloud of it ahead; confetti, a party
//! popper: a pop at its mouth, then bits of coloured paper shot out in a cone ahead and up, falling back down. One is
//! used up (not in creative). Particles only: neither does anything else.
use creature::particles::key;
use mod_api::{
    Hand, HostSide, InteractionResult, ItemPart, LevelRef, ParticleOptions, ServerHost, SoundSource, UseItemEvent,
};

pub const DUST: &str = "butterflies:butterfly_dust";
pub const CONFETTI: &str = "butterflies:confetti";

/// The dust's colours: the butterflies' own, pale.
const DUST_COLOURS: [[f32; 3]; 5] = [
    [1.0, 0.85, 0.45],
    [1.0, 0.7, 0.35],
    [0.65, 0.8, 1.0],
    [0.9, 0.75, 1.0],
    [1.0, 1.0, 0.9],
];
/// The confetti's bits: block particles of these bright blocks, which fly and fall under gravity as paper bits do.
const CONFETTI_BLOCKS: [&str; 9] = [
    "minecraft:red_concrete",
    "minecraft:orange_concrete",
    "minecraft:yellow_concrete",
    "minecraft:lime_concrete",
    "minecraft:light_blue_concrete",
    "minecraft:blue_concrete",
    "minecraft:magenta_concrete",
    "minecraft:pink_concrete",
    "minecraft:white_concrete",
];
/// How many bits a popper shoots, how wide its cone is (the spread around the look, as a fraction of it) and how
/// fast the bits leave (blocks per tick, between the two).
const BITS: usize = 140;
const CONE: f64 = 0.55;
const SPEED: (f64, f64) = (0.5, 1.1);

/// Butterfly dust and confetti: right-click throws them.
pub struct Thrown;

#[mod_api::sunrise_part]
impl ItemPart for Thrown {
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        if let HostSide::Server(host) = host {
            let item = creature::hand::id(&*host, event.stack);
            throw(host, event.player, event.hand, &item);
        }
        Some(InteractionResult::Consume)
    }
}

/// `player` throws `item` (dust or confetti) held in `hand`.
fn throw(host: &mut dyn ServerHost, player: u128, hand: Hand, item: &str) {
    let Some(player) = host.player(player) else {
        return;
    };
    let (Some((eye, look)), Some(level)) = (creature::hand::eye_and_look(&*host, player), player.level(&*host)) else {
        return;
    };
    let at = |d: f64| [eye[0] + look[0] * d, eye[1] + look[1] * d - 0.2, eye[2] + look[2] * d];
    if item == DUST {
        // A cloud that widens as it goes, the dust glinting in it.
        for (i, colour) in DUST_COLOURS.iter().enumerate() {
            let d = 1.2 + i as f64 * 0.6;
            let spread = 0.15 + i as f64 * 0.12;
            if let Some(k) = key("minecraft:dust") {
                level.send_particles(
                    host,
                    &ParticleOptions::dust(k, *colour, 0.7),
                    None,
                    at(d),
                    [spread; 3],
                    0.01,
                    10,
                );
            }
        }
        if let Some(k) = key("minecraft:wax_on") {
            level.send_particles(
                host,
                &ParticleOptions::simple(k),
                None,
                at(2.5),
                [0.6, 0.4, 0.6],
                0.5,
                12,
            );
        }
        level.play_sound(
            host,
            "minecraft:block.amethyst_block.chime",
            at(1.5),
            SoundSource::Players,
            0.8,
            1.6,
            None,
        );
    } else {
        popper(host, level, at(1.8), look);
    }
    creature::hand::use_one(host, player, hand);
}

/// A party popper going off at `mouth`, aimed along `look`: a pop with a few sparks, then the bits, each sent on
/// its own way (a zero-count particle moves along its `spread`) in a cone ahead and a little up, falling back down.
fn popper(host: &mut dyn ServerHost, level: LevelRef, mouth: [f64; 3], look: [f64; 3]) {
    if let Some(k) = key("minecraft:firework") {
        level.send_particles(host, &ParticleOptions::simple(k), None, mouth, [0.1, 0.1, 0.1], 0.05, 4);
    }
    let blast = "minecraft:entity.firework_rocket.blast";
    level.play_sound(host, blast, mouth, SoundSource::Players, 0.6, 1.8, None);
    let twinkle = "minecraft:entity.firework_rocket.twinkle";
    level.play_sound(host, twinkle, mouth, SoundSource::Players, 0.5, 1.6, None);
    let states: Vec<_> = CONFETTI_BLOCKS
        .iter()
        .filter_map(|name| host.block_state_id(name))
        .collect();
    let Some(k) = key("minecraft:block").filter(|_| !states.is_empty()) else {
        return;
    };
    let mut random = creature::colours::Random::new(BITS as u32);
    let mut unit = || (random.next() % 2001) as f64 / 1000.0 - 1.0;
    let bit = ParticleOptions::simple(k);
    for i in 0..BITS {
        let dir = [
            look[0] + unit() * CONE,
            look[1] + unit() * CONE + 0.35,
            look[2] + unit() * CONE,
        ];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-6);
        let speed = SPEED.0 + (unit() + 1.0) / 2.0 * (SPEED.1 - SPEED.0);
        let dir = [dir[0] / len, dir[1] / len, dir[2] / len];
        level.send_particles(host, &bit, Some(states[i % states.len()]), mouth, dir, speed, 0);
    }
}
