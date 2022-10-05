# Japanese Riichi Mahjong Waiting Hand Decomposition

In [Japanese Riichi Mahjong], a closed hand with 3N+1 tiles is considered "waiting" (1 tile away from winning) if it 
matches:

- [One or more regular waiting pattern(s)](regular::RegularWait).
- [An irregular waiting pattern](irregular::IrregularWait).

This module provides [`WaitSet`], which can be calculated to show all the ways for a closed hand to be considered
waiting. It uses [`decomposer::Decomposer`] behind the scenes to iterate through all regular waiting patterns.
