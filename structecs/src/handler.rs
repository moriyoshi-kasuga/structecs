use std::{ptr::NonNull, sync::Arc};

use crate::{Acquirable, Extractable, ExtractionMetadata};

struct UnTraitFn<E: Extractable, Args, Return> {
    #[allow(clippy::type_complexity)]
    func: Box<dyn Fn(&Acquirable<E>, Args) -> Return>,
}

impl<E: Extractable, Args, Return> UnTraitFn<E, Args, Return> {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&Acquirable<E>, Args) -> Return + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }

    pub fn call(&self, entity: &Acquirable<E>, args: Args) -> Return {
        (self.func)(entity, args)
    }
}

struct TypeErasedFn {
    ptr: NonNull<u8>,
    dropper: unsafe fn(NonNull<u8>),
}

impl TypeErasedFn {
    pub fn new<F, E: Extractable, Args, Return>(func: F) -> Self
    where
        F: Fn(&Acquirable<E>, Args) -> Return + 'static,
    {
        let untrait_fn = UnTraitFn::new(func);
        let dropper = |ptr: NonNull<u8>| unsafe {
            let boxed: Box<UnTraitFn<E, Args, Return>> =
                Box::from_raw(ptr.as_ptr() as *mut UnTraitFn<E, Args, Return>);
            drop(boxed);
        };
        let boxed: Box<UnTraitFn<E, Args, Return>> = Box::new(untrait_fn);
        let raw_ptr = Box::into_raw(boxed) as *mut u8;
        Self {
            ptr: unsafe { NonNull::new_unchecked(raw_ptr) },
            dropper,
        }
    }

    pub fn call<E: Extractable, Args, Return>(&self, entity: &Acquirable<E>, args: Args) -> Return {
        let function = unsafe { &*(self.ptr.as_ptr() as *const UnTraitFn<E, Args, Return>) };
        function.call(entity, args)
    }
}

impl Drop for TypeErasedFn {
    fn drop(&mut self) {
        unsafe { (self.dropper)(self.ptr) };
    }
}

unsafe impl Send for TypeErasedFn {}
unsafe impl Sync for TypeErasedFn {}

pub struct ComponentHandler<S: Extractable, Args, Return> {
    function: Arc<TypeErasedFn>,
    _marker: std::marker::PhantomData<(S, Args, Return)>,
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

        let function = TypeErasedFn::new(func);
        Self {
            function: Arc::new(function),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn call<E: Extractable>(&self, entity: &Acquirable<E>, args: Args) -> Return {
        self.function.call(entity, args)
    }
}

impl<S: Extractable, Args, Return> Clone for ComponentHandler<S, Args, Return> {
    fn clone(&self) -> Self {
        Self {
            function: Arc::clone(&self.function),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: Extractable, Args, Return> std::fmt::Debug for ComponentHandler<S, Args, Return> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentHandler").finish()
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
