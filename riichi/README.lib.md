# Riichi Mahjong Game Engine

This crate implements a game engine of [standard Japanese Riichi Mahjong](https://riichi.wiki/Japanese_mahjong).


## What's Included

- Basic [building blocks]: [Tile]s, [HandGroup]s, [Meld]s, [Player]s, etc.
- A [data model] to represent all aspects of a round of game: [State], [Action], [Reaction], etc.
- The [game engine]: [State] + [Action] + [Reaction] => next [State] or [end of round].

A minimal example of using the [game engine] can be found in its docs.

[building blocks]: common
[Tile]: common::Tile
[HandGroup]: common::HandGroup
[Meld]: common::Meld
[Player]: common::Player
[data model]: model
[State]: model::State
[Action]: model::Action
[Reaction]: model::Reaction
[end of round]: model::RoundEnd
[game engine]: engine::Engine


## Optional features

### `serde` (Default: enabled)

Defines a JSON-centric serialization format for most of the [`common`] and [`model`] types.

This simplifies interop with external programs (bots, client-server, analysis, data processing), persistence of game
states, etc..

See each individual type's docs for the detailed format.

### `tenhou-log` (Default: enabled)

Defines an intermediate de/serialization data model for Tenhou's JSON-formatted logs, and reconstruction of each round's
preconditions, action-reactions, and end conditions into our own [data model](model).

See [`interop::tenhou_log`] mod-level docs for details.

### `static-lut` (Default: disabled)

Our implementation of [regular waiting hand decomposition][decomp] uses 2 lookup tables to accelerate the computation.
These can either be generated on-demand (when [`Decomposer`][decomp] is first instantiated, using [`once_cell`]), or
directly compiled into the binary as `static` variables (using [`phf`]).

This optional feature enables the static compilation of these LUTs.

[decomp]: crate::analysis::decomposer::Decomposer

## References

- <https://riichi.wiki/Japanese_mahjong>
- <https://en.wikipedia.org/wiki/Mahjong>
- <https://ja.wikipedia.org/wiki/麻雀>
