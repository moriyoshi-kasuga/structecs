use std::{any::TypeId, sync::{Arc, atomic::{AtomicU32, Ordering}}};

use dashmap::DashMap;
use parking_lot::RwLock;
use rayon::prelude::*;

use crate::{
    archetype::{Archetype, ArchetypeId},
    entity::EntityData,
    extractor::Extractor,
    Acquirable, EntityId, Extractable,
};

/// The central storage for all entities and their components.
/// 
/// Entities are organized into archetypes based on their structure for better performance.
/// 
/// # Thread Safety
/// 
/// World uses lock-free data structures (DashMap) and per-archetype RwLocks for
/// efficient concurrent access. Multiple threads can:
/// - Add entities to different archetypes in parallel
/// - Query different archetypes in parallel
/// - Query and add entities simultaneously (queries snapshot archetypes)
#[derive(Default)]
pub struct World {
    /// Archetypes indexed by their TypeId, with per-archetype RwLocks for fine-grained concurrency.
    archetypes: DashMap<ArchetypeId, Arc<RwLock<Archetype>>>,
    
    /// Cached extractors for each entity type (lock-free concurrent access).
    extractors: DashMap<TypeId, Arc<Extractor>>,
    
    /// Maps entity IDs to their archetype for fast lookup (lock-free concurrent access).
    entity_index: DashMap<EntityId, ArchetypeId>,
    
    /// Next entity ID to assign (atomic for lock-free ID generation).
    next_entity_id: AtomicU32,
}

impl World {
    /// Create a new empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create an extractor for type E.
    fn get_extractor<E: Extractable>(&self) -> Arc<Extractor> {
        let type_id = TypeId::of::<E>();
        self.extractors
            .entry(type_id)
            .or_insert_with(|| Arc::new(Extractor::new::<E>()))
            .clone()
    }

    /// Get or create an archetype for type E.
    fn get_archetype<E: Extractable>(&self) -> Arc<RwLock<Archetype>> {
        let archetype_id = ArchetypeId::of::<E>();
        let extractor = self.get_extractor::<E>();
        self.archetypes
            .entry(archetype_id)
            .or_insert_with(|| Arc::new(RwLock::new(Archetype::new(extractor))))
            .clone()
    }

    /// Add an entity to the world.
    /// 
    /// Returns the ID assigned to the entity.
    /// 
    /// This method is thread-safe and can be called concurrently from multiple threads.
    /// Entities with different types can be added in parallel with minimal contention.
    pub fn add_entity<E: Extractable>(&self, entity: E) -> EntityId {
        // Generate entity ID atomically
        let entity_id = EntityId::new(self.next_entity_id.fetch_add(1, Ordering::Relaxed));

        let extractor = self.get_extractor::<E>();
        let entity_data = EntityData::new(entity, extractor);
        
        let archetype_id = ArchetypeId::of::<E>();
        let archetype = self.get_archetype::<E>();
        
        // Lock the archetype only for the duration of adding the entity
        archetype.write().add_entity(entity_id, entity_data);
        
        // Update the entity index (lock-free)
        self.entity_index.insert(entity_id, archetype_id);
        
        entity_id
    }

    /// Extract a specific component from an entity.
    pub fn extract_component<T: 'static>(&self, entity_id: &EntityId) -> Option<Acquirable<T>> {
        let archetype_id = self.entity_index.get(entity_id)?.clone();
        let archetype = self.archetypes.get(&archetype_id)?;
        let guard = archetype.read();
        let entity_data = guard.get_entity(entity_id)?;
        entity_data.extract::<T>()
    }

    /// Remove an entity from the world.
    /// 
    /// This method is thread-safe and can be called concurrently from multiple threads.
    pub fn remove_entity(&self, entity_id: &EntityId) -> bool {
        let archetype_id = match self.entity_index.remove(entity_id) {
            Some((_, id)) => id,
            None => return false,
        };

        if let Some(archetype) = self.archetypes.get(&archetype_id) {
            archetype.write().remove_entity(entity_id).is_some()
        } else {
            false
        }
    }

    /// Create an iterator over all entities with component T.
    /// 
    /// This is more efficient than collecting to a Vec as it doesn't allocate.
    /// 
    /// # Concurrency
    /// 
    /// This method snapshots archetype data while holding read locks briefly,
    /// then releases them. The iterator operates on the snapshot, allowing
    /// concurrent queries and entity additions to proceed without blocking.
    pub fn query_iter<T: 'static>(&self) -> impl Iterator<Item = (EntityId, Acquirable<T>)> {
        // Collect archetypes that have component T
        let relevant_archetypes: Vec<_> = self.archetypes
            .iter()
            .filter_map(|entry| {
                let archetype = entry.value().read();
                if archetype.has_component::<T>() {
                    // Collect entities while holding the read lock
                    // We need to clone EntityIds since we can't hold references past the lock
                    Some(archetype.iter_component::<T>()
                        .map(|(id, comp)| (*id, comp))
                        .collect::<Vec<_>>())
                } else {
                    None
                }
            })
            .collect();
        
        // Now we can iterate over the snapshot without holding any locks
        relevant_archetypes.into_iter().flatten()
    }

    /// Create a parallel iterator over all entities with component T.
    /// 
    /// Uses Rayon for parallel iteration across archetypes for better performance
    /// on large datasets. Each archetype's entities are processed in parallel.
    /// 
    /// # Concurrency
    /// 
    /// This method uses fine-grained locking - each archetype is read-locked
    /// independently and briefly. Multiple threads can query different archetypes
    /// in parallel with minimal contention.
    /// 
    /// # When to Use
    /// 
    /// Parallel queries have overhead and are most beneficial when:
    /// - Working with large entity counts (>10,000 entities)
    /// - Performing complex operations on each entity
    /// - The work can be effectively parallelized
    /// 
    /// For simple queries or small datasets, prefer `query_iter()`.
    pub fn par_query_iter<T: 'static + Send + Sync>(&self) -> impl ParallelIterator<Item = (EntityId, Acquirable<T>)>
    where
        Acquirable<T>: Send,
    {
        // Collect archetypes into a Vec so we can use Rayon's parallel iterator
        let archetypes: Vec<_> = self.archetypes.iter()
            .map(|entry| entry.value().clone())
            .collect();
        
        archetypes.into_par_iter().flat_map(|archetype| {
            let guard = archetype.read();
            if guard.has_component::<T>() {
                // Process entities in parallel within each archetype
                guard.entities_slice().par_iter().filter_map(|(id, data)| {
                    data.extract::<T>().map(|component| (*id, component))
                }).collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        })
    }

    // TODO: Query builder needs refactoring to work with Arc<RwLock<Archetype>>
    // For now, use query_iter() or par_query_iter() directly
    // 
    // /// Get a query builder for more complex queries.
    // pub fn query_builder(&self) -> Query<'_> {
    //     ...
    // }

    /// Get the number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.entity_index.len()
    }

    /// Get the number of archetypes in the world.
    pub fn archetype_count(&self) -> usize {
        self.archetypes.len()
    }
}
