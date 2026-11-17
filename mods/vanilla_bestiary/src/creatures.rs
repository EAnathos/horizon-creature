//! Minecraft's creatures in the bestiary (SPECS §2, §3): what the game's data doesn't say (`game_data.rs` says where
//! each spawns on its own, what it drops and what breeds it). Each creature that spawns on its own, bosses included;
//! not the golems a player builds, the happy ghast a player grows, nor the mule, only ever bred.
use creature::bestiary::Rarity::{self, Common, Rare, Uncommon, VeryRare};

/// One creature: how rare, where it lives besides its natural spawns, and what can be found out about it.
pub struct Creature {
    pub id: &'static str,
    pub rarity: Rarity,
    /// Where it lives besides the biomes it spawns in on its own: the structures', raids' and bosses' creatures (a
    /// biome, or `@` and a dimension for any biome of it).
    pub biomes: &'static [&'static str],
    pub breed: Breed,
    /// Tamed (`entity_tame`): the item shown.
    pub tame: Option<&'static str>,
    pub products: &'static [Product],
    pub special: Option<Special>,
}

/// What breeds it, shown beside the fact: its `<creature>_food` tag's first item, another, or nothing.
pub enum Breed {
    Data,
    With(&'static str),
    No,
}

/// What it gives without being killed, found out when a player gets it.
pub struct Product {
    /// The fact's id after `product.` and its language key's last word.
    pub fact: &'static str,
    pub item: &'static str,
    pub how: &'static [How],
}

pub enum How {
    /// A player uses this item on it (a bucket on a cow).
    Used(&'static str),
    /// Shears or a brush make it drop something (`loot_drops` with `used`).
    Tool,
    /// A player catches it in a bucket.
    Bucket,
    /// It drops one of these items on its own (an egg, a scute), seen by the players near.
    Drops(&'static [&'static str]),
}

/// One thing it does, seen by the players near: the item shown and the event.
pub struct Special {
    pub item: &'static str,
    pub seen: Seen,
}

pub enum Seen {
    /// It blows up (a creeper).
    Explodes,
    /// It takes or places a block (an enderman).
    ChangesBlock,
    /// It throws items away (a piglin's barter).
    DropsItems,
    /// It turns into this creature.
    Becomes(&'static str),
}

const OVERWORLD: &str = "@minecraft:overworld";
const NETHER: &str = "@minecraft:the_nether";
const END: &str = "@minecraft:the_end";
const DEEP_OCEANS: &[&str] = &[
    "minecraft:deep_ocean",
    "minecraft:deep_cold_ocean",
    "minecraft:deep_lukewarm_ocean",
    "minecraft:deep_frozen_ocean",
];
const VILLAGES: &[&str] = &[
    "minecraft:plains",
    "minecraft:meadow",
    "minecraft:desert",
    "minecraft:savanna",
    "minecraft:taiga",
    "minecraft:snowy_plains",
];

const MILK: Product = Product {
    fact: "milk",
    item: "minecraft:milk_bucket",
    how: &[How::Used("minecraft:bucket")],
};
const fn bucket(item: &'static str) -> Product {
    Product {
        fact: "bucket",
        item,
        how: &[How::Bucket],
    }
}
const fn becomes(item: &'static str, kind: &'static str) -> Option<Special> {
    Some(Special {
        item,
        seen: Seen::Becomes(kind),
    })
}

/// A creature with nothing more than its data: natural spawns, drops, its breeding food if any.
const fn plain(id: &'static str, rarity: Rarity) -> Creature {
    Creature {
        id,
        rarity,
        biomes: &[],
        breed: Breed::Data,
        tame: None,
        products: &[],
        special: None,
    }
}

const fn living_in(id: &'static str, rarity: Rarity, biomes: &'static [&'static str]) -> Creature {
    Creature {
        biomes,
        ..plain(id, rarity)
    }
}

pub static CREATURES: &[Creature] = &[
    living_in("minecraft:allay", Rare, &[OVERWORLD]),
    Creature {
        products: &[Product {
            fact: "scute",
            item: "minecraft:armadillo_scute",
            how: &[How::Tool, How::Drops(&["minecraft:armadillo_scute"])],
        }],
        ..plain("minecraft:armadillo", Uncommon)
    },
    Creature {
        products: &[bucket("minecraft:axolotl_bucket")],
        ..plain("minecraft:axolotl", Rare)
    },
    plain("minecraft:bat", Common),
    living_in(
        "minecraft:bee",
        Common,
        &[
            "minecraft:plains",
            "minecraft:sunflower_plains",
            "minecraft:meadow",
            "minecraft:flower_forest",
            "minecraft:forest",
            "minecraft:birch_forest",
            "minecraft:old_growth_birch_forest",
            "minecraft:cherry_grove",
        ],
    ),
    living_in("minecraft:blaze", Uncommon, &[NETHER]),
    Creature {
        products: &[Product {
            fact: "mushrooms",
            item: "minecraft:brown_mushroom",
            how: &[How::Tool],
        }],
        ..plain("minecraft:bogged", Uncommon)
    },
    living_in("minecraft:breeze", Rare, &[OVERWORLD]),
    plain("minecraft:camel", Uncommon),
    living_in("minecraft:camel_husk", Rare, &["minecraft:desert"]),
    Creature {
        biomes: &[
            "minecraft:plains",
            "minecraft:meadow",
            "minecraft:desert",
            "minecraft:savanna",
            "minecraft:taiga",
            "minecraft:snowy_plains",
            "minecraft:swamp",
        ],
        tame: Some("minecraft:cod"),
        ..plain("minecraft:cat", Uncommon)
    },
    plain("minecraft:cave_spider", Uncommon),
    Creature {
        products: &[Product {
            fact: "egg",
            item: "minecraft:egg",
            how: &[How::Drops(&[
                "minecraft:egg",
                "minecraft:blue_egg",
                "minecraft:brown_egg",
            ])],
        }],
        ..plain("minecraft:chicken", Common)
    },
    Creature {
        products: &[bucket("minecraft:cod_bucket")],
        ..plain("minecraft:cod", Common)
    },
    Creature {
        products: &[MILK],
        ..plain("minecraft:cow", Common)
    },
    living_in("minecraft:creaking", Rare, &["minecraft:pale_garden"]),
    Creature {
        special: Some(Special {
            item: "minecraft:tnt",
            seen: Seen::Explodes,
        }),
        ..plain("minecraft:creeper", Common)
    },
    plain("minecraft:dolphin", Uncommon),
    Creature {
        breed: Breed::With("minecraft:golden_carrot"),
        tame: Some("minecraft:saddle"),
        ..plain("minecraft:donkey", Uncommon)
    },
    plain("minecraft:drowned", Common),
    living_in("minecraft:elder_guardian", VeryRare, DEEP_OCEANS),
    living_in("minecraft:ender_dragon", VeryRare, &[END]),
    Creature {
        special: Some(Special {
            item: "minecraft:grass_block",
            seen: Seen::ChangesBlock,
        }),
        ..plain("minecraft:enderman", Uncommon)
    },
    living_in("minecraft:endermite", Rare, &[OVERWORLD, NETHER, END]),
    living_in("minecraft:evoker", Rare, &[OVERWORLD]),
    plain("minecraft:fox", Uncommon),
    plain("minecraft:frog", Uncommon),
    plain("minecraft:ghast", Uncommon),
    plain("minecraft:glow_squid", Uncommon),
    Creature {
        products: &[
            MILK,
            Product {
                fact: "horn",
                item: "minecraft:goat_horn",
                how: &[How::Drops(&["minecraft:goat_horn"])],
            },
        ],
        ..plain("minecraft:goat", Uncommon)
    },
    living_in("minecraft:guardian", Uncommon, DEEP_OCEANS),
    Creature {
        special: becomes("minecraft:rotten_flesh", "minecraft:zoglin"),
        ..plain("minecraft:hoglin", Uncommon)
    },
    Creature {
        breed: Breed::With("minecraft:golden_carrot"),
        tame: Some("minecraft:saddle"),
        ..plain("minecraft:horse", Uncommon)
    },
    Creature {
        special: becomes("minecraft:water_bucket", "minecraft:zombie"),
        ..plain("minecraft:husk", Common)
    },
    living_in("minecraft:iron_golem", Uncommon, VILLAGES),
    Creature {
        breed: Breed::With("minecraft:hay_block"),
        tame: Some("minecraft:white_carpet"),
        ..plain("minecraft:llama", Uncommon)
    },
    plain("minecraft:magma_cube", Uncommon),
    Creature {
        breed: Breed::With("minecraft:wheat"),
        products: &[
            MILK,
            Product {
                fact: "stew",
                item: "minecraft:mushroom_stew",
                how: &[How::Used("minecraft:bowl")],
            },
            Product {
                fact: "mushrooms",
                item: "minecraft:red_mushroom",
                how: &[How::Tool],
            },
        ],
        ..plain("minecraft:mooshroom", Rare)
    },
    Creature {
        tame: Some("minecraft:pufferfish"),
        ..plain("minecraft:nautilus", Uncommon)
    },
    plain("minecraft:ocelot", Rare),
    Creature {
        products: &[Product {
            fact: "sneeze",
            item: "minecraft:slime_ball",
            how: &[How::Drops(&["minecraft:slime_ball"])],
        }],
        ..plain("minecraft:panda", Rare)
    },
    plain("minecraft:parched", Uncommon),
    Creature {
        breed: Breed::No,
        tame: Some("minecraft:wheat_seeds"),
        ..plain("minecraft:parrot", Rare)
    },
    living_in("minecraft:phantom", Uncommon, &[OVERWORLD]),
    Creature {
        special: becomes("minecraft:lightning_rod", "minecraft:zombified_piglin"),
        ..plain("minecraft:pig", Common)
    },
    Creature {
        breed: Breed::No,
        special: Some(Special {
            item: "minecraft:gold_ingot",
            seen: Seen::DropsItems,
        }),
        ..plain("minecraft:piglin", Common)
    },
    living_in("minecraft:piglin_brute", Rare, &[NETHER]),
    living_in("minecraft:pillager", Uncommon, &[OVERWORLD]),
    plain("minecraft:polar_bear", Uncommon),
    Creature {
        products: &[bucket("minecraft:pufferfish_bucket")],
        ..plain("minecraft:pufferfish", Uncommon)
    },
    plain("minecraft:rabbit", Common),
    living_in("minecraft:ravager", Rare, &[OVERWORLD]),
    Creature {
        products: &[bucket("minecraft:salmon_bucket")],
        ..plain("minecraft:salmon", Common)
    },
    Creature {
        products: &[Product {
            fact: "wool",
            item: "minecraft:white_wool",
            how: &[How::Tool],
        }],
        ..plain("minecraft:sheep", Common)
    },
    living_in("minecraft:shulker", Uncommon, &[END]),
    living_in("minecraft:silverfish", Uncommon, &[OVERWORLD]),
    Creature {
        special: becomes("minecraft:powder_snow_bucket", "minecraft:stray"),
        ..plain("minecraft:skeleton", Common)
    },
    Creature {
        biomes: &[OVERWORLD],
        tame: Some("minecraft:saddle"),
        ..plain("minecraft:skeleton_horse", Rare)
    },
    plain("minecraft:slime", Uncommon),
    Creature {
        biomes: &[OVERWORLD],
        products: &[Product {
            fact: "seeds",
            item: "minecraft:torchflower_seeds",
            how: &[How::Drops(&["minecraft:torchflower_seeds", "minecraft:pitcher_pod"])],
        }],
        ..plain("minecraft:sniffer", VeryRare)
    },
    plain("minecraft:spider", Common),
    plain("minecraft:squid", Common),
    plain("minecraft:stray", Common),
    plain("minecraft:strider", Common),
    plain("minecraft:sulfur_cube", Uncommon),
    Creature {
        biomes: &["minecraft:swamp", "minecraft:mangrove_swamp"],
        products: &[bucket("minecraft:tadpole_bucket")],
        special: becomes("minecraft:lily_pad", "minecraft:frog"),
        ..plain("minecraft:tadpole", Uncommon)
    },
    living_in("minecraft:trader_llama", Uncommon, &[OVERWORLD]),
    Creature {
        products: &[bucket("minecraft:tropical_fish_bucket")],
        ..plain("minecraft:tropical_fish", Common)
    },
    Creature {
        products: &[Product {
            fact: "scute",
            item: "minecraft:turtle_scute",
            how: &[How::Drops(&["minecraft:turtle_scute"])],
        }],
        ..plain("minecraft:turtle", Uncommon)
    },
    living_in("minecraft:vex", Rare, &[OVERWORLD]),
    Creature {
        biomes: VILLAGES,
        special: becomes("minecraft:lightning_rod", "minecraft:witch"),
        ..plain("minecraft:villager", Common)
    },
    living_in("minecraft:vindicator", Rare, &[OVERWORLD]),
    living_in("minecraft:wandering_trader", Uncommon, &[OVERWORLD]),
    living_in("minecraft:warden", VeryRare, &["minecraft:deep_dark"]),
    plain("minecraft:witch", Uncommon),
    living_in("minecraft:wither", VeryRare, &[OVERWORLD, NETHER, END]),
    living_in("minecraft:wither_skeleton", Uncommon, &[NETHER]),
    Creature {
        tame: Some("minecraft:bone"),
        ..plain("minecraft:wolf", Uncommon)
    },
    living_in("minecraft:zoglin", Rare, &[OVERWORLD]),
    Creature {
        special: becomes("minecraft:water_bucket", "minecraft:drowned"),
        ..plain("minecraft:zombie", Common)
    },
    plain("minecraft:zombie_horse", Rare),
    living_in("minecraft:zombie_nautilus", Rare, &[OVERWORLD]),
    Creature {
        special: becomes("minecraft:golden_apple", "minecraft:villager"),
        ..plain("minecraft:zombie_villager", Uncommon)
    },
    plain("minecraft:zombified_piglin", Common),
];
