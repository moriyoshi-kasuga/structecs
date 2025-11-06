use std::any::TypeId;

use rustc_hash::FxHashMap;

/// Trait for types that can be extracted from entity data.
///
/// This is typically derived using `#[derive(Extractable)]`.
pub trait Extractable: 'static + Sized {
    /// Metadata describing how to extract components from this type.
    const METADATA_LIST: &'static [ExtractionMetadata];
}

/// Metadata describing how to extract types from an entity structure.
pub enum ExtractionMetadata {
    /// Direct target at a specific offset.
    Target { type_id: TypeId, offset: usize },
    /// Nested extractable type with its own metadata.
    Nested {
        type_id: TypeId,
        offset: usize,
        nested: &'static [ExtractionMetadata],
    },
}

impl ExtractionMetadata {
    /// Create metadata for a direct target type.
    #[inline]
    pub const fn new<T: 'static>(offset: usize) -> Self {
        Self::Target {
            type_id: TypeId::of::<T>(),
            offset,
        }
    }

    /// Create metadata for a nested extractable type.
    #[inline]
    pub const fn new_nested<T: crate::Extractable>(
        offset: usize,
        nested: &'static [ExtractionMetadata],
    ) -> Self {
        Self::Nested {
            type_id: TypeId::of::<T>(),
            offset,
            nested,
        }
    }

    /// Flatten nested metadata into a single HashMap of type -> offset mappings.
    #[inline]
    pub fn flatten(list: &[ExtractionMetadata]) -> FxHashMap<TypeId, usize> {
        let mut result = FxHashMap::default();
        Self::flatten_internal(list, 0, &mut result);
        result
    }

    fn flatten_internal(
        list: &[ExtractionMetadata],
        base_offset: usize,
        result: &mut FxHashMap<TypeId, usize>,
    ) {
        for metadata in list {
            match metadata {
                ExtractionMetadata::Target { type_id, offset } => {
                    result.insert(*type_id, base_offset + *offset);
                }
                ExtractionMetadata::Nested {
                    type_id,
                    offset,
                    nested,
                } => {
                    result.insert(*type_id, base_offset + *offset);
                    Self::flatten_internal(nested, base_offset + *offset, result);
                }
            }
        }
    }
}
