# Architecture

## Overview

First, a distinction between semantic structures (Signal, Computed, Effect) and component types
(SendSignal, ComputeMemo, DeferredEffect): The semantic structures are central concepts but do not
exist as a concrete type. The Signal API provides convenience functions to create an ECS entity for
each semantic structure that contains the proper Components. The three mappings of valid
combinations of components to semantic structures is enumerated in the next sections.

## Primitives

A Propagator aggregates data from its dependencies. For a Computed, the data is merged into its
LazySignalsState. An Effect may uses its source values to perform some side effects.

A LazySignalsState component holds the LazySignalsData value. To send a signal, merge the
next_value and add a SendSignal component.

To form a Computed, add a ComputedImmutable component to the Signal entity.

A LazyEffect component identifies an Effect.

### Init Systems

There are two init systems that run every tick. One is for Computeds and the other for Effects.
Newly added Computed and Effect entities will have a RebuildSubscribers component to mark them.
These systems just run subscribe for each of the sources and triggers so that the relevant entities
are notified at the proper time.

### Signal Processing

During processing, a (brief) write lock for the world is obtained. If the value is unchanged, the
SendSignal for each sent signal is discarded. Otherwise, each Signal's data field is replaced with
next_value. The Signal is added to a "changed" set. Subscribers are added to a "running" set and
removed from the LazySignalsState's subscribers. Finally, the SendSignal component is removed.

The initial "running" set is iterated. If the item is a Computed, then add a ComputeMemo component
to mark it for update. If it is an Effect, add a DeferredEffect component to mark it for
scheduling. Effects may be triggered, which means sending a signal with no value.

Walk the subscriber tree, adding each item's subscribers to the "running" set and removing them
from its own subscribers. As each item processes, add it to a "processed" set and do not add any
item to a new "running" set if it exists in the "processed" set. When the current "running" set is
exhausted, run the new one. The system exits when each item in each running set finishes.

### Memo Processing

The Propagator of every entity marked with a ComputeMemo component runs and the result is
stored in the LazyImmutableState. As each value is read, the Computed is added to the
next_subscribers of the source entity. If the value is itself a Memo, it will recompute if it's
in the "dirty" set. Otherwise it simply returns its value. If the value is different, it will be
added to the "changed" set which will be used to limit which effects are scheduled. The system
exits when each item in the "running" set finishes.

### Effect Processing

The effects system compares the dependencies for each entity with a DeferredEffect component
against a "changed" set. If any dependency of an Effect is changed, the Effect function is
called after placing the Effect into the "running" set. Effects will also run if the entity is
in the "triggered" set.

The first 4 systems can be run as needed in between systems that need to have signals processed
between them. It is recommended to only run the effects once per tick to avoid running the same
effects if triggered more than once. Alternatively, care must be taken to make sure effects can be
triggered repeatedly or else that the situation is avoided.

The system exits when each item in the "running" set finishes.
