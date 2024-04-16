use bevy_app::PostUpdate;
use bevy_ecs::prelude::World;

pub mod bevy_rx;
use bevy_rx::*;

pub mod prelude {
    pub use crate::{
        bevy_rx::{ Memo, Signal, ReactiveContext, Reactor },
        ReactiveExtensionsPlugin,
    };
}

/// Derived from bevy_rx:
pub struct ReactiveExtensionsPlugin;

impl ReactiveExtensionsPlugin {
    fn apply_deferred_effects(_world: &mut World) {
        /* TODO: effects
        world.resource_scope::<ReactiveContext<World>, _>(|world, mut rctx| {
            let mut effects: Vec<_> = std::mem::take(
                rctx.reactive_state.resource_mut::<RxDeferredEffects>().stack.as_mut()
            );
            for effect in effects.drain(..) {
                effect(world, &mut rctx.reactive_state);
            }
        })
        */
    }
}

impl bevy_app::Plugin for ReactiveExtensionsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ReactiveContext<World>>().add_systems(
            PostUpdate,
            Self::apply_deferred_effects
        );
    }
}
