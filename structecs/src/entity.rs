use std::{ptr::NonNull, sync::Arc};

use crate::{Extractable, extractor::Extractor};

impl Drop for EntityData {
    fn drop(&mut self) {
        unsafe { (self.extractor.dropper)(self.data) };
    }
}

unsafe impl Send for EntityData {}
unsafe impl Sync for EntityData {}

#[derive(Clone)]
pub struct EntityData {
    /// Pointer to the entity data
    pub(crate) data: NonNull<u8>,

    /// Extractor for component access
    pub(crate) extractor: &'static Extractor,
}

impl EntityData {
    pub(crate) fn new<E: crate::Extractable>(entity: E, extractor: &'static Extractor) -> Self {
        let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
        Self {
            data: unsafe { NonNull::new_unchecked(ptr) },
            extractor,
        }
    }

    #[inline(always)]
    pub(crate) fn extract<T: Extractable>(self: &Arc<Self>) -> Option<crate::Acquirable<T>> {
        // SAFETY: extract_ptr validates the type through the Extractor
        let extracted = unsafe { self.extract_ptr::<T>()? };
        Some(crate::Acquirable::new_raw(extracted, self.clone()))
    }

    #[inline(always)]
    pub(crate) unsafe fn extract_ptr<T: 'static>(&self) -> Option<NonNull<T>> {
        // SAFETY: The caller must ensure proper synchronization. The extractor validates
        // that type T exists in the entity data and returns None if not present.
        unsafe { self.extractor.extract_ptr::<T>(self.data) }
    }
}
