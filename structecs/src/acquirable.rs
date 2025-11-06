use std::{ops::Deref, ptr::NonNull};

use crate::entity::EntityData;

/// A smart pointer to a component that keeps the entity data alive.
///
/// Implements `Deref` for transparent access to the component.
pub struct Acquirable<T: 'static> {
    target: NonNull<T>,
    pub(crate) inner: EntityData,
}

impl<T: 'static> Clone for Acquirable<T> {
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            inner: self.inner.clone(),
        }
    }
}

impl<T: 'static> AsRef<T> for Acquirable<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        // SAFETY: target points to valid T within the entity data,
        // which is kept alive by the inner EntityData.
        unsafe { self.target.as_ref() }
    }
}

impl<T: 'static> Deref for Acquirable<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: 'static> Acquirable<T> {
    #[inline(always)]
    pub(crate) fn new(target: NonNull<T>, inner: EntityData) -> Self {
        Self { target, inner }
    }

    #[inline(always)]
    pub(crate) unsafe fn new_target(inner: EntityData) -> Self {
        // SAFETY: The caller must ensure that T is the correct type for the entity data.
        // Typically called after type checking via Extractor.
        Acquirable::new(inner.data_ptr().cast::<T>(), inner)
    }

    /// Extract a different component type from the same entity.
    pub fn extract<U: 'static>(&self) -> Option<Acquirable<U>> {
        // SAFETY: extract_ptr performs type checking via the Extractor
        // and only returns a pointer if type U exists in the entity.
        let extracted = unsafe { self.inner.extract_ptr::<U>()? };
        Some(Acquirable::new(extracted, self.inner.clone()))
    }
}

unsafe impl<T: 'static + Send + Sync> Send for Acquirable<T> where T: Send {}
unsafe impl<T: 'static + Send + Sync> Sync for Acquirable<T> where T: Sync {}
