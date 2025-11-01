use crate::{Acquirable, EntityId, archetype::Archetype};

/// Iterator over query results.
///
/// This is more efficient than collecting into a Vec as it:
/// - Avoids allocation
/// - Can be consumed lazily
/// - Can be chained with other iterators
pub struct QueryIter<'w, T: 'static> {
    archetypes: std::slice::Iter<'w, Archetype>,
    #[allow(clippy::type_complexity)]
    current: Option<Box<dyn Iterator<Item = (&'w EntityId, Acquirable<T>)> + 'w>>,
}

impl<'w, T: 'static> QueryIter<'w, T> {
    pub(crate) fn new(archetypes: &'w [Archetype]) -> Self {
        Self {
            archetypes: archetypes.iter(),
            current: None,
        }
    }
}

impl<'w, T: 'static> Iterator for QueryIter<'w, T> {
    type Item = (&'w EntityId, Acquirable<T>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next item from current archetype iterator
            if let Some(ref mut iter) = self.current
                && let Some(item) = iter.next()
            {
                return Some(item);
            }

            // Move to next archetype that has component T
            loop {
                let archetype = self.archetypes.next()?;
                if archetype.has_component::<T>() {
                    self.current = Some(Box::new(archetype.iter_component::<T>()));
                    break;
                }
            }
        }
    }
}

/// Builder for more complex queries (future expansion).
pub struct Query<'w> {
    archetypes: &'w [Archetype],
}

impl<'w> Query<'w> {
    pub(crate) fn new(archetypes: &'w [Archetype]) -> Self {
        Self { archetypes }
    }

    /// Query for all entities with component T.
    pub fn iter<T: 'static>(&self) -> QueryIter<'w, T> {
        QueryIter::new(self.archetypes)
    }

    /// Collect results into a Vec (for compatibility).
    pub fn collect<T: 'static>(&self) -> Vec<(&'w EntityId, Acquirable<T>)> {
        self.iter::<T>().collect()
    }
}
