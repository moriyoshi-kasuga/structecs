use std::{
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::extractor::Extractor;

/// Unique identifier for an entity in the World.
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct EntityId {
    pub(crate) id: u32,
}

impl EntityId {
    pub(crate) fn new(id: u32) -> Self {
        Self { id }
    }

    /// Get the raw ID value
    pub fn id(&self) -> u32 {
        self.id
    }
}

/// Internal reference-counted data for an entity.
pub(crate) struct EntityDataInner {
    pub(crate) data: NonNull<u8>,
    pub(crate) counter: NonNull<AtomicUsize>,
    pub(crate) extractor: Arc<Extractor>,
}

unsafe impl Send for EntityDataInner {}
unsafe impl Sync for EntityDataInner {}

impl Drop for EntityDataInner {
    fn drop(&mut self) {
        unsafe {
            if self.counter.as_ref().fetch_sub(1, Ordering::Release) > 1 {
                return;
            }
        }
        std::sync::atomic::fence(Ordering::Acquire);
        unsafe { (self.extractor.dropper)(self.data) };
        unsafe { drop(Box::from_raw(self.counter.as_ptr())) };
    }
}

impl Clone for EntityDataInner {
    fn clone(&self) -> Self {
        unsafe {
            self.counter.as_ref().fetch_add(1, Ordering::Relaxed);
        }
        Self {
            data: self.data,
            counter: self.counter,
            extractor: Arc::clone(&self.extractor),
        }
    }
}

impl EntityDataInner {
    pub(crate) unsafe fn extract_ptr<T: 'static>(&self) -> Option<NonNull<T>> {
        unsafe { self.extractor.extract_ptr::<T>(self.data) }
    }
}

/// Wrapper around entity data that provides component extraction.
pub struct EntityData {
    pub(crate) inner: EntityDataInner,
}

impl EntityData {
    pub(crate) fn new<E: crate::Extractable>(entity: E, extractor: Arc<Extractor>) -> Self {
        let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
        let inner = EntityDataInner {
            data: unsafe { NonNull::new_unchecked(ptr) },
            counter: Box::leak(Box::new(AtomicUsize::new(1))).into(),
            extractor,
        };
        Self { inner }
    }

    pub fn extract<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        let extracted = unsafe { self.inner.extract_ptr::<T>()? };
        Some(crate::Acquirable::new(extracted, self.inner.clone()))
    }
}
