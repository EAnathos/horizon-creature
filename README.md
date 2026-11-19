<p align="center">
  <img src="docs/media/banner.webp" alt="Sunrise Creatures at sunrise: the vulture, butterflies, a moth, a firefly, a worm, the cyclops, a fairy and a scorpion, with the creatures to come (a magma golem, a dung beetle, a sporelin, a living flower, a stick insect, a seagull, a flying fish, a jellyfish, a storm spirit)" width="100%">
</p>

<p align="center">
  <a href="#install"><b>Install</b></a> ·
  <a href="#the-mods"><b>The mods</b></a> ·
  <a href="#in-the-game"><b>In the game</b></a> ·
  <a href="CONTRIBUTING.md"><b>Contributing</b></a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/licence-MIT-E2AC52" alt="Licence: MIT"></a>
  <a href="https://playsunrise.net"><img src="https://img.shields.io/badge/runs%20on-Sunrise%20(Minecraft%2026.3)-F0795A" alt="Runs on Sunrise, Minecraft 26.3"></a>
  <img src="https://img.shields.io/badge/status-beta-7A6FD0" alt="Status: beta">
  <img src="https://img.shields.io/badge/language-English%20%C2%B7%20Fran%C3%A7ais-5A9E6F" alt="English and French">
</p>

# Sunrise Creatures

**Creature mods for [Sunrise](https://playsunrise.net)**, the Rust reimplementation of Minecraft Java 26.3 (formerly
Horizon). A vulture to tame and fly, butterflies and moths, fireflies, earthworms for your garden, a one-eyed giant
shepherd, shy fairies, scorpions and spirits of living lightning; and Minecraft's own creatures in the guide.

- **Install the ones you like**: each mod works alone, with one shared library.
- **Native mods**, written in Rust on Sunrise's mod API, with their own models, animations, sounds and behaviours.
- **In beta**: played and tested in single player; multiplayer and balance come later.

<table>
  <tr>
    <td width="50%"><img src="docs/media/readme/vulture.webp" alt="A tamed vulture with its saddle and two chests, and a wild one, on a flowery meadow"><br><sub>A tamed vulture, saddled, with its two chests</sub></td>
    <td width="50%"><img src="docs/media/readme/butterflies.webp" alt="Butterflies of several colours and a moth over a meadow"><br><sub>Butterflies in many colours and patterns</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/media/readme/cyclops.webp" alt="A blue one-eyed cyclops among its sheep"><br><sub>The cyclops and its flock</sub></td>
    <td width="50%"><img src="docs/media/readme/fairies.webp" alt="Two violet fairies hovering over flowers"><br><sub>Fairies, to befriend with glow berries</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/media/readme/scorpions.webp" alt="A desert, a badlands and a cave scorpion on sand"><br><sub>Scorpions of the desert, the badlands and the caves</sub></td>
    <td width="50%"><img src="docs/media/readme/bestiary.webp" alt="The creature guide open on the butterfly's page, its morphs below and the species met on the right"><br><sub>The creature guide's bestiary</sub></td>
  </tr>
</table>

<sub>In game on Sunrise, HUD hidden.</sub>

## The mods

| | Mod | What it adds |
|---|---|---|
| ![](docs/models/vulture-vulture-front.png) | [Vulture](mods/vulture/README.md) | A desert vulture you tame with raw chicken, saddle and fly like a plane, with chests and talons |
| ![](docs/models/butterflies-butterfly-front.png) | [Butterflies](mods/butterflies/README.md) | Butterflies by day and moths by night, in many colours and patterns; their dust makes confetti |
| ![](docs/models/fireflies-firefly-front.png) | [Fireflies](mods/fireflies/README.md) | Fireflies that light up the night, kept in a jar as a soft lantern |
| ![](docs/models/worms-worm-front.png) | [Worms](mods/worms/README.md) | Earthworms you settle in your garden, where they help crops grow |
| ![](docs/models/cyclops-cyclops-front.png) | [Cyclops](mods/cyclops/README.md) | A one-eyed giant shepherd at cave mouths, with its flock, its club and its boulders |
| ![](docs/models/fairies-fairy-front.png) | [Fairies](mods/fairies/README.md) | Shy glowing fairies you befriend with glow berries: they circle your head, lighten your step and leave you fairy dust |
| ![](docs/models/scorpions-scorpion-front.png) | [Scorpions](mods/scorpions/README.md) | Real-size scorpions in the shade of deserts, badlands and caves: they rear up when you come close and sting when hit; their stinger makes a dagger you throw |
| ![](docs/models/storm_spirits-storm_spirit-front.png) | [Storm Spirits](mods/storm_spirits/README.md) | Spirits of living lightning born in thunderstorms: they zap whoever hits them, and their sparks call lightning where you aim |
| ![](common/assets/creature/textures/item/guide.png) | [Vanilla Bestiary](mods/vanilla_bestiary/README.md) | Minecraft's own creatures in the creature guide's bestiary, found out as you meet them |

