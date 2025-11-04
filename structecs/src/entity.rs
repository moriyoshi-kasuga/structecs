use std::{
    any::TypeId,
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::{Extractable, extractor::Extractor};

/// Unique identifier for an entity in the World.
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct EntityId {
    pub(crate) id: u32,
}

impl EntityId {
    pub(crate) fn new(id: u32) -> Self {
        Self { id }
    }

    /// Create an EntityId from a raw u32 value.
    ///
    /// # Safety
    /// The caller must ensure that the provided `id` is valid and unique within the context
    /// of the World. Using an invalid or duplicate ID may lead to undefined behavior.
    pub fn from_raw(id: u32) -> Self {
        Self { id }
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity({})", self.id)
    }
}

/// Internal reference-counted data for an entity.
///
/// Memory layout is optimized to reduce false sharing in concurrent scenarios.
/// The counter is placed in its own cache line to avoid contention with other fields.
#[repr(C)]
pub(crate) struct EntityDataInner {
    /// Reference counter - placed first and aligned to cache line for optimal concurrent access
    pub(crate) counter: AtomicUsize,

    /// Pointer to the entity data
    pub(crate) data: NonNull<u8>,

    /// Extractor for component access
    pub(crate) extractor: Arc<Extractor>,
}

#[repr(transparent)]
pub struct EntityData {
    inner: NonNull<EntityDataInner>,
}

// SAFETY: EntityData uses atomic reference counting and all internal data
// is properly synchronized with Arc and RwLock. Safe to send across threads.
unsafe impl Send for EntityData {}
// SAFETY: EntityData uses Arc for extractor and RwLock for additional components,
// providing safe concurrent access. Safe to share across threads.
unsafe impl Sync for EntityData {}

impl EntityData {
    pub(crate) fn inner(&self) -> &EntityDataInner {
        // SAFETY: inner is always valid and points to a properly initialized EntityDataInner
        // that is kept alive by reference counting.
        unsafe { self.inner.as_ref() }
    }

    pub(crate) fn new<E: crate::Extractable>(entity: E, extractor: Arc<Extractor>) -> Self {
        let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
        let inner = EntityDataInner {
            counter: AtomicUsize::new(1),
            // SAFETY: Box::into_raw never returns null
            data: unsafe { NonNull::new_unchecked(ptr) },
            extractor,
        };
        Self {
            // SAFETY: Box::into_raw never returns null
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(inner))) },
        }
    }

    pub(crate) unsafe fn extract_by_offset<T: 'static>(
        &self,
        offset: usize,
    ) -> crate::Acquirable<T> {
        let data_ptr = self.inner().data;
        // SAFETY: The caller guarantees that offset is valid for type T within the entity data.
        // The offset comes from the Extractor which validates it during creation.
        let extracted = unsafe { data_ptr.add(offset).cast::<T>() };
        crate::Acquirable::new(extracted, self.clone())
    }

    pub(crate) fn data_ptr(&self) -> NonNull<u8> {
        self.inner().data
    }

    pub(crate) fn extract<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        // SAFETY: extract_ptr validates the type through the Extractor
        let extracted = unsafe { self.extract_ptr::<T>()? };
        Some(crate::Acquirable::new(extracted, self.clone()))
    }

    pub(crate) unsafe fn extract_ptr<T: 'static>(&self) -> Option<NonNull<T>> {
        // SAFETY: The caller must ensure proper synchronization. The extractor validates
        // that type T exists in the entity data and returns None if not present.
        unsafe { self.inner().extractor.extract_ptr::<T>(self.inner().data) }
    }

    pub(crate) fn add_additional<E: Extractable>(&self, data: E) {
        todo!()
    }

    pub(crate) fn extract_additional<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        todo!()
    }

    pub(crate) fn has_additional<T: 'static>(&self) -> bool {
        todo!()
    }

    pub(crate) fn remove_additional<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        todo!()
    }
}

impl Drop for EntityData {
    fn drop(&mut self) {
        let inner = self.inner();
        if inner.counter.fetch_sub(1, Ordering::Release) > 1 {
            return;
        }

        std::sync::atomic::fence(Ordering::Acquire);

        // Drop the main entity data
        // SAFETY: The dropper was created for the entity type when it was constructed.
        // The data pointer is valid and points to properly allocated data of the correct type.
        unsafe { (inner.extractor.dropper)(inner.data) };

        // SAFETY: We are the last reference (counter reached 0), so we can safely
        // deallocate the inner data. The inner pointer is valid and was allocated via Box.
        unsafe {
            let inner = Box::from_raw(self.inner.as_ptr());
            drop(inner);
        }
    }
}

impl Clone for EntityData {
    fn clone(&self) -> Self {
        self.inner().counter.fetch_add(1, Ordering::Relaxed);

        Self { inner: self.inner }
    }
}
