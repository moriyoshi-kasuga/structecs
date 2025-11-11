#![doc = include_str!("../../README.md")]

use std::{any::TypeId, sync::LazyLock};

use rustc_hash::FxHashMap;
// Re-export the derive macro
pub use structecs_macros::Extractable;

// Module declarations
mod acquirable;
#[cfg(feature = "archetype")]
mod archetype;
mod entity;
mod extractable;
mod extractor;
mod handler;

// Public exports
pub use acquirable::{Acquirable, WeakAcquirable};
#[cfg(feature = "archetype")]
pub use archetype::Archetype;
pub use extractable::{Extractable, ExtractableType, ExtractionMetadata};
pub use handler::ComponentHandler;

pub mod __private {
    // Re-export inventory submit for use in derive macros
    pub use inventory::submit;

    pub const fn concat_str<const TOTAL: usize>(a: &'static str, b: &'static str) -> [u8; TOTAL] {
        let bytes_a = a.as_bytes();
        let bytes_b = b.as_bytes();
        let mut buffer = [0u8; TOTAL];
        let mut i = 0;
        while i < bytes_a.len() {
            buffer[i] = bytes_a[i];
            i += 1;
        }

        buffer[i] = b':';
        i += 1;
        buffer[i] = b':';
        i += 1;

        let mut j = 0;
        while j < bytes_b.len() {
            buffer[i + j] = bytes_b[j];
            j += 1;
        }
        buffer
    }
}

pub static GLOBAL_EXTRACTOR_CACHE: LazyLock<FxHashMap<TypeId, extractor::Extractor>> =
    LazyLock::new(|| {
        inventory::iter::<extractable::ExtractableType>
            .into_iter()
            .map(|extractable| {
                (
                    extractable.type_id,
                    extractor::Extractor::new_type(extractable),
                )
            })
            .collect()
    });

pub(crate) fn get_extractor<E: extractable::Extractable>() -> &'static extractor::Extractor {
    let type_id = TypeId::of::<E>();
    // SAFETY: The GLOBAL_EXTRACTOR_CACHE is populated at program start with all
    // extractable types via inventory, so the unwrap_unchecked is safe here.
    unsafe { GLOBAL_EXTRACTOR_CACHE.get(&type_id).unwrap_unchecked() }
}
