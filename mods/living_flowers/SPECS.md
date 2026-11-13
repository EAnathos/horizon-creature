# Living Flowers: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): flowers that are not quite
flowers. They stand among the others in a meadow until you come close, then pull up their roots and go for you.
Befriended with a flower of their kind, one follows you and guards you, and in a flower pot it becomes a living
decoration that sways and looks at you. **Every small and tall vanilla flower** has its living kind ✅, each with its
own look and its own attack; the other flowering blocks (pink petals, wildflowers, cactus flower...) have none.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the spec and the models, no code yet.

- **One model per kind** (`assets/models/<kind>.bbmodel`, pictures in docs/MODELS.md), all with the same groups so the
  same code poses them all: two root feet (`root_*`), a stem or a body (`body`), two leaves for arms (`leaf_*`) and the
  bloom for a head (`head`), its face in the middle (glowing eyes, a zigzag of teeth) and its petals around it, each a
  group (`petal_top`, `petal_bottom`, `petal_left`, `petal_right`) that opens wide when it attacks and closes while it
  hides. Built with a shared builder in Blockbench, so the faces and proportions match.
- **Kinds sharing a shape share a model** ✅, with a texture each: the four tulips (`tulip.bbmodel`: red, orange, white,
  pink) and the golden dandelion (a second texture of `dandelion.bbmodel`). The eyeblossom is one model, open or closed
  by its pose.

Next: the code once Horizon's launcher builds the pinned revision again
(docs/HORIZON-ISSUES.md, #15764).

## 1. Technical context

- **Mod id**: `living_flowers` ✅. **One entity** ✅, `living_flowers:living_flower`, whose **kind** (§2.1) is rolled at
  spawn by biome and saved on it; one spawn egg (a kind for its biome, or a poppy). Items: the potted living flowers
  (§4).
- **One bestiary page** ✅ for every kind: like the butterfly's morphs, the kinds met are shown at the bottom of the
  page, each as its flower's icon, and picking one shows it in the 3D view. The library's morphs become any look a
  species gives (a colour and pattern, or a kind with its own model; common/SPECS.md §7), which the Sporelins' two caps
  use too.
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4), monsters sparing it (§3.4), the
  creature guide's pages (§7), **taming and ownership** (§6.2, moved from the vulture with the Sporelins and the
  Seagulls), and the **display block** of the glass jar (§4.1: a placed block that draws a creature) for the pots ✅.
- It copies the wandering trader (goals only, its own attack and target goals, as the cyclops), with a box the size of
  its kind's model ✅: about 0.4 × 0.85 blocks for a small flower, 0.6 × 1.5 for a tall one.

## 2. The living flower ✅

- **Hidden among flowers** ✅: a wild one stands still, rooted, among the real flowers of its biome, and looks just like
  one from a distance (its face closed). It spawns where its vanilla flower grows (§2.1), 1 to 2 together (tall ones
  alone), day and night.
- **Aggressive** ✅: a player who comes within **4 blocks** (6 if sprinting; a sneaking one can pass at 2) wakes it: it
  pulls up its roots with a rustle, opens its petals wide and goes for them, waddling fast on its roots. It gives up
  past 16 blocks, walks back and roots again where it stands. Awake day and night ✅.
- 10 health (tall ones 20). **Drops** ✅: its flower.
- Monsters leave it alone (library §3.4); it doesn't attack animals or villagers.

### 2.1 The kinds

Each kind's attack draws on what its flower does in vanilla (the effect it gives in a suspicious stew, where it has
one) ✅:

