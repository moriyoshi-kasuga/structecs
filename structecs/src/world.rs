use std::marker::PhantomData;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use dashmap::DashMap;

use crate::{
    Acquirable, EntityId, Extractable,
    archetype::{Archetype, ArchetypeId},
    entity::EntityData,
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

    /// Add multiple entities to the world in batch.
    ///
    /// Returns a Vec of EntityIds assigned to the entities in order.
    ///
    /// This method is optimized for bulk insertion by:
    /// - Pre-allocating entity IDs in a single atomic operation
    /// - Getting the archetype once for all entities
    /// - Minimizing index update overhead
    ///
    /// # Performance
    ///
    /// For adding many entities of the same type, this method is significantly faster
    /// than calling `add_entity()` repeatedly due to reduced atomic operations and
    /// archetype lookups.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    pub fn add_entities<E: Extractable>(
        &self,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityId> {
        let entities: Vec<E> = entities.into_iter().collect();
        let count = entities.len();

        if count == 0 {
            return Vec::new();
        }

        // Pre-allocate entity IDs in bulk (single atomic operation)
        let start_id = self
            .next_entity_id
            .fetch_add(count as u32, Ordering::Relaxed);

        // Get archetype once for all entities
        let archetype_id = ArchetypeId::of::<E>();
        let archetype = self.get_archetype::<E>();

        // Pre-allocate result Vec
        let mut entity_ids = Vec::with_capacity(count);

        // Add all entities
        for (i, entity) in entities.into_iter().enumerate() {
            let entity_id = EntityId::new(start_id + i as u32);
            archetype.add_entity(entity_id, entity);
            self.entity_index.insert(entity_id, archetype_id);
            entity_ids.push(entity_id);
        }

        entity_ids
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

    /// Remove multiple entities from the world in batch.
    ///
    /// Returns the number of entities successfully removed.
    ///
    /// This method is optimized for bulk deletion by:
    /// - Grouping entities by archetype to minimize archetype lookups
    /// - Batch-removing entities from each archetype
    ///
    /// # Performance
    ///
    /// For removing many entities, this method is more efficient than calling
    /// `remove_entity()` repeatedly because it processes entities in archetype
    /// groups, reducing overhead.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    pub fn remove_entities(&self, entity_ids: &[EntityId]) -> usize {
        use std::collections::HashMap;

        // Group entity IDs by archetype
        let mut archetype_groups: HashMap<ArchetypeId, Vec<EntityId>> = HashMap::new();

        for entity_id in entity_ids {
            if let Some((_, archetype_id)) = self.entity_index.remove(entity_id) {
                archetype_groups
                    .entry(archetype_id)
                    .or_default()
                    .push(*entity_id);
            }
        }

        // Remove entities from each archetype
        let mut removed_count = 0;
        for (archetype_id, entities) in archetype_groups {
            if let Some(archetype) = self.archetypes.get(&archetype_id) {
                for entity_id in entities {
                    if archetype.remove_entity(&entity_id).is_some() {
                        removed_count += 1;
                    }
                }
            }
        }

        removed_count
    }

    /// Query all entities with component T.
    ///
    /// Returns an iterator over (EntityId, Acquirable<T>) pairs.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Direct iteration
    /// for (id, player) in world.query::<Player>() {
    ///     println!("Player {}: health = {}", id, player.health);
    /// }
    ///
    /// // Collect to Vec when you need .len() or random access
    /// let players: Vec<_> = world.query::<Player>().collect();
    /// assert_eq!(players.len(), 100);
    /// ```
    ///
    /// # Performance
    ///
    /// This method is optimized to reduce allocations compared to the previous
    /// implementation. It pre-allocates capacity based on the number of matching
    /// archetypes and collects results efficiently.
    ///
    /// While this still collects results internally (due to Rust's borrow checker
    /// limitations with iterators), it provides better performance through:
    /// - Single allocation with pre-calculated capacity
    /// - Efficient extend operations
    /// - No redundant intermediate Vec allocations
    ///
    /// # Concurrency
    ///
    /// Multiple threads can call this method simultaneously. Each archetype is
    /// locked independently and briefly, minimizing contention.
    pub fn query<T: 'static>(&self) -> Vec<(EntityId, Acquirable<T>)> {
        // Collect matching archetypes
        let matching: Vec<_> = self
            .archetypes
            .iter()
            .filter(|entry| entry.value().has_component::<T>())
            .map(|entry| entry.value().clone())
            .collect();

        // Pre-allocate based on archetype count (heuristic: 16 entities per archetype)
        let estimated_capacity = matching.len() * 16;
        let mut results = Vec::with_capacity(estimated_capacity);

        // Collect from all matching archetypes
        for archetype in matching {
            results.extend(archetype.iter_component::<T>());
        }

        results
    }

    /// Get the number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.entity_index.len()
    }

    /// Get the number of archetypes in the world.
    pub fn archetype_count(&self) -> usize {
        self.archetypes.len()
    }

    /// Query entities with a base struct and additional components.
    ///
    /// Returns a QueryWith builder that allows iteration over entities
    /// that have the base struct type T, optionally with additional components A.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Query for Player entities with PlayerDeathed and PlayerBuff additionals
    /// world.query_with::<Player, (PlayerDeathed, PlayerBuff)>()
    ///     .iter()
    ///     .for_each(|(id, player, (deathed, buff))| {
    ///         // player: Acquirable<Player>
    ///         // deathed: Option<Acquirable<PlayerDeathed>>
    ///         // buff: Option<Acquirable<PlayerBuff>>
    ///     });
    /// ```
    pub fn query_with<'w, T: 'static, A: AdditionalTuple>(&'w self) -> QueryWith<'w, T, A> {
        QueryWith {
            world: self,
            _phantom: PhantomData,
        }
    }

    /// Check if an entity has an additional component.
    pub fn has_additional<T: 'static>(&self, entity_id: &EntityId) -> bool {
        self.get_entity_data(entity_id)
            .map(|data| data.has_additional::<T>())
            .unwrap_or(false)
    }

    /// Check if an entity exists in the world.
    pub fn contains_entity(&self, entity_id: &EntityId) -> bool {
        self.entity_index.contains_key(entity_id)
    }

    /// Remove all entities from the world.
    ///
    /// This method clears all entities and archetypes, resetting the world
    /// to an empty state. The entity ID counter is NOT reset.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe but should typically be called when no other
    /// operations are in progress for best performance.
    pub fn clear(&self) {
        self.entity_index.clear();
        self.archetypes.clear();
    }
}

