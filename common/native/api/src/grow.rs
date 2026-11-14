//! Crops helped to grow (SPECS §3.10): a settled worm (a stage now and then) and fairy dust (as bone meal).
use horizon_client::mod_api::{Host, ParticleOptions, ParticleType};

/// The age a crop is grown at, by block (those with an `age`); others are left alone.
pub fn grown_age(block: &str) -> Option<u32> {
    match block {
        "minecraft:wheat"
        | "minecraft:carrots"
        | "minecraft:potatoes"
        | "minecraft:melon_stem"
        | "minecraft:pumpkin_stem" => Some(7),
        "minecraft:beetroots" | "minecraft:nether_wart" => Some(3),
        "minecraft:torchflower_crop" => Some(1),
        "minecraft:pitcher_crop" => Some(4),
        _ => None,
    }
}

/// Whether the crop at `pos` can still grow: its age, and the age it is grown at.
pub fn growing(host: &dyn Host, pos: [i32; 3]) -> Option<(u32, u32)> {
    let grown = grown_age(&host.block(pos))?;
    let age: u32 = host.block_property(pos, "age")?.parse().ok()?;
    (age < grown).then_some((age, grown))
}

/// Makes the crop at `pos` grow `stages` (at most to grown), with green sparkles on it as bone meal shows. Whether it
/// grew.
pub fn grow(host: &mut dyn Host, pos: [i32; 3], stages: u32) -> bool {
    let Some((age, grown)) = growing(host, pos) else {
        return false;
    };
    if !host.set_block_property(pos, "age", &(age + stages).min(grown).to_string()) {
        return false;
    }
    let at = [
        f64::from(pos[0]) + 0.5,
        f64::from(pos[1]) + 0.4,
        f64::from(pos[2]) + 0.5,
    ];
    let sparkle = ParticleOptions::simple(ParticleType::HappyVillager);
    host.send_particles(&sparkle, None, at, [0.3, 0.25, 0.3], 0.0, 6);
    true
}
