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

    /// Query all entities with component T.
    ///
    /// This method snapshots data from relevant archetypes and returns a Vec.
    /// Locks are held briefly during the snapshot phase, then released immediately,
    /// allowing concurrent operations to proceed without blocking.
    ///
    /// # Performance
    ///
    /// This method is optimized for struct-based queries. Since structecs manages
    /// entities at the struct level (not individual components), querying is
    /// straightforward and efficient. Results are collected into a single Vec
    /// to minimize allocations.
    ///
    /// # Concurrency
    ///
    /// Multiple threads can call this method simultaneously. Each archetype is
    /// locked independently and briefly, minimizing contention.
    pub fn query<T: 'static>(&self) -> Vec<(EntityId, Acquirable<T>)> {
        let mut results = Vec::new();
        
        for entry in self.archetypes.iter() {
            let archetype = entry.value();
            if archetype.has_component::<T>() {
                results.extend(archetype.iter_component::<T>());
            }
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
}

/// QueryWith builder for querying entities with base struct + additional components.
pub struct QueryWith<'w, T, A> {
    world: &'w World,
    _phantom: PhantomData<(T, A)>,
}

impl<'w, T: 'static, A: AdditionalTuple> QueryWith<'w, T, A> {
    /// Query entities with base struct T and additionals A.
    pub fn query(&'w self) -> Vec<(EntityId, Acquirable<T>, A::Output)> {
        self.world
            .query::<T>()
            .into_iter()
            .map(|(id, base)| {
                let data = self.world.get_entity_data(&id).unwrap();
                let additionals = A::extract_from(&data);
                (id, base, additionals)
            })
            .collect()
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
