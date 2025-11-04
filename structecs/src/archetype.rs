use std::{any::TypeId, sync::Arc};

use dashmap::DashMap;
use rustc_hash::FxBuildHasher;

use crate::{Acquirable, EntityId, Extractable, entity::EntityData, extractor::Extractor};

/// An archetype represents a unique combination of component types.
/// All entities with the same structure share an archetype.
pub struct Archetype {
    /// The extractor for this archetype's entity structure.
    pub(crate) extractor: Arc<Extractor>,

    /// Entities stored in this archetype.
    pub(crate) entities: DashMap<EntityId, EntityData, FxBuildHasher>,
}

impl Archetype {
    pub(crate) fn new<E: Extractable>() -> Self {
        Self {
            extractor: Arc::new(Extractor::new::<E>()),
            entities: DashMap::with_hasher(FxBuildHasher),
        }
    }

    pub(crate) fn add_entity<E: Extractable>(&self, id: EntityId, entity: E) -> EntityData {
        let data = EntityData::new(entity, self.extractor.clone());
        self.entities.insert(id, data.clone());
        data
    }

    /// Iterate over entities that have component T.
    pub(crate) unsafe fn iter_component_unchecked<T: 'static>(
        &self,
    ) -> impl Iterator<Item = (EntityId, Acquirable<T>)> {
        // SAFETY: The caller guarantees that this archetype contains type T
        // (typically verified via type index before calling this method).
        let offset = unsafe { self.extractor.offset(&TypeId::of::<T>()).unwrap_unchecked() };
        self.entities.iter().map(move |v| {
            let (id, data) = v.pair();
            // SAFETY: The offset is valid for type T in this archetype,
            // and all entities in this archetype have the same structure.
            let component = unsafe { data.extract_by_offset::<T>(offset) };
            (*id, component)
        })
    }

    /// Get entity data by ID.
    pub(crate) fn extract_entity<T: 'static>(&self, entity_id: &EntityId) -> Option<Acquirable<T>> {
        self.entities
            .get(entity_id)
            .and_then(|data| data.extract::<T>())
    }

    /// Remove an entity by ID.
    pub(crate) fn remove_entity(&self, entity_id: &EntityId) -> Option<EntityData> {
        self.entities.remove(entity_id).map(|(_, data)| data)
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
