# Japanese Riichi Mahjong Waiting Hand Decomposition

In [Japanese Riichi Mahjong][riichi], a closed hand with 3N+1 tiles is considered ["waiting" (1 tile away from 
winning)][machi] if it matches any of the following:

- [One or more regular / standard waiting pattern(s)](regular::RegularWait) --- the hand is _decomposed_ into 
  three-tile groups and a pair (either complete or waiting).
- [An irregular waiting pattern](irregular::IrregularWait) --- the hand matches Seven Pairs or Thirteen Orphans.

This crate can be used to analyze a 3N+1 closed hand and determine all possible waiting patterns for it. For each 
pattern, the waiting tile, pattern kind, and details of decomposition (for regular patterns) are calculated.

## Example

The [included CLI](src/main.rs) prints all possible waiting patterns for the hands supplied through command-line 
arguments or the standard input. It basically does the following (using the classic [Pure Nine Gates] example):

```rust
use riichi_decomp::*;
use riichi_elements::prelude::*;

let mut decomposer = Decomposer::new();
let tile_set = TileSet34::from_iter(tiles_from_str("1112345678999m"));
let result = WaitSet::from_tile_set(&mut decomposer, &tile_set);

assert_eq!(result.waiting_tiles.0, 0b111111111);
assert_eq!(result.regular.len(), 15);
assert_eq!(result.irregular, None);
```

Note that a [`Decomposer`] instance can be reused to analyze different hands. 

## Optional Features

### `serde` (default: enabled)

Provides serialization for all results. This is considered "informational" --- i.e. not 

### `static-lut` (default: enabled)

[riichi]: https://riichi.wiki/Japanese_mahjong
[machi]: https://riichi.wiki/Machi
[Pure Nine Gates]: https://riichi.wiki/Chuuren_poutou#Nine_tile_wait
