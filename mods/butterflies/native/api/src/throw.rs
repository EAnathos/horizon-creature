//! Thrown by hand (SPECS §4): right-click with butterfly dust throws a shimmering cloud of it ahead; confetti, a party
//! popper: a pop at its mouth, then bits of coloured paper shot out in a cone ahead and up, falling back down. One is
//! used up (not in creative). Particles only: neither does anything else.
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{Host, ParticleOptions, ParticleType};

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

/// From the mod's `use_item`: whether `stack` was thrown.
pub fn thrown(host: &mut dyn Host, stack: &ItemStack) -> bool {
    let item = &*stack.item;
    if item != DUST && item != CONFETTI {
        return false;
    }
    let Some(player) = host.acting_player() else {
        return false;
    };
    // From the eyes, along the look (vanilla's `Entity.calculateViewVector`).
    let eye = [player.position[0], player.position[1] + EYE_HEIGHT, player.position[2]];
    let (yaw, pitch) = (f64::from(player.yaw).to_radians(), f64::from(player.pitch).to_radians());
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    let at = |d: f64| [eye[0] + look[0] * d, eye[1] + look[1] * d - 0.2, eye[2] + look[2] * d];
    if item == DUST {
        // A cloud that widens as it goes, the dust glinting in it.
        for (i, colour) in DUST_COLOURS.iter().enumerate() {
            let d = 1.2 + i as f64 * 0.6;
            let spread = 0.15 + i as f64 * 0.12;
            host.send_particles(&dust(*colour, 0.7), None, at(d), [spread; 3], 0.01, 10);
        }
        let glint = ParticleOptions::simple(ParticleType::WaxOn);
        host.send_particles(&glint, None, at(2.5), [0.6, 0.4, 0.6], 0.5, 12);
        host.play_sound("minecraft:block.amethyst_block.chime", at(1.5), PLAYERS, 0.8, 1.6);
    } else {
        popper(host, at(1.8), look);
    }
    creature::hand::use_one(host, stack);
    true
}

/// A party popper going off at `mouth`, aimed along `look`: a pop with a few sparks, then the bits, each sent on
/// its own way (a zero-count particle moves along its `spread`) in a cone ahead and a little up, falling back down.
fn popper(host: &mut dyn Host, mouth: [f64; 3], look: [f64; 3]) {
    let sparks = ParticleOptions::simple(ParticleType::Firework);
    host.send_particles(&sparks, None, mouth, [0.1, 0.1, 0.1], 0.05, 4);
    host.play_sound("minecraft:entity.firework_rocket.blast", mouth, PLAYERS, 0.6, 1.8);
    host.play_sound("minecraft:entity.firework_rocket.twinkle", mouth, PLAYERS, 0.5, 1.6);
    let states: Vec<_> = CONFETTI_BLOCKS
        .iter()
        .filter_map(|name| {
            host.block_states()
                .iter()
                .find(|(_, f)| &*f.name == *name)
                .map(|(id, _)| id)
        })
        .collect();
    if states.is_empty() {
        return;
    }
    let mut random = creature::colours::Random::new(BITS as u32);
    let mut unit = || (random.next() % 2001) as f64 / 1000.0 - 1.0;
    let bit = ParticleOptions::simple(ParticleType::Block);
    for i in 0..BITS {
        let dir = [
            look[0] + unit() * CONE,
            look[1] + unit() * CONE + 0.35,
            look[2] + unit() * CONE,
        ];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-6);
        let speed = SPEED.0 + (unit() + 1.0) / 2.0 * (SPEED.1 - SPEED.0);
        let dir = [dir[0] / len, dir[1] / len, dir[2] / len];
        host.send_particles(&bit, Some(states[i % states.len()]), mouth, dir, speed, 0);
    }
}

/// A standing player's eye height.
const EYE_HEIGHT: f64 = 1.62;
/// `SoundSource.PLAYERS`.
const PLAYERS: usize = 7;

fn dust(colour: [f32; 3], scale: f32) -> ParticleOptions {
    let mut options = ParticleOptions::simple(ParticleType::Dust);
    options.color = [colour[0], colour[1], colour[2], 1.0];
    options.scale = scale;
    options
}
