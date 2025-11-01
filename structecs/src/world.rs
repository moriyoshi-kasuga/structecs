use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use dashmap::DashMap;
use rayon::prelude::*;

use crate::{
    Acquirable, EntityId, Extractable,
    archetype::{Archetype, ArchetypeId},
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
    /// Archetypes indexed by their TypeId
    archetypes: DashMap<ArchetypeId, Arc<Archetype>>,

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

    /// Get or create an archetype for type E.
    fn get_archetype<E: Extractable>(&self) -> Arc<Archetype> {
        let archetype_id = ArchetypeId::of::<E>();
        self.archetypes
            .entry(archetype_id)
            .or_insert_with(|| Arc::new(Archetype::new::<E>()))
            .clone()
    }

    fn get_archetype_by_entity(&self, entity_id: &EntityId) -> Option<Arc<Archetype>> {
        let archetype_id = *self.entity_index.get(entity_id)?.value();
        self.archetypes.get(&archetype_id).map(|a| a.clone())
    }

    fn get_entity_data(&self, entity_id: &EntityId) -> Option<crate::entity::EntityData> {
        let archetype = self.get_archetype_by_entity(entity_id)?;
        archetype.entities.get(entity_id).map(|d| d.clone())
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

        let archetype_id = ArchetypeId::of::<E>();
        let archetype = self.get_archetype::<E>();

        archetype.add_entity(entity_id, entity);

        // Update the entity index (lock-free)
        self.entity_index.insert(entity_id, archetype_id);

        entity_id
    }

    pub fn add_additional<E: Extractable>(&self, entity_id: &EntityId, entity: E) -> bool {
        let data = match self.get_entity_data(entity_id) {
            Some(d) => d,
            None => return false,
        };
        data.add_additional(entity);
        true
    }

    pub fn extract_additional<T: 'static>(&self, entity_id: &EntityId) -> Option<Acquirable<T>> {
        let data = self.get_entity_data(entity_id)?;
        data.extract_additional::<T>()
    }

    pub fn remove_additional<T: 'static>(&self, entity_id: &EntityId) -> Option<Acquirable<T>> {
        let data = self.get_entity_data(entity_id)?;
        data.remove_additional::<T>()
    }

    /// Extract a specific component from an entity.
    pub fn extract_component<T: 'static>(&self, entity_id: &EntityId) -> Option<Acquirable<T>> {
        let archetype = self.get_archetype_by_entity(entity_id)?;
        archetype.extract_entity(entity_id)
    }

    /// Remove an entity from the world.
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    pub fn remove_entity(&self, entity_id: &EntityId) -> bool {
        let archetype_id = match self.entity_index.remove(entity_id) {
            Some((_, id)) => id,
            None => return false,
        };

        if let Some(archetype) = self.archetypes.get(&archetype_id)
            && let Some(_) = archetype.remove_entity(entity_id)
        {
            true
        } else {
            false
        }
    }

    /// Create an iterator over all entities with component T.
    ///
    /// This method snapshots data from relevant archetypes and returns an iterator.
    /// Locks are held briefly during the snapshot phase, then released immediately,
    /// allowing concurrent operations to proceed without blocking.
    ///
    /// # Performance
    ///
    /// This method is optimized for struct-based queries. Since structecs manages
    /// entities at the struct level (not individual components), iteration is
    /// straightforward and efficient.
    ///
    /// # Concurrency
    ///
    /// Multiple threads can call this method simultaneously. Each archetype is
    /// locked independently and briefly, minimizing contention.
    pub fn query_iter<T: 'static>(&self) -> impl Iterator<Item = (EntityId, Acquirable<T>)> {
        // Snapshot relevant archetypes in a single pass
        self.archetypes
            .iter()
            .filter_map(|entry| {
                let archetype = entry.value();
                if archetype.has_component::<T>() {
                    // Snapshot this archetype's data while holding the lock
                    Some(archetype.iter_component::<T>().collect::<Vec<_>>())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
    }

    /// Create a parallel iterator over all entities with component T.
    ///
    /// Uses Rayon for parallel processing across archetypes. This method is
    /// optimized for large datasets where the parallelization overhead is
    /// justified by the performance gains.
    ///
    /// # Performance Considerations
    ///
    /// - **Best for**: Large entity counts (>10,000 entities) with CPU-intensive operations
    /// - **Not ideal for**: Small datasets or simple queries (use `query_iter()` instead)
    /// - **Lock strategy**: Each archetype is locked independently and briefly during snapshotting
    ///
    /// # Concurrency
    ///
    /// Multiple threads can call this method simultaneously. The DashMap allows
    /// lock-free reads, and each archetype's RwLock is held only briefly during
    /// the snapshot phase. After snapshotting, parallel processing proceeds without
    /// holding any locks.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Process large numbers of entities in parallel
    /// world.par_query_iter::<Player>()
    ///     .for_each(|(id, player)| {
    ///         // CPU-intensive operation here
    ///     });
    /// ```
    pub fn par_query_iter<T: 'static + Send + Sync>(
        &self,
    ) -> impl ParallelIterator<Item = (EntityId, Acquirable<T>)>
    where
        Acquirable<T>: Send,
    {
        // Snapshot all relevant archetypes in parallel
        self.archetypes
            .iter()
            .par_bridge()
            .filter_map(|entry| {
                let archetype = entry.value();
                if archetype.has_component::<T>() {
                    // Snapshot this archetype's data while holding the lock
                    Some(archetype.iter_component::<T>().collect::<Vec<_>>())
                } else {
                    None
                }
            })
            .flatten()
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
