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
pub(crate) struct EntityDataInner {
    pub(crate) data: NonNull<u8>,
    pub(crate) counter: AtomicUsize,
    pub(crate) extractor: Arc<Extractor>,
    #[allow(clippy::type_complexity)]
    pub(crate) additional: RwLock<Vec<(TypeId, NonNull<u8>, Arc<Extractor>)>>,
}

#[repr(transparent)]
pub struct EntityData {
    inner: NonNull<EntityDataInner>,
}

unsafe impl Send for EntityData {}
unsafe impl Sync for EntityData {}

impl EntityData {
    pub(crate) fn inner(&self) -> &EntityDataInner {
        unsafe { self.inner.as_ref() }
    }

    pub(crate) fn new<E: crate::Extractable>(entity: E, extractor: Arc<Extractor>) -> Self {
        let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
        let inner = EntityDataInner {
            data: unsafe { NonNull::new_unchecked(ptr) },
            counter: AtomicUsize::new(1),
            extractor,
            additional: RwLock::new(Vec::new()),
        };
        Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(inner))) },
        }
    }

    pub(crate) fn extract<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        let extracted = unsafe { self.extract_ptr::<T>()? };
        Some(crate::Acquirable::new(extracted, self.clone()))
    }

    pub(crate) unsafe fn extract_ptr<T: 'static>(&self) -> Option<NonNull<T>> {
        unsafe { self.inner().extractor.extract_ptr::<T>(self.inner().data) }
    }

    pub(crate) fn add_additional<E: Extractable>(&self, data: E) {
        let ptr = {
            let boxed = Box::into_raw(Box::new(data)) as *mut u8;
            unsafe { NonNull::new_unchecked(boxed) }
        };

        let inner = self.inner();
        let type_id = TypeId::of::<E>();
        let extractor = Arc::new(Extractor::new::<E>());

        let mut additionals = inner.additional.write();

        // Check if already exists and replace
        if let Some(existing) = additionals.iter_mut().find(|(tid, _, _)| *tid == type_id) {
            // Drop old data properly
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
        let (_, ptr, _) = additionals.swap_remove(pos);

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
        unsafe { (inner.extractor.dropper)(inner.data) };

        // Drop all additional data
        let additionals = inner.additional.read();
        for (_, ptr, extractor) in additionals.iter() {
            unsafe {
                (extractor.dropper)(*ptr);
            }
        }

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
