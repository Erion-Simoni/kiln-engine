//! ECS world wrapper

use hecs::World;
use kiln_core::EntityId;

pub struct GameWorld {
    world: World,
}

impl GameWorld {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }

    pub fn world(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn spawn<T>(&mut self, components: T) -> EntityId
    where
        T: hecs::Bundle,
    {
        let entity = self.world.spawn(components);
        EntityId::new(entity.id())
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}
