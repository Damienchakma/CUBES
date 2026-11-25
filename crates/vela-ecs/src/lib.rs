//! A compact archetype-free ECS designed for small ARM64 targets.
//!
//! Components are stored in type-specific dense arrays with sparse entity lookup.
//! This avoids Bevy's larger runtime footprint while keeping world mutation explicit.

use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug)]
struct EntitySlot {
    generation: u32,
    alive: bool,
}

struct Storage {
    entities: Vec<Entity>,
    sparse: HashMap<Entity, usize>,
    values: Vec<Box<dyn Any + Send + Sync>>,
}

impl Storage {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            sparse: HashMap::new(),
            values: Vec::new(),
        }
    }
}

/// The runtime entity/component world.
#[derive(Default)]
pub struct World {
    entities: Vec<EntitySlot>,
    free: Vec<u32>,
    components: HashMap<TypeId, Storage>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.entities[index as usize];
            slot.alive = true;
            return Entity {
                index,
                generation: slot.generation,
            };
        }

        let index = self.entities.len() as u32;
        self.entities.push(EntitySlot {
            generation: 0,
            alive: true,
        });
        Entity {
            index,
            generation: 0,
        }
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entities
            .get(entity.index as usize)
            .is_some_and(|slot| slot.alive && slot.generation == entity.generation)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.iter().filter(|slot| slot.alive).count()
    }

    pub fn component_count(&self) -> usize {
        self.components
            .values()
            .map(|storage| storage.values.len())
            .sum()
    }

    pub fn insert<T: Any + Send + Sync>(&mut self, entity: Entity, component: T) -> Option<T> {
        if !self.contains(entity) {
            return None;
        }
        let type_id = TypeId::of::<T>();
        let storage = self.components.entry(type_id).or_insert_with(Storage::new);
        if let Some(index) = storage.sparse.get(&entity).copied() {
            let previous = std::mem::replace(&mut storage.values[index], Box::new(component));
            return previous.downcast::<T>().ok().map(|boxed| *boxed);
        }
        let index = storage.values.len();
        storage.entities.push(entity);
        storage.values.push(Box::new(component));
        storage.sparse.insert(entity, index);
        None
    }

    pub fn get<T: Any + Send + Sync>(&self, entity: Entity) -> Option<&T> {
        let storage = self.components.get(&TypeId::of::<T>())?;
        let index = storage.sparse.get(&entity)?;
        storage.values[*index].downcast_ref()
    }

    pub fn get_mut<T: Any + Send + Sync>(&mut self, entity: Entity) -> Option<&mut T> {
        let storage = self.components.get_mut(&TypeId::of::<T>())?;
        let index = *storage.sparse.get(&entity)?;
        storage.values[index].downcast_mut()
    }

    pub fn remove<T: Any + Send + Sync>(&mut self, entity: Entity) -> Option<T> {
        let storage = self.components.get_mut(&TypeId::of::<T>())?;
        let index = storage.sparse.remove(&entity)?;
        let removed = storage
            .values
            .swap_remove(index)
            .downcast::<T>()
            .ok()
            .map(|boxed| *boxed);
        storage.entities.swap_remove(index);
        if index < storage.entities.len() {
            let moved = storage.entities[index];
            storage.sparse.insert(moved, index);
        }
        removed
    }

    pub fn contains_component<T: Any + Send + Sync>(&self, entity: Entity) -> bool {
        self.components
            .get(&TypeId::of::<T>())
            .is_some_and(|storage| storage.sparse.contains_key(&entity))
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.contains(entity) {
            return false;
        }
        for storage in self.components.values_mut() {
            if let Some(index) = storage.sparse.remove(&entity) {
                storage.values.swap_remove(index);
                storage.entities.swap_remove(index);
                if index < storage.entities.len() {
                    let moved = storage.entities[index];
                    storage.sparse.insert(moved, index);
                }
            }
        }
        let slot = &mut self.entities[entity.index as usize];
        slot.alive = false;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(entity.index);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position(f32);
    #[derive(Debug, PartialEq)]
    struct Velocity(f32);

    #[test]
    fn component_storage_round_trip() {
        let mut world = World::new();
        let entity = world.spawn();
        assert_eq!(world.insert(entity, Position(2.0)), None);
        assert_eq!(world.get::<Position>(entity), Some(&Position(2.0)));
        world.get_mut::<Position>(entity).unwrap().0 = 4.0;
        assert_eq!(world.remove::<Position>(entity), Some(Position(4.0)));
        assert!(world.get::<Position>(entity).is_none());
    }

    #[test]
    fn despawn_removes_all_components_and_reuses_slot() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position(1.0));
        world.insert(entity, Velocity(2.0));
        assert!(world.despawn(entity));
        assert!(!world.contains(entity));
        assert_eq!(world.component_count(), 0);

        let reused = world.spawn();
        assert_eq!(reused.index(), entity.index());
        assert_ne!(reused.generation(), entity.generation());
    }
}
