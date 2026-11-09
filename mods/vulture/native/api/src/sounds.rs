//! The vulture's sounds (SPECS §7), played by the library (`creature::sounds`, common/SPECS.md §3.2):
//! `vulture:entity.vulture.*` from `content/assets/vulture/sounds.json`.
use creature::sounds::Voice;

pub const AMBIENT: &str = "vulture:entity.vulture.ambient";
pub const FLAP: &str = "vulture:entity.vulture.flap";
pub const TAKEOFF: &str = "vulture:entity.vulture.takeoff";

pub const VOICE: Voice = Voice {
    hurt: "vulture:entity.vulture.hurt",
    death: "vulture:entity.vulture.death",
};

pub use creature::sounds::queue;