Every mod needs **[Sunrise Creatures](common/README.md)**, the library: it adds the glass jar, the creature guide and
the "Sunrise Creatures" creative tab, where all the series' items are.

<details>
<summary><b>Planned</b></summary>

- [Dung Beetles](mods/dung_beetles/README.md): beetles that roll dung balls under the stars and turn worn pastures
  back into grass.
- [Sporelins](mods/sporelins/README.md): shy mushroom sprites that tend a mushroom patch.
- [Seagulls](mods/seagulls/README.md): coastal food thieves that become fishing companions.
- [Living Flowers](mods/living_flowers/README.md): every small and tall vanilla flower come alive, to ambush you or
  guard you.
- [Jellyfish](mods/jellyfish/README.md): stinging jellyfish drifting in blooms, some glowing in the deep.
- [Flying Fish](mods/flying_fish/README.md): shoals that burst out of warm seas and glide over the waves.
- [Phasmids](mods/phasmids/README.md): stick and leaf insects hiding in the trees, a game of spotting them.
- [Magma Golem](mods/magma_golem/README.md): a six-block giant of living magma asleep in the Nether, a mini-boss to
  cool with snow.

</details>

## Install

You need **the Sunrise launcher**, from [playsunrise.net](https://playsunrise.net), and a Minecraft: Java Edition
account.

1. In the launcher's **Mods** page, find **Sunrise Creatures**: the collection installs every creature at once, or
   pick the ones you like (the library comes with them). Also at
   [playsunrise.net/mods](https://playsunrise.net/mods).
2. Press **Play**. These are native mods: the launcher builds the game with them the first time (a few minutes).

Each version is also a [release of this repository](https://github.com/EAnathos/sunrise-creatures/releases), signed:
`sunrise-launcher mods add-github <release link> --trust-native-code` installs one from there.

## In the game

- **The creature guide**: every player gets one the first time they join a world. Right-click it to open the
  **bestiary**: each creature you come across gets a page, turning in 3D, with the biomes you met it in, how rare it
  is and what you found out about it (what it eats, what catches it...).
- **The glass jar** (3 glass in a U and an oak slab, makes 2; only with Butterflies, Fireflies, Worms or Fairies
  installed): right-click a butterfly, a moth, a firefly, a worm or a fairy to catch it, right-click a block with the
  filled jar to release it. Released on a flower (or, for a worm, in a garden), it stays there. Sneak + right-click
  places the filled jar as a small glass jar holding the creature; fireflies and fairies make it glow.
- Monsters leave the series' creatures alone.
- Each mod's page tells you how to find and use its creatures.

Single player only for now; multiplayer comes later for the whole series.

## Build from source

The mods build with the repository's own tools (`cargo creature`, Rust) and Sunrise's `cargo sunrise-mod`:
[CONTRIBUTING.md](CONTRIBUTING.md) has the setup, the commands and the conventions, and
[docs/PUBLISHING.md](docs/PUBLISHING.md) how a version is released.

## Licence

[MIT](LICENSE). The sounds come from [freesound.org](https://freesound.org), under
[CC0](https://creativecommons.org/publicdomain/zero/1.0/).
