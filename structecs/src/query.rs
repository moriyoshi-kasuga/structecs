use std::{any::TypeId, sync::Arc};

use dashmap::{DashMap, iter::Iter};
use rustc_hash::FxBuildHasher;

use crate::{EntityId, Extractable, World, entity::EntityData};

type DashMapIter<'a> = Iter<'a, EntityId, EntityData, FxBuildHasher>;

pub struct QueryIter<T: 'static> {
    _phantom: std::marker::PhantomData<T>,
    #[allow(clippy::type_complexity)]
    matching: Vec<(usize, Arc<DashMap<EntityId, EntityData, FxBuildHasher>>)>,
    current: Option<(usize, DashMapIter<'static>)>,
}

impl<T: 'static> QueryIter<T> {
    pub(crate) fn new(world: &World) -> Self {
        let type_id = TypeId::of::<T>();
        let archetype = world
            .type_index
            .get(&type_id)
            .iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|archetype_id| {
                world.archetypes.get(archetype_id).map(|a| {
                    // SAFETY: The archetype is guaranteed to contain type T
                    let offset = unsafe { a.extractor.offset(&type_id).unwrap_unchecked() };
                    (offset, a.entities.clone())
                })
            })
            .collect();
        Self {
            _phantom: std::marker::PhantomData,
            matching: archetype,
            current: None,
        }
    }
}

impl<T: Extractable> Iterator for QueryIter<T> {
    type Item = (EntityId, crate::Acquirable<T>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((offset, current_iter)) = &mut self.current {
                if let Some(entry) = current_iter.next() {
                    let entity_id = *entry.key();
                    let entity_data = entry.value();
                    return Some((entity_id, unsafe { entity_data.extract_by_offset(*offset) }));
                } else {
                    self.current = None;
                }
            } else if let Some((offset, next_map)) = self.matching.pop() {
                let iter = next_map.iter();
                // SAFETY: We transmute the lifetime of the iterator to 'static because
                // the underlying DashMap is held in an Arc within the QueryIter struct,
                // ensuring that it lives as long as the QueryIter itself.
                let iter =
                    unsafe { std::mem::transmute::<DashMapIter<'_>, DashMapIter<'static>>(iter) };
                self.current = Some((offset, iter));
            } else {
                return None;
            }
        }
    }
}
