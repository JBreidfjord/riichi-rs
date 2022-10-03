# Tenhou Deck/Wall-Shuffling Algorithm

[![Crates.io](https://img.shields.io/crates/v/tenhou-shuffle)](https://crates.io/crates/tenhou-shuffle)
[![docs.rs](https://img.shields.io/docsrs/tenhou-shuffle)](https://docs.rs/tenhou-shuffle)

[Tenhou] is an online [Japanese Riichi Mahjong][riichi] game. As of 2022-10-02, Tenhou has
published [their algorithm for shuffling the deck / wall of tiles][blog-algo], alongside
with [validation data for a particular game seed][blog-data], including intermediate results.

This crate re-implements the published algorithm for reconstructing the full shuffled wall of any game from its RNG
seed.

This crate is `no_std`.

## MT19937 RNG

This crate bundles an implementation of [`MT19937`], copy-pasted from <https://crates.io/crates/mt19937>. This is mostly
for `no_std` compatibility.

- Seed: `[u32; 624]`.
- Raw output type: `u32`.

## The Algorithm

Assuming `u32` is little-endian (LSByte first) in byte buffers.

Each game begins by seeding an MT19937 RNG for the game. This seed can be retrieved as a base-64 encoding of the seed
array.

Each round in the game requires a shuffle. We first derive the randomness as follows:

- Generate 9 chunks of 1024 bits (`[u32; 288]`) from the game-wide RNG. This array is named `src`
  in [the original algorithm description][blog-algo]. **This is implemented as [`src_from_mt`]**.

- For every 1024-bit chunk (`[u32; 32]`, little-endian), calculate its SHA512 hash. All 9 chunks of 512 bits are
  concatenated into `[u32; 144]`. This array is named `rnd` in [the original algorithm description][blog-algo].
  **This is implemented as [`rnd_from_src`]**.

Then, we shuffle the wall with `rnd`:

- Start with wall = sorted array of all tiles; length = 136 (4 players) or 108 (3 players).
- Perform a low-to-high [Fisher-Yates Shuffle][fisher-yates], using `rnd[i]` as the random number table, scaled
  to `i..n` as `i + rnd[i] % (n - i)`. **This is implemented as [`shuffle_with_rnd`]**.
- The shuffled wall array should be dealt from the highest index to the lowest.

[Tenhou]: https://tenhou.net

[riichi]: https://riichi.wiki/Japanese_mahjong

[fisher-yates]: https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle

[blog-algo]: https://web.archive.org/web/20220328062032/http://blog.tenhou.net/article/30503297.html

[blog-data]: https://web.archive.org/web/20211026055010/http://blog.tenhou.net/article/174202532.html
