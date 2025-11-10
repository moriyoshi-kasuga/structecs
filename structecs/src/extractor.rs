use std::{any::TypeId, ptr::NonNull};

use rustc_hash::FxHashMap;

use crate::{ExtractionMetadata, extractable::ExtractableType};

/// Extracts components from entity data using pre-computed offsets.
pub struct Extractor {
    pub(crate) offsets: FxHashMap<TypeId, usize>,
    pub(crate) dropper: unsafe fn(NonNull<u8>),
}

impl Extractor {
    pub(crate) fn new_type(target: &ExtractableType) -> Self {
        Self {
            offsets: ExtractionMetadata::flatten(target.metadata),
            dropper: target.dropper,
        }
    }

    /// Extract a pointer to a component of type T from entity data.
    ///
    /// # Safety
    /// The caller must ensure the pointer is used correctly and not outlive the entity data.
    #[inline(always)]
    pub(crate) unsafe fn extract_ptr<T: 'static>(&self, data: NonNull<u8>) -> Option<NonNull<T>> {
        let type_id = const { TypeId::of::<T>() };
        let offset = self.offsets.get(&type_id)?;
        // SAFETY: The offset is valid for type T and was computed during type analysis.
        // The data pointer points to the base of the entity data.
        Some(unsafe { data.add(*offset).cast::<T>() })
    }
}
