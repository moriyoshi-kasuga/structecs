use std::{ops::Deref, ptr::NonNull};

use crate::entity::EntityDataInner;

/// A smart pointer to a component that keeps the entity data alive.
///
/// Implements `Deref` for transparent access to the component.
pub struct Acquirable<T: 'static> {
    target: NonNull<T>,
    inner: EntityDataInner,
}

impl<T: 'static> AsRef<T> for Acquirable<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.target.as_ref() }
    }
}

impl<T: 'static> Deref for Acquirable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: 'static> Acquirable<T> {
    pub(crate) fn new(target: NonNull<T>, inner: EntityDataInner) -> Self {
        Self { target, inner }
    }

    /// Extract a different component type from the same entity.
    pub fn extract<U: 'static>(&self) -> Option<Acquirable<U>> {
        let extracted = unsafe { self.inner.extract_ptr::<U>()? };
        Some(Acquirable::new(extracted, self.inner.clone()))
    }
}

// Safe to send across threads since we use atomic reference counting
unsafe impl<T: 'static> Send for Acquirable<T> where T: Send {}
unsafe impl<T: 'static> Sync for Acquirable<T> where T: Sync {}
