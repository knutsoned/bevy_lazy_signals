# Architecture

## Overview

The LazySignals API provides convenience functions to create an ECS entity for
each semantic structure that contains the proper components (bundles). The overall design is
inspired by similar structures in functional programming, such as the infamous Haskell monad.
The specifics of the developer API are inspired by the
[TC 39 Signals proposal](https://github.com/tc39/proposal-signals).

It is lossy by default, which means if multiple signals are sent in the same tick, which every
signal is applied most recently contains the value that will be propagated to the Computeds,
Effects, and Actions. The prior signal values within the tick are overwritten before they are sent
on as each command calls the merge_next function.

The mappings of valid combinations of components to bundles is enumerated in the next sections.

## Primitives

A propagator aggregates data from its dependencies. For a Computed, the data is merged into its
LazySignalsState. An Effect may uses its source values to perform some side effects. An Action
is an Effect but instead of having exclusive world access, it returns a CommandQueue to be
evaluated by a LazySignals system when the Action completes. It uses Bevy async tasks and is the
default method to spawn long running operations.

The commands returned by an Action as well as the closures passed to Effects may
themselves send new signals, to be evaluated during the standard LazySignals update cycle.

A LazySignalsState component holds the value, bound by the traits defined by LazySignalsData. An
ImmutableState component stores the type information required for reflection and is populated with
the return value of the init_component call when the LazyImmutableState is created.

To send a signal, merge the next_value and add a SendSignal component.

To form a Computed, add a ComputedImmutable component to the Signal entity.

A LazyEffect component identifies an Effect. A LazyEffect can contain an Action instead, which
does not have exclusive world access, but returns a CommandQueue to be applied by the LazySignals
update system.

## Exclusive Systems

### Check Tasks

The task checking system checks the status of each Action marked with RunningTask. If a task is
completed, RunningTask is removed and any commands in the returned CommandQueue are applied.

### Init System

The init system runs every tick. Newly added Computed, Effect, and Action components will have an
InitDependencies component to mark them. These systems just run subscribe for each of the sources
and triggers so that the relevant entities are notified at the proper time.

### Signal Processing

During processing, a (should be brief) write lock for the world is obtained. If the value of a
signal is unchanged, the SendSignal for each sent signal is simply discarded. Otherwise, each
Signal's data field is replaced with next_value. The Signal is marked with ValueChanged.
Subscribers are added to a "running" set and removed from the LazySignalsState's subscribers.
Finally, the SendSignal component is removed.

The initial "running" set is iterated. If the item is a Computed, then add a ComputeMemo component
to mark it for update. If it is an Effect or Action, add a DeferredEffect component to mark it
for scheduling. Effects may be triggered, which means sending a signal with no value, or triggering
upstream effects and tasks for a unit or typed but possibly unchanged value (e g. to represent a
button press).

Walk the subscriber tree, adding each item's subscribers to the "next_running" set and removing
them from its own subscribers. As each item processes, add it to a "processed" set and do not add
any item to the "next_running" set if it exists in the "processed" set. When the current "running"
set is exhausted, run the next one. The system exits when each item in the final running set
finishes and the next running set is empty.

### Memo Processing

The closure in the Computed component of every entity marked with a ComputeMemo component runs and
the result is stored in the LazyImmutableState. As each value is read, the Computed is added to the
next_subscribers of the source entity. If the value is itself a Computed, it will recompute if it's
marked with Dirty. Otherwise it simply returns the value. If the value is different, ValueChanged
will be added after the closure is evaluated, which will be used to limit which effects are
scheduled next. The Dirty component is removed whether the value changed or not.

A stack is kept of all running operations. If any source is dirty, the Computed will put itself and
its dirty sources on the stack. This avoids the use of direct recursion. The system exits when each
item in the stack finishes.

### Effect Processing

The effects system examimes the dependencies of each entity with a DeferredEffect component. If any
dependency of an Effect is changed, the Effect closure is called after placing the Effect into the
"running" set. Effects will also run if the entity has a Triggered component.

The first 4 systems can be run as needed in between systems that need to have signals processed
between them. It is recommended to only run the effects once per tick to avoid running the same
effects if triggered more than once. Alternatively, care must be taken to make sure effects can be
triggered safely repeatedly or else that the situation is avoided.

The system exits when each item in the "running" set finishes. Actions are processed like
Effects, but their closures do not receive a &mut World and instead must return a CommandQueue.
