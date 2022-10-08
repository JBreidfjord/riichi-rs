# Lookup Table Generation for `riichi-decomp`

This crate is _NOT_ intended as a public interface. [`riichi-decomp`][decomp] should be used instead.

This crate contains algorithms for generating two lookup tables (LUTs). These LUTs speed up decomposition of a waiting 
hand into "standard" or "regular" form, i.e. not Seven Pairs or Thirteen Orphans. Separating the generation from the 
main crate makes it possible to call it at compile time through `build.rs`.

## The Tables

For a more detailed explanation, please refer to the source.

### "C-Table": Single-suit, 3N+2, Complete

- Key: octal-packed single-suit tile count histogram; 27 bits in a `u32`. (this is basically one element of 
  `TileSet34::packed` from [`riichi-elements`][elements]).
- Value: at most 4 "alternatives", each containing up to 4 groups, with the (optional) pair implied. This is encoded 
  as a nested 4 x 4 x 4-bit array (single-suit hand groups can be encoded in 4 bits) using [`nanovec::NanoStackBit`] 
  and [`nanovec::NanoArrayBit`].

### "W-Table": Single-suit, 3N+1, Waiting

- Key: same definition as `C-Table`
- Value: radix-packed array of waiting patterns, using [`nanovec::NanoStackRadix`].

[decomp]: https://crates.io/crates/riichi-decomp
[elements]: https://crates.io/crates/riichi-elements
