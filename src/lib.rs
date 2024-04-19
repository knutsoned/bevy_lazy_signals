use bevy_app::PostUpdate;
use bevy_ecs::prelude::World;

/* 
// vendored code
pub mod bevy_rx;
use bevy_rx::*;
*/

pub mod reference;
pub mod types;
use types::*;

pub mod prelude {
    // core impl from vendored code
    // bevy_rx::{ Memo, Signal, ReactiveContext, Reactor },

    // core impl from local
    pub use crate::types::{ ReactiveContext };

    pub use crate::ReactiveExtensionsPlugin;
}

/// Derived from bevy_rx:
pub struct ReactiveExtensionsPlugin;

impl ReactiveExtensionsPlugin {
    fn apply_deferred_effects(world: &mut World) {
        // TODO add deferred Signal and Memo update systems
        world.resource_scope::<ReactiveContext<World>, _>(|world, mut rctx| {
            let mut effects: Vec<_> = std::mem::take(
                rctx.reactive_state.resource_mut::<DeferredEffects>().stack.as_mut()
            );
            for effect in effects.drain(..) {
                effect(world, &mut rctx.reactive_state);
            }
        })
    }
}

impl bevy_app::Plugin for ReactiveExtensionsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ReactiveContext<World>>().add_systems(
            PostUpdate, // TODO should be a param
            Self::apply_deferred_effects // TODO let this system be added in a chain by the user
        );
    }
}
