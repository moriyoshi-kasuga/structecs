use std::{any::TypeId, ptr::NonNull};

use rustc_hash::FxHashMap;

/// Trait for types that can be extracted from entity data.
///
/// This is typically derived using `#[derive(Extractable)]`.
pub trait Extractable: 'static + Sized {
    /// Metadata describing how to extract components from this type.
    const METADATA_LIST: &'static [ExtractionMetadata];
    #[cfg(debug_assertions)]
    const IDENTIFIER: &'static str;
}

pub struct ExtractableType {
    pub type_id: TypeId,
    pub metadata: &'static [ExtractionMetadata],
    pub dropper: unsafe fn(NonNull<u8>),
}

impl ExtractableType {
    pub const fn new<T: Extractable>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            metadata: T::METADATA_LIST,
            dropper: |data_ptr: NonNull<u8>| {
                // SAFETY: The caller guarantees that data_ptr points to a valid instance of T.
                unsafe {
                    let boxed: Box<T> = Box::from_raw(data_ptr.as_ptr() as *mut T);
                    drop(boxed);
                }
            },
        }
    }
}

inventory::collect!(ExtractableType);

/// Metadata describing how to extract types from an entity structure.
pub enum ExtractionMetadata {
    /// Direct target at a specific offset.
    Target {
        type_id: TypeId,
        offset: usize,

        #[cfg(debug_assertions)]
        identifier: &'static str,
    },
    /// Nested extractable type with its own metadata.
    Nested {
        type_id: TypeId,
        offset: usize,
        nested: &'static [ExtractionMetadata],

        #[cfg(debug_assertions)]
        identifier: &'static str,
    },
}

impl ExtractionMetadata {
    /// Create metadata for a direct target type.
    #[inline]
    pub const fn new<T: Extractable>(offset: usize) -> Self {
        Self::Target {
            type_id: TypeId::of::<T>(),
            offset,
            #[cfg(debug_assertions)]
            identifier: T::IDENTIFIER,
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
            #[cfg(debug_assertions)]
            identifier: T::IDENTIFIER,
        }
    }

    /// 呼ぶ際にpanicをするさいには、それぞれの関数の一番最初にそれぞれが使用するべき。
    /// constのpanicはtrack_callerなど意味がないので、public API関数の最初に置くのが良い。
    pub const fn is_has<List: Extractable, Target: Extractable>() -> bool {
        let list = List::METADATA_LIST;
        let target = Target::IDENTIFIER;
        let mut idx = 0;
        while list.len() > idx {
            if list[idx].has_val(target) {
                return true;
            }
            idx += 1;
        }
        false
    }

    #[cfg(debug_assertions)]
    pub const fn has_val(&self, identifier: &str) -> bool {
        const fn eq_str(a: &str, b: &str) -> bool {
            let a_bytes = a.as_bytes();
            let b_bytes = b.as_bytes();
            if a_bytes.len() != b_bytes.len() {
                return false;
            }
            let mut idx = 0;
            while idx < a_bytes.len() {
                if a_bytes[idx] != b_bytes[idx] {
                    return false;
                }
                idx += 1;
            }
            true
        }

        match self {
            ExtractionMetadata::Target { identifier: id, .. } => eq_str(id, identifier),
            ExtractionMetadata::Nested {
                identifier: id,
                nested,
                ..
            } => {
                if eq_str(id, identifier) {
                    true
                } else {
                    let mut idx = 0;
                    while nested.len() > idx {
                        if nested[idx].has_val(identifier) {
                            return true;
                        }
                        idx += 1;
                    }
                    false
                }
            }
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
                ExtractionMetadata::Target {
                    type_id, offset, ..
                } => {
                    result.insert(*type_id, base_offset + *offset);
                }
                ExtractionMetadata::Nested {
                    type_id,
                    offset,
                    nested,
                    ..
                } => {
                    result.insert(*type_id, base_offset + *offset);
                    Self::flatten_internal(nested, base_offset + *offset, result);
                }
            }
        }
    }
}
