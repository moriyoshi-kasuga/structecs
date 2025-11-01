use std::{any::TypeId, sync::Arc};

use crate::{Acquirable, EntityId, entity::EntityData, extractor::Extractor};

/// An archetype represents a unique combination of component types.
/// All entities with the same structure share an archetype.
pub struct Archetype {
    /// The extractor for this archetype's entity structure.
    pub(crate) extractor: Arc<Extractor>,

    /// Entities stored in this archetype.
    pub(crate) entities: Vec<(EntityId, EntityData)>,
}

impl Archetype {
    pub(crate) fn new(extractor: Arc<Extractor>) -> Self {
        Self {
            extractor,
            entities: Vec::new(),
        }
    }

    /// Add an entity to this archetype.
    pub(crate) fn add_entity(&mut self, id: EntityId, data: EntityData) {
        self.entities.push((id, data));
    }

    /// Check if this archetype can provide component type T.
    #[inline]
    pub(crate) fn has_component<T: 'static>(&self) -> bool {
        self.extractor.has_component::<T>()
    }

    /// Iterate over entities that have component T.
    pub(crate) fn iter_component<T: 'static>(
        &self,
    ) -> impl Iterator<Item = (&EntityId, Acquirable<T>)> + '_ {
        self.entities.iter().filter_map(|(id, data)| {
            let component = data.extract::<T>()?;
            Some((id, component))
        })
    }

    /// Get a slice of all entities for parallel iteration.
    #[inline]
    pub(crate) fn entities_slice(&self) -> &[(EntityId, EntityData)] {
        &self.entities
    }

    /// Get entity data by ID.
    pub(crate) fn get_entity(&self, entity_id: &EntityId) -> Option<&EntityData> {
        self.entities
            .iter()
            .find(|(id, _)| id == entity_id)
            .map(|(_, data)| data)
    }

    /// Remove an entity by ID.
    pub(crate) fn remove_entity(&mut self, entity_id: &EntityId) -> Option<EntityData> {
        let pos = self.entities.iter().position(|(id, _)| id == entity_id)?;
        Some(self.entities.swap_remove(pos).1)
    }

    /// Get the number of entities in this archetype.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.entities.len()
    }

    /// Check if this archetype is empty.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

/// Unique identifier for an archetype based on its TypeId.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub(crate) struct ArchetypeId(pub TypeId);

impl ArchetypeId {
    pub(crate) fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>())
    }
}
