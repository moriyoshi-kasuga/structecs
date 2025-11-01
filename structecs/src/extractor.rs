use std::{any::TypeId, collections::HashMap, ptr::NonNull};

use crate::{Extractable, ExtractionMetadata};

/// Extracts components from entity data using pre-computed offsets.
pub struct Extractor {
    pub(crate) offsets: HashMap<TypeId, usize>,
    pub(crate) dropper: unsafe fn(NonNull<u8>),
}

impl Extractor {
    /// Create a new extractor for the given extractable type.
    pub(crate) fn new<E: Extractable>() -> Self {
        Self {
            offsets: ExtractionMetadata::flatten(E::METADATA_LIST)
                .into_iter()
                .collect(),
            dropper: |ptr| unsafe {
                drop(Box::from_raw(ptr.as_ptr() as *mut E));
            },
        }
    }

    /// Extract a reference to a component of type T from entity data.
    pub fn extract<T: 'static>(&self, data: NonNull<u8>) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let offset = self.offsets.get(&type_id)?;
        let ptr = unsafe { data.as_ptr().add(*offset) as *const T };
        Some(unsafe { &*ptr })
    }

    /// Extract a pointer to a component of type T from entity data.
    ///
    /// # Safety
    /// The caller must ensure the pointer is used correctly and not outlive the entity data.
    pub unsafe fn extract_ptr<T: 'static>(&self, data: NonNull<u8>) -> Option<NonNull<T>> {
        let type_id = TypeId::of::<T>();
        let offset = self.offsets.get(&type_id)?;
        Some(unsafe { data.add(*offset).cast::<T>() })
    }

    /// Extract a reference to a component, panicking if not found.
    ///
    /// # Safety
    /// This function assumes that the type T is present in the extractor's offsets.
    pub unsafe fn extract_unchecked<T: 'static>(&self, data: NonNull<u8>) -> &T {
        let type_id = TypeId::of::<T>();
        let offset = self.offsets.get(&type_id).unwrap();
        unsafe {
            let ptr = data.as_ptr().add(*offset) as *const T;
            &*ptr
        }
    }

    /// Check if this extractor can extract a component of type T.
    #[inline]
    pub fn has_component<T: 'static>(&self) -> bool {
        self.offsets.contains_key(&TypeId::of::<T>())
    }
}
