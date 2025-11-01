use std::{any::TypeId, collections::HashMap, sync::Arc};

use crate::{
    archetype::{Archetype, ArchetypeId},
    entity::EntityData,
    extractor::Extractor,
    query::Query,
    Acquirable, EntityId, Extractable,
};

/// The central storage for all entities and their components.
/// 
/// Entities are organized into archetypes based on their structure for better performance.
#[derive(Default)]
pub struct World {
    /// Archetypes indexed by their TypeId.
    archetypes: HashMap<ArchetypeId, Archetype>,
    
    /// Cached extractors for each entity type.
    extractors: HashMap<TypeId, Arc<Extractor>>,
    
    /// Maps entity IDs to their archetype for fast lookup.
    entity_index: HashMap<EntityId, ArchetypeId>,
    
    /// Next entity ID to assign.
    next_entity_id: u32,
}

impl World {
    /// Create a new empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create an extractor for type E.
    fn get_extractor<E: Extractable>(&mut self) -> Arc<Extractor> {
        let type_id = TypeId::of::<E>();
        self.extractors
            .entry(type_id)
            .or_insert_with(|| Arc::new(Extractor::new::<E>()))
            .clone()
    }

    /// Get or create an archetype for type E.
    fn get_archetype<E: Extractable>(&mut self) -> &mut Archetype {
        let archetype_id = ArchetypeId::of::<E>();
        let extractor = self.get_extractor::<E>();
        self.archetypes
            .entry(archetype_id)
            .or_insert_with(|| Archetype::new(extractor))
    }

    /// Add an entity to the world.
    /// 
    /// Returns the ID assigned to the entity.
    pub fn add_entity<E: Extractable>(&mut self, entity: E) -> EntityId {
        let entity_id = EntityId::new(self.next_entity_id);
        self.next_entity_id += 1;

        let extractor = self.get_extractor::<E>();
        let entity_data = EntityData::new(entity, extractor);
        
        let archetype_id = ArchetypeId::of::<E>();
        let archetype = self.get_archetype::<E>();
        archetype.add_entity(entity_id, entity_data);
        
        self.entity_index.insert(entity_id, archetype_id);
        
        entity_id
    }

    /// Extract a specific component from an entity.
    pub fn extract_component<T: 'static>(&self, entity_id: &EntityId) -> Option<Acquirable<T>> {
        let archetype_id = self.entity_index.get(entity_id)?;
        let archetype = self.archetypes.get(archetype_id)?;
        let entity_data = archetype.get_entity(entity_id)?;
        entity_data.extract::<T>()
    }

    /// Remove an entity from the world.
    pub fn remove_entity(&mut self, entity_id: &EntityId) -> bool {
        let archetype_id = match self.entity_index.remove(entity_id) {
            Some(id) => id,
            None => return false,
        };

        if let Some(archetype) = self.archetypes.get_mut(&archetype_id) {
            archetype.remove_entity(entity_id).is_some()
        } else {
            false
        }
    }

    /// Query for all entities with component T (returns Vec for compatibility).
    /// 
    /// Consider using `query_iter()` for an iterator-based approach.
    pub fn query<T: 'static>(&self) -> Vec<(&EntityId, Acquirable<T>)> {
        self.query_iter::<T>().collect()
    }

    /// Create an iterator over all entities with component T.
    /// 
    /// This is more efficient than `query()` as it doesn't allocate a Vec.
    pub fn query_iter<T: 'static>(&self) -> impl Iterator<Item = (&EntityId, Acquirable<T>)> + '_ {
        self.archetypes.values().flat_map(|archetype| {
            if archetype.has_component::<T>() {
                Some(archetype.iter_component::<T>())
            } else {
                None
            }
        }).flatten()
    }

    /// Get a query builder for more complex queries.
    pub fn query_builder(&self) -> Query<'_> {
        let archetypes: Vec<&Archetype> = self.archetypes.values().collect();
        // SAFETY: We're creating a slice from collected references which is safe
        let archetypes_slice = unsafe {
            std::slice::from_raw_parts(archetypes.as_ptr() as *const Archetype, archetypes.len())
        };
        Query::new(archetypes_slice)
    }

    /// Get the number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.entity_index.len()
    }

    /// Get the number of archetypes in the world.
    pub fn archetype_count(&self) -> usize {
        self.archetypes.len()
    }
}
