# Riichi Mahjong Game Engine

This crate implements a game engine of [standard Japanese Riichi Mahjong][riichi-wiki-home] in the form of a library,
building upon the foundation of [riichi-elements] and [riichi-decomp].

[riichi-elements]: https://crates.io/crates/riichi-elements
[riichi-decomp]: https://crates.io/crates/riichi-decomp


## Table of Contents

- [`model`] --- Data structures for the entire game:
    - [State] (including [`StateCore`](model::StateCore)), [Action], [Reaction]
    - [`RoundBegin`](model::RoundBegin), [`RoundEnd`](model::RoundEnd), ...
    - [`AgariResult`](model::AgariResult), [`Scoring`](model::Scoring), ...
    - [`RoundHistory`](model::RoundHistory), [`RoundHistoryLite`](model::RoundHistoryLite), ...
- [`engine`] --- The game [Engine].
- [`rules`] --- Configurable [Ruleset] for the engine.
- [`yaku`] --- All known [Yaku]'s and utils.
- [`interop`] --- Working with data models from other implementations of the Japanese Riichi Mahjong game.

[RoundBegin]: model::RoundBegin
[RoundEnd]: model::RoundEnd
[State]: model::State
[Action]: model::Action
[Reaction]: model::Reaction
[Engine]: engine::Engine
[Ruleset]: rules::Ruleset
[Yaku]: yaku::Yaku


## Quick Example

See docs on [`engine::Engine`].

```rust
use riichi::prelude::*;  // includes `Engine` and `riichi_elements::prelude::*`

let mut engine = Engine::new();

engine.begin_round(RoundBegin {
    ruleset: Default::default(),
    round_id: RoundId { kyoku: 0, honba: 0 },  // east 1 kyoku, 0 honba (first round in game)
    wall: wall::make_sorted_wall([1, 1, 1]),  // 1111m2222m3333m4444m0555m...
    pot: 0,
    points: [25000, 25000, 25000, 25000],
});
assert_eq!(engine.state().core.seq, 0);
assert_eq!(engine.state().core.actor, P0);

engine.register_action(Action::Discard(Discard {
    tile: t!("1m"), ..Discard::default()}))?;

// use `engine.register_reaction` for Chii/Pon/Daiminkan/Ron

let step = engine.step();
assert_eq!(step.action_result, ActionResult::Pass);

assert_eq!(engine.state().core.seq, 1);
assert_eq!(engine.state().core.actor, P1);
/* ... */

# Ok::<(), riichi::engine::ActionError>(())
```

In a more realistic setting:

- `round_id`, `pot`, and `points` may be either their begin-of-game values or derived from the previous round's results.
- `wall` should be shuffled, e.g. using the `rand` crate.
- The [State] of the engine should be observed by the players at each step.
- [Action]s and [Reaction]s should be from players' inputs.

## How We Model the Game

### Game Setup

Each game ([Hanchan], [Tonpuu], ...) is played by 4 players and consists of at least 1 round ([Kyoku]). The 3-player 
variant is currently not supported.

Each round starts with an [initial state](model::RoundBegin):

- The "[Ba]-[Kyoku]-[Honba]" triplet, i.e. "East 1 Kyoku, 0 Honba", represented as [`model::RoundId`].
- How many points each player has at the beginning of the round.
- How many ["riichi sticks"][Riichi] currently remains on the table.
- The complete [shuffled wall][Setup] (34 x 4 = 136 tiles) to be used in this round (see [`riichi_elements::wall`]).

[Hanchan]: https://riichi.wiki/Hanchan
[Tonpuu]: https://riichi.wiki/Tonpuusen
[Ba]: https://riichi.wiki/Ba
[Kyoku]: https://riichi.wiki/Kyoku
[Honba]: https://riichi.wiki/Honba
[Riichi]: https://riichi.wiki/Riichi
[Setup]: https://riichi.wiki/Japanese_mahjong_setup

### State Machine of a Round

The game flow within a round can be modeled as the following state machine:

