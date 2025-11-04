use std::{
    any::TypeId,
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use parking_lot::RwLock;

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

    /// Padding to separate counter from other fields (prevents false sharing)
    /// Modern CPUs typically use 64-byte cache lines
    _pad: [u8; 56],

    /// Pointer to the entity data
    pub(crate) data: NonNull<u8>,

    /// Extractor for component access
    pub(crate) extractor: Arc<Extractor>,

    /// Additional components (optional runtime data)
    #[allow(clippy::type_complexity)]
    pub(crate) additional: RwLock<Vec<(TypeId, NonNull<u8>, Arc<Extractor>)>>,

    /// Removed additional components that need cleanup when entity is fully dropped
    #[allow(clippy::type_complexity)]
    pub(crate) removed_additional: RwLock<Vec<(NonNull<u8>, Arc<Extractor>)>>,
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
            _pad: [0; 56],
            // SAFETY: Box::into_raw never returns null
            data: unsafe { NonNull::new_unchecked(ptr) },
            extractor,
            additional: RwLock::new(Vec::new()),
            removed_additional: RwLock::new(Vec::new()),
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
        let ptr = {
            let boxed = Box::into_raw(Box::new(data)) as *mut u8;
            // SAFETY: Box::into_raw never returns null
            unsafe { NonNull::new_unchecked(boxed) }
        };

        let inner = self.inner();
        let type_id = TypeId::of::<E>();
        let extractor = Arc::new(Extractor::new::<E>());

        let mut additionals = inner.additional.write();

        // Check if already exists and replace
        if let Some(existing) = additionals.iter_mut().find(|(tid, _, _)| *tid == type_id) {
            // Drop old data properly
            // SAFETY: The dropper was created for this specific type when the data was added.
            // The pointer is valid and points to properly aligned data of the correct type.
            unsafe {
                (existing.2.dropper)(existing.1);
            }
            existing.1 = ptr;
            existing.2 = extractor;
        } else {
            additionals.push((type_id, ptr, extractor));
        }
    }

    pub(crate) fn extract_additional<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        let inner = self.inner();
        let additionals = inner.additional.read();

        let type_id = TypeId::of::<T>();
        let ptr = additionals
            .iter()
            .find(|(tid, _, _)| *tid == type_id)?
            .1
            .cast::<T>();

        Some(crate::Acquirable::new(ptr, self.clone()))
    }

    pub(crate) fn has_additional<T: 'static>(&self) -> bool {
        let inner = self.inner();
        let additionals = inner.additional.read();
        let type_id = TypeId::of::<T>();
        additionals.iter().any(|(tid, _, _)| *tid == type_id)
    }

    pub(crate) fn remove_additional<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        let inner = self.inner();
        let mut additionals = inner.additional.write();

        let type_id = TypeId::of::<T>();
        let pos = additionals.iter().position(|(tid, _, _)| *tid == type_id)?;
        let (_, ptr, extractor) = additionals.swap_remove(pos);

        // Store the removed additional for cleanup when EntityData is dropped
        inner.removed_additional.write().push((ptr, extractor));

        Some(crate::Acquirable::new(ptr.cast::<T>(), self.clone()))
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

        // Drop all additional data
        let additionals = inner.additional.read();
        for (_, ptr, extractor) in additionals.iter() {
            // SAFETY: Each dropper corresponds to its data type, and the pointer is valid.
            // The data was allocated when the additional component was added.
            unsafe {
                (extractor.dropper)(*ptr);
            }
        }

        // Drop all removed additional data
        let removed_additionals = inner.removed_additional.read();
        for (ptr, extractor) in removed_additionals.iter() {
            // SAFETY: Each dropper corresponds to its data type, and the pointer is valid.
            // The data was allocated when the additional component was added.
            unsafe {
                (extractor.dropper)(*ptr);
            }
        }

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
