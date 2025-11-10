use std::{any::TypeId, sync::LazyLock};

use rustc_hash::FxHashMap;
// Re-export the derive macro
pub use structecs_macros::Extractable;

// Module declarations
mod acquirable;
mod entity;
mod extractable;
mod extractor;
mod handler;

// Public exports
pub use acquirable::Acquirable;
pub use extractable::{Extractable, ExtractableType, ExtractionMetadata};
pub use handler::ComponentHandler;

pub mod __private {
    // Re-export inventory submit for use in derive macros
    pub use inventory::submit;
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

#[test]
fn sample_usage() {
    use crate as structecs;
    use crate::*;

    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(entity)]
    struct NamedEntity {
        name: String,
        entity: Entity,
    }

    let named = NamedEntity {
        name: "Test".to_string(),
        entity: Entity { id: 42 },
    };
    let acquirable = Acquirable::new(named);
    let extracted_entity = acquirable.extract::<Entity>().unwrap();
    assert_eq!(*extracted_entity, Entity { id: 42 });
}
