use std::{any::TypeId, ptr::NonNull};

use rustc_hash::FxHashMap;

use crate::{Extractable, ExtractionMetadata};

/// Extracts components from entity data using pre-computed offsets.
pub struct Extractor {
    pub(crate) offsets: FxHashMap<TypeId, usize>,
    pub(crate) dropper: unsafe fn(NonNull<u8>),
}

impl Extractor {
    /// Create a new extractor for the given extractable type.
    pub(crate) fn new<E: Extractable>() -> Self {
        Self {
            offsets: ExtractionMetadata::flatten(E::METADATA_LIST)
                .into_iter()
                .collect(),
            dropper: |ptr| {
                // SAFETY: The pointer was created from Box::into_raw with type E,
                // so it's safe to reconstruct and drop the Box<E>.
                unsafe {
                    drop(Box::from_raw(ptr.as_ptr() as *mut E));
                }
            },
        }
    }

    /// Extract a pointer to a component of type T from entity data.
    ///
    /// # Safety
    /// The caller must ensure the pointer is used correctly and not outlive the entity data.
    pub(crate) unsafe fn extract_ptr<T: 'static>(&self, data: NonNull<u8>) -> Option<NonNull<T>> {
        let type_id = TypeId::of::<T>();
        let offset = self.offsets.get(&type_id)?;
        // SAFETY: The offset is valid for type T and was computed during type analysis.
        // The data pointer points to the base of the entity data.
        Some(unsafe { data.add(*offset).cast::<T>() })
    }

    pub(crate) fn offset(&self, type_id: &TypeId) -> Option<usize> {
        self.offsets.get(type_id).copied()
    }

    pub(crate) fn type_ids(&self) -> impl Iterator<Item = &TypeId> {
        self.offsets.keys()
    }
}
