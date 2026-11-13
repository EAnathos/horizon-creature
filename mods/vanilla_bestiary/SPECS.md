# Vanilla Bestiary: specification

An add-on of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): Minecraft's own creatures
get pages in the creature guide's bestiary, found out in play like the series' creatures. It adds no creature and no
item ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)), whose book
it fills (§7). Like every module of the series, it needs no other mod.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: a spec only.

Next: the code once Horizon's launcher builds the pinned revision again
(docs/HORIZON-ISSUES.md, #15764), with the library's book tabs (common/SPECS.md §7.1).

## 1. Technical context

- **Mod id**: `vanilla_bestiary` ✅. A native mod only (`native/`), plus its translations (`content/`, the facts' text
  in English and French). No entity, no item.
- Uses from the library: the bestiary (§7): it registers one species per vanilla creature (`creature::bestiary::
  register`), with its biomes, its facts, its spawn egg as icon and its rarity.
- **No 3D view** ✅: the book's 3D view draws blocks and our own models (`Host::draw_scene`), not a vanilla mob's model,
  at our Horizon pin. A vanilla creature's page shows its spawn egg, large, in place of it, until Horizon can draw an
  entity in a scene (a request to file, docs/HORIZON-ISSUES.md). The library's `Species::draw` becomes optional.

## 2. Which creatures ✅

- **Every vanilla creature that spawns on its own** ✅, the Overworld's, the Nether's and the End's: animals (cow, pig,
  sheep, chicken, horse, wolf, cat, fox, bee, axolotl, frog, camel, sniffer, armadillo...), water creatures, villagers
  and the wandering trader, monsters (zombie, creeper, skeleton, enderman, the illagers...), and the bosses (the Ender
  Dragon, the Wither, the Warden, the elder guardian). Not the player, armour stands or other objects.
- Each with its **biomes** (where it spawns, read from the game's spawn rules when Horizon gives them to a mod, else a
  list in the mod), its **rarity** (common to very rare) and its **spawn egg** as icon. The Ender Dragon and the Wither,
  which have no egg, get an item of theirs (the dragon egg, a wither skeleton skull).
- Met as the series' creatures are: a wild one within a few blocks, in one of its biomes (a summoned, bred or tamed one
  doesn't count as wild ✅).

## 3. Facts ✅

What a player finds out about each, in play, each a word and an item icon on its page, as the series' facts:

- **Food / breeding**: fed the item that breeds it (wheat to a cow, seeds to a chicken, a golden carrot to a horse...).
- **Taming**: tamed (a wolf with a bone, a cat with fish, a horse ridden...).
- **Products**: what it gives without being killed (milk from a cow, wool from a sheep, a turtle's scute, honey from a
  bee's hive...).
- **Drops**: what it dropped when killed by the player, each item once.
- **Special**: one per creature where it has one (a creeper explodes, an enderman takes blocks, a piglin trades for
  gold, a villager's trades, a goat rams).

Found through the library's hooks (an item used on a creature, a creature tamed or bred, drops picked up after a kill);
a fact whose event a mod can't see at our pin is left out until it can.

## 4. Out of scope for v1

- The 3D view of vanilla creatures (§1), creatures of other mods, a list of everything still to meet.

## 5. Open questions

- ~~Q1: Which creatures?~~ Every one that spawns on its own, bosses included (§2).
- ~~Q2: Which facts?~~ Breeding, taming, products, drops and one special fact each (§3).
- ~~Q3: Without a 3D view?~~ Its spawn egg, large, until Horizon can draw a vanilla mob in a scene (§1).
