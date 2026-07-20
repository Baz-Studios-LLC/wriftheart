# WriftHeart

An 8-bit action-adventure RPG in Rust + [Bevy](https://bevyengine.org) — a native
desktop game where every sprite and sound is still generated in code, with no art or
audio files on disk.

The world broke long ago. Ten shards of the WriftHeart sank into ten far lands, and
the Black Castle stands over the wound. Recover the shards, mend the heart, and face
what waits inside.

## Play

**From source (any OS with a [Rust toolchain](https://rustup.rs)):**

```sh
cargo run --release
```

On macOS you can also double-click **`Play WriftHeart Release.command`**, which builds
the release binary if needed and launches it.

Packaged desktop builds will be published under
[Releases](https://github.com/Baz-Studios-LLC/wriftheart/releases).

## Controls

|                | Keyboard        | Gamepad       |
| -------------- | --------------- | ------------- |
| Move           | WASD / Arrows   | Left stick    |
| Ability slots  | E · X · C · V   | A · B · X · Y |
| Interact       | F               | D-pad Up      |
| Codex (map)    | M / Tab         | LB            |
| Bag & gear     | I               | RB            |
| Skill tree     | K               | D-pad Right   |
| Pause          | Esc             | Start         |

Menus confirm with E / Enter. **Every control is rebindable in-game** — pause →
CONTROLS.

## What's in the game

- **A procedural, infinite overworld** — 24+ biomes in difficulty rings, day/night,
  seasons, and weather; every world seed is its own map.
- **The main quest** — ten shard dungeons (multi-floor, two-key locks, themed foes and
  bosses), the Saltmaze questline, and the Black Castle finale.
- **Rift Spires** — endless procedural descent with depth-scaled enemies and loot.
- **Procedural gear** — procedurally generated weapons and armor (base × material ×
  rarity × affixes), commission crafting, tiered tools and harvesting.
- **A PoE-style passive tree** — nine branches (war, blood, fortune, bulwark, wind,
  precision, magic, gathering, crafting) with keystones and tradeoffs.
- **Life in the towns** — named villagers with hearts, gifts, and birthdays; quests;
  shops; seasonal festivals; a restorable Guildhall in every city.
- **The homestead** — build a house, farm crops through four seasons, keep chickens
  and cows, fish, cook, and craft from your home chest.
- **Flute songs** — four-note melodies learned from folk and books; warp home, call
  storms, hurry the sun, open singing stones.
- **A living codex** — map, calendar, people, guilds, bestiary, items, songs, awards
  (79 achievements, half of them hidden), stats ledger, and collectible lore books.
- **Four save slots** with autosave.

## Architecture

A native [Bevy](https://bevyengine.org) 0.19 app. The game renders to a fixed
384×208 pixel canvas scaled up with crisp pixels — an 80px sidebar HUD on the left, a
19×13-tile room beside it — on a fixed 60 fps timestep. Every sprite is authored as a
character-grid string baked to a texture at boot, and every sound is DSP-synthesized to
PCM at boot; there are no art or audio assets.

| Area | Location |
| --- | --- |
| Game loop, systems, HUD, codex, save/load | `src/app/` (60+ system modules) |
| Combat resolver, health, hitboxes | `src/combat.rs` |
| Hero, mobs, bosses, sprite art | `src/actors/` |
| Items, inventory, gear, skill tree | `src/items.rs`, `src/inventory.rs`, `src/gear_data.rs` |
| World generation (biomes, towns, spawns) | `src/worldgen/` |
| Dungeons (generation, rooms, render) | `src/dungeon/` |
| Pixel baking, bitmap font, software canvas, layers | `src/gfx/` |
| UI widgets | `src/ui/` |
| Input — bindings, gamepad, derived prompts | `src/input.rs` |

Combat is data-driven: any *damager* (hitbox, damage, team) hurts any *target*
(hitbox, health, team) on another team through one resolver — weapons, spells, hazards,
and enemy attacks all flow through it.

See **`PORT.md`** for the port's locked decisions, module map, and milestones, and
**`BOSSES.md`** for the boss designs.

## Development

```sh
cargo run --release      # build + play
cargo clippy             # lints — kept at zero warnings
cargo test --release     # parity + invariant test suites
```

Visuals are verified headlessly with the **`WRIFT_SHOT`** harness:

```sh
WRIFT_SHOT=<scene> WRIFT_SHOT_PATH=/tmp/x.png cargo run --release
```

It stages a scene (a dungeon map, the flute overlay, a boss arena, the pause menu, …)
and renders it straight to a PNG with no window, so geometry and layout can be checked
from a screenshot. All art is authored as character-grid strings baked to textures at
boot (`src/gfx/`); the bitmap font is uppercase-only by design.
