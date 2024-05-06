# Architecture

## Overview

First, a distinction between semantic structures (Signal, Memo, Effect) and component types
(Immutable, SendSignal, ComputeMemo, DeferredEffect): The semantic structures are central
concepts but do not exist as a concrete type. The Signal API provides convenience functions
to create an ECS entity for each semantic structure that contains the proper Components. The
three mappings of valid combinations of components to semantic structures is enumerated in the
next sections.

## Primitives

A Propagator aggregates data from its dependencies. For a Memo, the data is merged into its
Immutable. For an Effect, the Propagator uses the dependencies to perform some side effect.

An Immutable component holds the Observable value. To send a signal, set the next_value and add
a SendSignal component. To form a Memo, add a Propagator component to the Immutable. A
Propagator component without an Immutable is an Effect. The running set may be a stack in other
implementations.

### Signal Processing

During processing, a (brief) write lock for the world is obtained. A Signal is an Immutable
with no Propagator on the same entity. There is no Signal type per se. If the value is
unchanged, the SendSignal is discarded. Otherwise, each Signal's data field is replaced with
next_value. The Signal is added to a "changed" set. Subscribers are added to a running set and
removed from the Immutable subscribers. Finally the SendSignal component is removed.

The initial "running" set is iterated. If the item is a Memo (Propagator with Immutable), then
add a ComputeMemo component to mark it for update. If it is an Effect (Propagator without
Immutable), add a DeferredEffect component to mark it for scheduling. Walk the subscriber tree,
adding each item's subscribers to the "running" set and removing them from its own subscribers.
As each item processes, add it to a "completed" set and do not add any item to a new "running"
set if it exists in the "completed" set. When the current "running" set is exhausted, run the
new one. The system exits when each item in each running set finishes. This system should be a
while loop and not recursive.

### Memo Processing

The Propagator of every entity marked with a ComputeMemo component runs and the result is
stored in the Immutable. As each value is read, the Memo is added to the next_subscribers of
the value. If the value is itself a Memo, it will recompute if its ComputeMemo component is
present. Otherwise it simply returns its value. If the value is different, it will be added
to the "changed" set which will be used to limit which effects are scheduled.

This is the only part of the system that is recursive. As each member of the running set will
have its own stack when computation begins, it should be resistant to overflows. Each element
in the running set is added to an "completed" set and removed from the "running" set.

### Effect Processing

The effects system compares the dependencies for each entity with a DeferredEffect component
against a "changed" set. If any dependency of an Effect is changed, the Propagator function is
called after placing the Effect into the "running" set.

Any Immutable in the "completed" set will have its next_subscribers merged into its empty
subscribers set at the end of this system. The system exits when each item in the "running" set
finishes.