| Kind | Where it spawns | Its attack |
|---|---|---|
| **Poppy** ✅ | plains, sunflower plains, meadows, flower forests | bites (3 damage, every second), its petals snapping shut |
| **Dandelion** ✅ | plains, meadows, forests | keeps 3 to 8 blocks away and blows a puff of seeds every 3 s: 2 damage and Slowness 2 s |
| **Golden dandelion** ✅ | a rare dandelion (1 in 50) | as the dandelion, golden seeds: 3 damage, and it shimmers |
| **Cornflower** ✅ | plains, flower forests, meadows | lashes with its stem from 3 blocks, bending its bloom like a whip: 3 damage and a strong push, every 1.5 s |
| **Blue orchid** ✅ | swamps | spits a jet of swamp water from 5 blocks: 2 damage and a push, and puts out a burning player |
| **Allium** ✅ | flower forests | lobs a ball of florets that bursts where it lands (2 blocks around): 3 damage |
| **Azure bluet** ✅ | plains, sunflower plains, meadows, flower forests | small and quick, nips (2 damage) and Blindness 2 s |
| **Tulips** (red, orange, white, pink) ✅ | plains, flower forests | bite (3 damage) and Weakness 4 s |
| **Oxeye daisy** ✅ | plains, meadows, flower forests | the healer: weak nips (2 damage), and the living flowers within 6 blocks of it regenerate |
| **Lily of the valley** ✅ | flower forests | rings its bells: a sweet scent around it (2 blocks), Poison 3 s |
| **Wither rose** ✅ | rarely in soul sand valleys | bites: 3 damage and Wither 3 s |
| **Torchflower** ✅ | never wild: a torchflower grown from a sniffer's seed sometimes wakes up (1 in 10) | its fiery petals set the player on fire (3 s) on a hit |
| **Pitcher plant** ✅ | never wild, as the torchflower | snaps its pitcher shut on the player: 4 damage and pulls them close |
| **Eyeblossom** ✅ | pale gardens | closed and harmless by day; at night it opens its glowing eye and stares: Nausea 4 s to the player it sees, then bites (3 damage) |
| **Sunflower** ✅ | sunflower plains | by day, turns its face to the sun and flashes a glare from 8 blocks: 3 damage and Blindness 2 s |
| **Lilac** ✅ | forests, flower forests | shakes its blossom: a perfume cloud (3 blocks, every 4 s), 2 damage and Nausea 4 s |
| **Peony** ✅ | forests, flower forests | slams its heavy bloom: 4 damage and a big push |
| **Rose bush** ✅ | forests, flower forests | thorns: 4 damage, and a player who hits it takes 1 back |

## 3. Befriending ✅

- **With its own flower** ✅: a player holding a flower of its kind (a poppy for a poppy, a red tulip for a red tulip, an
  eyeblossom for an eyeblossom...) in their main hand calms it: it stops attacking them and looks at the flower.
  Right-click gives it; one chance in 3 it befriends that player (hearts and a little bloom of petals), otherwise it
  takes the flower and stays calm a moment. Any other flower does nothing.
- **Befriended** (the library's taming and ownership, §6.2): it follows its friend, waddling on its roots, stays or
  follows on right-click, named with a name tag, never despawns, and **guards** them: it attacks the monsters that
  attack its friend, as a tamed wolf does (each with its attack).
- Fed a flower of its kind, a befriended one heals (+4).

## 4. In a flower pot ✅

- **Potting** ✅: sneak + right-click a befriended living flower with an empty flower pot: it hops into the pot, and the
  player gets a **potted living flower** item, one per kind (`living_flowers:potted_poppy`, `potted_dandelion`...; a
  tall one stands taller than its pot, as a vanilla tall flower would).
- **Placed**, it is a **decorative** block that draws the living flower in its pot, **moving**: it sways, turns its
  bloom to look at the nearest player and opens its petals when its friend comes near. It fights nothing. It keeps its
  name and its friend.
- **Back out**: sneak + right-click the potted flower with an empty hand: it hops out, befriended as before, and the
  empty pot drops. Breaking the block gives the potted item back.
- A potted one gives no light, needs nothing and never dies.

## 5. Bestiary page ✅

One page for all the kinds (§1): the biomes met in, the kinds met, and the facts: "Ambush" (woken by you), "Flower"
(befriended with its flower), "Pot" (potted), "Guard" (fought a monster for you).

## 6. Out of scope for v1

- Breeding, riding. Flowers of other mods.

## 7. Open questions

- ~~Q1: One entity per kind?~~ One entity and one bestiary page, the kinds as its morphs (§1).
- ~~Q2: Hidden and aggressive, asleep at night?~~ Hidden and aggressive within 4 blocks, awake day and night (§2).
- ~~Q3: The attacks?~~ The poppy bites, the dandelion blows seeds (§2.1).
- ~~Q4: Befriending?~~ With its own flower, one chance in 3; it guards its friend (§3).
- ~~Q5: The pots?~~ Decorative, sneak + right-click to get it back out (§4).
- ~~Q6: Its drops?~~ Its flower (§2).
- ~~Q7: The cornflower's attack?~~ It lashes with its stem (§2.1).
- ~~Q8: Which flowers?~~ Every small and tall vanilla flower (§2.1).
- ~~Q9: The new kinds' attacks and where they spawn?~~ As in the table (§2.1).
