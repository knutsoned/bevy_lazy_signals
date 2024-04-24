# bevy_signals

Primitives and examples for integrating bevy_ecs with signals

[Architecture](ARCHITECTURE.md)

## Design Questions

- How to best prevent infinite loops?
- Is the effects system generic enough for consumers to be able to use their own?
- The current system eagerly adds all subscribers up the tree. Is it better to do this in a more
  deferred manner? Seems like it is more trouble to try to track all that than just note which ones
  actually changed at the end and then match the dependencies of effects against that. This assumes
  it is much cheaper to recalculate memos once unnecessarily versus avoiding those recalculations
  with a more complicated processing routine.
- Should we use macros or reflection or something else?
- Can the recursive part be made iterative?

TODO: examples

TODO: general usage

TODO: tutorial