```asciiart
   ┌──────┐
   │ Deal │
   └─┬────┘
     │
     │    ┌────────────────────────────────────────────────────────────────┐
     │    │                                                                │
     ▼    ▼             #1                                   #2            │
   ┌────────┐ Draw=Y    ┌────────────┐           ┌─────────────┐ Nothing   │
   │DrawHead├──────────►│            │           │             ├───────────┤
   └────────┘ Meld=N    │            │  Discard  │             │           │
   #4                   │            ├──────────►│             │  #3       ▼
                        │            │  Riichi   │             │  ┌─────────────────┐
                        │  In-turn   │           │ Resolved    │  │ Forced abortion │
                        │  player's  │           │ declaration │  └─────────────────┘
   ┌────────┐ Draw=Y    │  decision  │           │ from        │           ▲
┌─►│DrawTail├──────────►│            │           │ out-of-turn │           │
│  └────────┘ Meld=Y    │  (Action)  │           │ players     │ Daiminkan │
│  #4                   │            │           │             ├───────────┤
│                       │            │           │ (Reaction)  │           │
│                       │            │  Kakan    │             │           │
│  ┌────────┐ Draw=N    │            ├──────────►│             │ Chii      │
│  │Chii/Pon├──────────►│            │  Ankan    │             ├─────────┐ │
│  └────────┘ Meld=Y    └──┬───────┬─┘           └──────┬──────┘ Pon     │ │
│  #4   ▲                  │       │                    │                │ │
│       │         NineKinds│       │Tsumo               │Ron             │ │
│       │                  ▼       ▼                    ▼                │ │
│       │         ┌──────────┐   ┌─────┐             ┌─────┐             │ │
│       │       #3│ Abortion │   │ Win │#3           │ Win │#3           │ │
│       │         └──────────┘   └─────┘             └─────┘             │ │
│       │                                                                │ │
│       └────────────────────────────────────────────────────────────────┘ │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

There are multiple states within one logical turn of a round:

1. The player in turn is ready to take an action ([`model::Action`]), after incoming draw and/or meld. This action 
   might be terminal (abortion by nine kinds, or win by self draw).

2. Each other player may independently declare an reaction ([`model::Reaction`]): Chii, Pon, Daiminkan, or Ron.
   The resolved reaction type determines the next state.

3. After reaction resolution, we need to check for any involuntary round-ending conditions.

4. All done, then the next player gains draw and/or meld depending on what has happened so far, marking the 
   beginning of the next turn.

Not all actions are valid at all times; the validity often depends on state variables not illustrated in the state 
machine diagram.

### One-state-per-turn Simplification

It is possible to simplify by only explicitly modeling one state (per turn), namely the one before the in-turn 
player makes a decision (after taking a draw or a Chii/Pon). This is basically `#1` in the state machine diagram, 
represented by [`model::State`].

All other states in the diagram can be derived from this:

- The state marked `#2` is basically the pre-action state (`#1`) + the action taken.
- The states marked `#3` are terminal (abortion / win). They can be handled separately outside the normal game flow.
- The states marked `#4` are internal transitory states skipped over by the engine without any player input.

This key simplification enables a regular representation of the normal game flow of a round as a sequence of triplets:
[State] + [Action] + [Reaction] (optional).


## Optional features

### `serde` (Default: enabled)

Defines a JSON-centric serialization format for most of the common data structures, including those in the
[riichi-elements] crate.

This simplifies interop with external programs (bots, client-server, analysis, data processing), persistence of game
states, etc..

See each individual type's docs for the detailed format.

### `tenhou-log-json` (Default: enabled)

Defines an intermediate de/serialization data model for Tenhou's JSON-formatted logs, and reconstruction of each round's
preconditions, action-reactions, and end conditions into our own [data model](model).

See [`interop::tenhou_log_json`] mod-level docs for details.

### `static-lut` (Default: disabled)

Enables the corresponding feature in the [riichi-decomp] crate, which builds the lookup tables required by its hand 
analysis algorithms statically. If disabled, the lookup tables will be generated upon first instantiation of 
[`riichi_decomp::Decomposer`].


## References

- <https://riichi.wiki/Japanese_mahjong>
- <https://en.wikipedia.org/wiki/Mahjong>
- <https://ja.wikipedia.org/wiki/麻雀>

[riichi-wiki-home]: https://riichi.wiki/Japanese_mahjong
