# Building blocks of Japanese Riichi Mahjong

[![Crates.io](https://img.shields.io/crates/v/riichi-elements)](https://crates.io/crates/riichi-elements)
[![docs.rs](https://img.shields.io/docsrs/riichi-elements)](https://docs.rs/riichi-elements)

[Japanese Riichi Mahjong][riichi] is a variant of Mahjong, a card game played with tiles on a table. This crate 
defines the fundamental game elements / building blocks of Standard Japanese Riichi Mahjong. Documentation will 
assume knowledge of basic game concepts but will also link to relevant wiki pages.

This crate is `no_std` by default; [`std` is an optional feature](#std-default-disabled).

[riichi]: https://riichi.wiki/Japanese_mahjong

## What's Inside

### [`tile`]: Tiles and utils

[`tile::Tile`] encodes the following kinds of tiles (total 37) used in a standard game:

- 3 suits x 9 numerals each
- 4 winds
- 3 dragons
- "red 5" for each suit (optional)

The following are explicitly excluded:

- Tiles not used in the Japanese variant (flowers, seasons, wildcards, etc.) --- may appear in other Mahjong variants.
- Red tiles other than 5's --- may appear in some non-standard Japanese Riichi Mahjong rules.

A tile can be encoded as a 6-bit integer (`0..=36`) or its common string shorthand (e.g. "1m", "2p", "3s", "4z").

Notable utils:
- [`tile::tiles_from_str`] parses a string shorthand (e.g. "11123m566778s22z") into an iterator of tiles.
- [`tile::t`] is a macro for creating a "tile literal" in code (e.g. `t!("1m")`).

### [`tile_set`]: Multi- and single-sets of tiles

These can be used to represent any unordered set of tiles, such as a closed hand, a full winning hand, all waiting
tiles of a waiting hand, and tiles discarded by a player.

- [`tile_set::TileSet37`]: multi-set; treats "red 5" tiles separately from "normal 5" (34 + 3 = 37 kinds)
- [`tile_set::TileSet34`]: multi-set; treats "red 5" tiles the same as "normal 5" (34 kinds)
- [`tile_set::TileMask34`]: single-set counting unique tiles; treats "red 5" tiles the same as "normal 5" (34 kinds)

### [`meld`]: Tiles revealed and set aside from the closed hand.

[`meld::Meld`] includes:

- [`meld::Chii`]
- [`meld::Pon`]
- [`meld::Kakan`]
- [`meld::Daiminkan`]
- [`meld::Ankan`]

A meld can be encoded as a (non-zero) 15-bit integer; see [`meld::Meld::packed`].

### [`hand_group`]: groups of 3 tiles in the closed hand

[`hand_group::HandGroup`] includes:

- [`hand_group::HandGroup::Shuntsu`] ("123m")
- [`hand_group::HandGroup::Koutsu`] ("555z")

A hand group can be encoded as a 6-bit integer; see [`hand_group::HandGroup::packed`].

### [`player`]

[`player::Player`] = 0/1/2/3 (mod-4 arithmetics), representing the players initially seated at the East/South/West/North
positions respectively at the start of a game. This can be used to also represent the "relative-player mod 4".

### [`wall`]

See [mod-level docs][wall] for the convention on representing the wall of tiles.

- [`wall::Wall`] represents a standard 136-tile wall.
- [`wall::PartialWall`] represents a wall with some tiles unknown.

Note that these are intended to represent the complete wall, without considering the effects of dealing, drawing, or
any kind of revealing. To this end, utils are provided for indexing different parts of the wall (deal, normal draw, dora
indicators, kan draws).

## Optional Features

### `serde` (default: enabled)

Defines a JSON-centric serialization format for most of the types.

### `std` (default: disabled)

This crate is `no_std` by default. Use this to enable `std` support.

Notably, when used with `serde`, this enables deserializing [`tile::Tile`] from `String`. 
