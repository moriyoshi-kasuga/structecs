use crate::{Acquirable, Extractable, ExtractionMetadata, entity::EntityData};

struct TypeErasedFn<Args, Return> {
    caller: Box<dyn Fn(EntityData, Args) -> Return>,
    #[cfg(debug_assertions)]
    type_name: String,
}

impl<Args, Return> TypeErasedFn<Args, Return> {
    pub fn new<F, E: Extractable>(func: F) -> Self
    where
        F: Fn(&Acquirable<E>, Args) -> Return + 'static,
    {
        let caller = move |data: EntityData, args: Args| -> Return {
            #[allow(clippy::expect_used)]
            // SAFETY: We ensure that E is extractable from the EntityData when creating the TypeErasedFn
            let e = unsafe { &data.extract::<E>().unwrap_unchecked() };
            func(e, args)
        };
        Self {
            caller: Box::new(caller),
            #[cfg(debug_assertions)]
            type_name: format!(
                "impl Fn(&Acquirable<{}>, {}) -> {}",
                std::any::type_name::<E>(),
                std::any::type_name::<Args>(),
                std::any::type_name::<Return>()
            ),
        }
    }

    pub fn call<E: Extractable>(&self, entity: &Acquirable<E>, args: Args) -> Return {
        (self.caller)(entity.inner.clone(), args)
    }
}

unsafe impl<Args, Return> Send for TypeErasedFn<Args, Return> {}
unsafe impl<Args, Return> Sync for TypeErasedFn<Args, Return> {}

pub struct ComponentHandler<S: Extractable, Args = (), Return = ()> {
    function: TypeErasedFn<Args, Return>,
    _marker: std::marker::PhantomData<S>,
}

impl<S: Extractable, Args, Return> ComponentHandler<S, Args, Return> {
    pub fn new<E: Extractable>(func: impl Fn(&Acquirable<E>, Args) -> Return + 'static) -> Self {
        #[cfg(debug_assertions)]
        if !is_e_has_s::<E, S>() {
            panic!(
                "Type S ({}) does not contain type E ({}) in its extraction metadata.",
                std::any::type_name::<S>(),
                std::any::type_name::<E>(),
            );
        }

        Self {
            function: TypeErasedFn::new(func),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn call<E: Extractable>(&self, entity: &Acquirable<E>, args: Args) -> Return {
        #[cfg(debug_assertions)]
        if !is_e_has_s::<E, S>() {
            panic!(
                "Type S ({}) does not contain type E ({}) in its extraction metadata.",
                std::any::type_name::<S>(),
                std::any::type_name::<E>(),
            );
        }

        self.function.call(entity, args)
    }
}

impl<S: Extractable, Args, Return> std::fmt::Debug for ComponentHandler<S, Args, Return> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ComponentHandler");
        #[cfg(debug_assertions)]
        debug.field("function", &self.function.type_name);
        #[cfg(not(debug_assertions))]
        debug.field("function", &"<type erased>");
        debug.finish()
    }
}

#[cfg(debug_assertions)]
fn search(list: &[ExtractionMetadata], target: std::any::TypeId) -> bool {
    let mut idx = 0;
    while idx < list.len() {
        let metadata = &list[idx];
        match metadata {
            ExtractionMetadata::Target { type_id, .. } => {
                if target == *type_id {
                    return true;
                }
            }
            ExtractionMetadata::Nested {
                type_id, nested, ..
            } => {
                if target == *type_id || search(nested, target) {
                    return true;
                }
            }
        }
        idx += 1;
    }
    false
}

/// [`std::any::TypeId`] is not const evaluable yet, so we can't make this a const fn
#[cfg(debug_assertions)]
fn is_e_has_s<E: Extractable, S: Extractable>() -> bool {
    let s_type_id = std::any::TypeId::of::<S>();

    search(E::METADATA_LIST, s_type_id)
}
