# Rationale

Some of the authors of popular reactive frameworks for JavaScript have been participating in an
effort to create a Signal built-in that will provide a common API to support a range of uses.
For Bevy, such a library could help efforts to integrate UI frameworks, enable networking, support
scripting, scene editing, file operations, and so on.

Drawing on numerous discussions, similar projects, and numerous other sources, the following list
of constraints, forces, and goals was assembled:

- Provide a commands-based interface to allow both scheduling signals and triggers, and reading
  values, without needing exclusive world access.
- Fully leverage the ECS to avoid OOP in favor of indexed entites, system queries, and reflection.
- Minimize dependency on other internal Bevy features, except prelude, bevy_ecs, and bevy_reflect.
- Minimize external dependencies (thiserror is probably overkill and could be removed).
- Optimize performance at the cost of roughly doubling some of the data storage via sparse sets.
- Avoid the use of an intrusive global state in favor of a Bevy resource that tracks the internals.
- Since the data and propagator structure is immutable, the reactive mechanics can be simplified.
- Do not allow self-referential computations.
- Do encourage a "one-way data flow" application architecture that relies on immutable values
  within a system, and uses asynchronous updates to merge new values.
- Implement a variation of an immutable propagator network that is glitch-free.
- Avoid macros in favor of relying on reflection, and don't make any new macros.
- Make the function signatures of computed memos and effects be as close to a regular closure as
  possible.

For the purpose of the preceding:

- Immutable means that values can not be set directly but require an asynchronous system to update.
  All updates are applied in a batch and maintain internal integrity outside of the batch process.
- Propagator refers to a data network structure described by Gerald Sussman et al.
- One-way data flow is a signals architecture popular in ECMAScript, essentially a propagator
  network, where values are either atomic, or derived from other atomics, such that there are no
  circular dependencies. Changes to atomic values can be efficiently communicated to the extended
  set of subscribers. Updates to values are driven by "events" which are handled asynchronously.
- Glitch-free means the system is internally consistent at all times outside of its
  internal update cycle.
- Do not taunt Happy Fun Ball(tm).

Being lazy operations on immutable values, the reference implementation should have decent speed.
Because all updates are batched, they can be processed in an efficient way, minimizing the need for
exclusive world access. The consumer of the API provides all static type information at
compile-time, and is responsible for ensuring the concrete types of sources and triggers match up
with the provided type signatures.