/// QueryWith builder for querying entities with base struct + additional components.
pub struct QueryWith<'w, T, A> {
    world: &'w World,
    _phantom: PhantomData<(T, A)>,
}

impl<'w, T: 'static, A: AdditionalTuple> QueryWith<'w, T, A> {
    /// Query entities with base struct T and additionals A.
    ///
    /// Returns an iterator for efficient, zero-allocation querying.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Direct iteration
    /// for (id, player, (buff,)) in world.query_with::<Player, (Buff,)>().query() {
    ///     if let Some(buff) = buff {
    ///         println!("Player {} has buff power {}", player.name, buff.power);
    ///     }
    /// }
    ///
    /// // Collect if needed
    /// let results: Vec<_> = world.query_with::<Player, (Buff,)>().query().collect();
    /// ```
    pub fn query(&'w self) -> impl Iterator<Item = (EntityId, Acquirable<T>, A::Output)> + 'w {
        self.world.query::<T>().into_iter().map(|(id, base)| {
            let additionals = A::extract_from(&base.inner);
            (id, base, additionals)
        })
    }
}

/// Trait for tuples of additional components.
///
/// This trait allows querying for multiple additional components at once.
/// Each component in the tuple is returned as Option<Acquirable<T>>.
pub trait AdditionalTuple {
    type Output;
    fn extract_from(data: &EntityData) -> Self::Output;
}

macro_rules! impl_additional_tuple {
    ($($name:ident),*) => {
        impl<$($name: 'static),*> AdditionalTuple for ($($name),*,) {
            type Output = ($(Option<Acquirable<$name>>),*,);
            fn extract_from(data: &EntityData) -> Self::Output {
                (
                    $(data.extract_additional::<$name>()),*,
                )
            }
        }
    };
}

impl_additional_tuple!(A1);
impl_additional_tuple!(A1, A2);
impl_additional_tuple!(A1, A2, A3);
impl_additional_tuple!(A1, A2, A3, A4);
impl_additional_tuple!(A1, A2, A3, A4, A5);
impl_additional_tuple!(A1, A2, A3, A4, A5, A6);
impl_additional_tuple!(A1, A2, A3, A4, A5, A6, A7);
impl_additional_tuple!(A1, A2, A3, A4, A5, A6, A7, A8);
