use std::{ptr::NonNull, sync::Arc};

use crate::{Extractable, extractor::Extractor};

#[repr(C)]
pub(crate) struct EntityDataInner {
    /// Pointer to the entity data
    pub(crate) data: NonNull<u8>,

    /// Extractor for component access
    pub(crate) extractor: &'static Extractor,
}

unsafe impl Send for EntityDataInner {}
unsafe impl Sync for EntityDataInner {}

impl Drop for EntityDataInner {
    fn drop(&mut self) {
        unsafe { (self.extractor.dropper)(self.data) };
    }
}

#[derive(Clone)]
pub struct EntityData {
    inner: Arc<EntityDataInner>,
}

impl EntityData {
    pub(crate) fn new<E: crate::Extractable>(entity: E, extractor: &'static Extractor) -> Self {
        let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
        let inner = EntityDataInner {
            // SAFETY: Box::into_raw never returns null
            data: unsafe { NonNull::new_unchecked(ptr) },
            extractor,
        };
        Self {
            inner: Arc::new(inner),
        }
    }

    #[inline]
    pub(crate) unsafe fn extract_by_offset<T: Extractable>(
        &self,
        offset: usize,
    ) -> crate::Acquirable<T> {
        // SAFETY: The caller guarantees that offset is valid for type T within the entity data.
        // The offset comes from the Extractor which validates it during creation.
        let extracted = unsafe { self.inner.data.add(offset).cast::<T>() };
        crate::Acquirable::new_raw(extracted, self.clone())
    }

    #[inline(always)]
    pub(crate) fn extract<T: Extractable>(&self) -> Option<crate::Acquirable<T>> {
        // SAFETY: extract_ptr validates the type through the Extractor
        let extracted = unsafe { self.extract_ptr::<T>()? };
        Some(crate::Acquirable::new_raw(extracted, self.clone()))
    }

    #[inline(always)]
    pub(crate) unsafe fn extract_ptr<T: 'static>(&self) -> Option<NonNull<T>> {
        // SAFETY: The caller must ensure proper synchronization. The extractor validates
        // that type T exists in the entity data and returns None if not present.
        unsafe { self.inner.extractor.extract_ptr::<T>(self.inner.data) }
    }
}
