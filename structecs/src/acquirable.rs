use std::{ops::Deref, ptr::NonNull};

use crate::{Extractable, entity::EntityData};

/// A smart pointer to a component that keeps the entity data alive.
///
/// Implements `Deref` for transparent access to the component.
pub struct Acquirable<T: Extractable> {
    target: NonNull<T>,
    pub(crate) inner: EntityData,
}

impl<T: Extractable> Clone for Acquirable<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            inner: self.inner.clone(),
        }
    }
}

impl<T: Extractable> AsRef<T> for Acquirable<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        // SAFETY: target points to valid T within the entity data,
        // which is kept alive by the inner EntityData.
        unsafe { self.target.as_ref() }
    }
}

impl<T: Extractable> Deref for Acquirable<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Extractable> Acquirable<T> {
    pub fn new(target: T) -> Self {
        let data = EntityData::new(target, crate::get_extractor::<T>());
        // SAFETY: The extractor for T guarantees that T is at offset 0.
        unsafe { data.extract_by_offset::<T>(0) }
    }

    #[inline(always)]
    pub(crate) fn new_raw(target: NonNull<T>, inner: EntityData) -> Self {
        Self { target, inner }
    }

    /// Extract a different component type from the same entity.
    #[inline(always)]
    pub fn extract<U: Extractable>(&self) -> Option<Acquirable<U>> {
        // SAFETY: extract_ptr performs type checking via the Extractor
        // and only returns a pointer if type U exists in the entity.
        let extracted = unsafe { self.inner.extract_ptr::<U>()? };
        Some(Acquirable::new_raw(extracted, self.inner.clone()))
    }
}

unsafe impl<T: Extractable + Send + Sync> Send for Acquirable<T> where T: Send {}
unsafe impl<T: Extractable + Send + Sync> Sync for Acquirable<T> where T: Sync {}
