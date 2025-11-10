use std::{
    ops::Deref,
    ptr::NonNull,
    sync::{Arc, Weak},
};

use crate::{Extractable, entity::EntityData};

/// A smart pointer to a component that keeps the entity data alive.
///
/// `Acquirable<T>` provides transparent access to component `T` through `Deref`,
/// while maintaining ownership of the underlying entity data via reference counting.
///
/// # Examples
///
/// ```
/// use structecs::*;
///
/// #[derive(Extractable)]
/// struct Player {
///     name: String,
///     health: u32,
/// }
///
/// let player = Acquirable::new(Player {
///     name: "Alice".to_string(),
///     health: 100,
/// });
///
/// // Access via Deref
/// assert_eq!(player.name, "Alice");
/// ```
pub struct Acquirable<T: Extractable> {
    target: NonNull<T>,
    pub(crate) inner: Arc<EntityData>,
}

/// A weak reference to an entity's component.
///
/// `WeakAcquirable<T>` does not keep the entity alive and must be upgraded
/// to an `Acquirable<T>` to access the component data.
///
/// This is useful for preventing circular references and implementing
/// cache-like structures.
///
/// # Examples
///
/// ```
/// use structecs::*;
///
/// #[derive(Extractable)]
/// struct Entity {
///     id: u32,
/// }
///
/// let entity = Acquirable::new(Entity { id: 42 });
/// let weak = entity.downgrade();
///
/// // Entity is still alive
/// assert!(weak.upgrade().is_some());
///
/// drop(entity);
///
/// // Entity has been dropped
/// assert!(weak.upgrade().is_none());
/// ```
pub struct WeakAcquirable<T: Extractable> {
    inner: Weak<EntityData>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Extractable> Clone for Acquirable<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            inner: self.inner.clone(),
        }
    }
}

impl<T: Extractable> Deref for Acquirable<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { self.target.as_ref() }
    }
}

impl<T: Extractable> Acquirable<T> {
    pub fn new(target: T) -> Self {
        let data = Arc::new(EntityData::new(target, crate::get_extractor::<T>()));
        // SAFETY: The extractor for T guarantees that T is at offset 0.
        unsafe { data.extract_by_offset::<T>(0) }
    }

    #[inline(always)]
    pub(crate) fn new_raw(target: NonNull<T>, inner: Arc<EntityData>) -> Self {
        Self { target, inner }
    }

    /// Extract a different component type from the same entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Extractable)]
    /// struct Health {
    ///     value: u32,
    /// }
    ///
    /// #[derive(Extractable)]
    /// #[extractable(health)]
    /// struct Player {
    ///     name: String,
    ///     health: Health,
    /// }
    ///
    /// let player = Acquirable::new(Player {
    ///     name: "Alice".to_string(),
    ///     health: Health { value: 100 },
    /// });
    ///
    /// let health = player.extract::<Health>().unwrap();
    /// assert_eq!(health.value, 100);
    /// ```
    #[inline(always)]
    pub fn extract<U: Extractable>(&self) -> Option<Acquirable<U>> {
        // SAFETY: extract_ptr performs type checking via the Extractor
        // and only returns a pointer if type U exists in the entity.
        let extracted = unsafe { self.inner.extract_ptr::<U>()? };
        Some(Acquirable::new_raw(extracted, self.inner.clone()))
    }

    /// Create a weak reference to this entity's component.
    ///
    /// The weak reference does not keep the entity alive and can be upgraded
    /// back to an `Acquirable` if the entity still exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Extractable)]
    /// struct Entity {
    ///     id: u32,
    /// }
    ///
    /// let entity = Acquirable::new(Entity { id: 42 });
    /// let weak = entity.downgrade();
    ///
    /// assert!(weak.upgrade().is_some());
    /// ```
    #[inline(always)]
    pub fn downgrade(&self) -> WeakAcquirable<T> {
        WeakAcquirable {
            inner: Arc::downgrade(&self.inner),
            _marker: std::marker::PhantomData,
        }
    }

    /// Check if two `Acquirable` pointers point to the same entity data.
    ///
    /// # Examples
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Extractable)]
    /// struct Entity {
    ///     id: u32,
    /// }
    ///
    /// let entity1 = Acquirable::new(Entity { id: 42 });
    /// let entity2 = entity1.clone();
    /// let entity3 = Acquirable::new(Entity { id: 42 });
    ///
    /// assert!(entity1.ptr_eq(&entity2));
    /// assert!(!entity1.ptr_eq(&entity3));
    /// ```
    #[inline(always)]
    pub fn ptr_eq<U: Extractable>(&self, other: &Acquirable<U>) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Get the number of strong references to the entity data.
    ///
    /// This is only available in debug builds for debugging purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Extractable)]
    /// struct Entity {
    ///     id: u32,
    /// }
    ///
    /// let entity = Acquirable::new(Entity { id: 42 });
    ///
    /// #[cfg(debug_assertions)]
    /// {
    ///     assert_eq!(entity.strong_count(), 1);
    ///     let entity2 = entity.clone();
    ///     assert_eq!(entity.strong_count(), 2);
    /// }
    /// ```
    #[cfg(debug_assertions)]
    #[inline(always)]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// Get the number of weak references to the entity data.
    ///
    /// This is only available in debug builds for debugging purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Extractable)]
    /// struct Entity {
    ///     id: u32,
    /// }
    ///
    /// let entity = Acquirable::new(Entity { id: 42 });
    ///
    /// #[cfg(debug_assertions)]
    /// {
    ///     assert_eq!(entity.weak_count(), 0);
    ///     let weak = entity.downgrade();
    ///     assert_eq!(entity.weak_count(), 1);
    /// }
    /// ```
    #[cfg(debug_assertions)]
    #[inline(always)]
    pub fn weak_count(&self) -> usize {
        Arc::weak_count(&self.inner)
    }
}

unsafe impl<T: Extractable + Send + Sync> Send for Acquirable<T> where T: Send {}
unsafe impl<T: Extractable + Send + Sync> Sync for Acquirable<T> where T: Sync {}

impl<T: Extractable> WeakAcquirable<T> {
    /// Upgrade the weak reference to an `Acquirable` if the entity is still alive.
    ///
    /// Returns `None` if the entity has been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Extractable)]
    /// struct Entity {
    ///     id: u32,
    /// }
    ///
    /// let entity = Acquirable::new(Entity { id: 42 });
    /// let weak = entity.downgrade();
    ///
    /// // Entity is still alive
    /// assert!(weak.upgrade().is_some());
    ///
    /// drop(entity);
    ///
    /// // Entity has been dropped
    /// assert!(weak.upgrade().is_none());
    /// ```
    #[inline(always)]
    pub fn upgrade(&self) -> Option<Acquirable<T>> {
        let inner = self.inner.upgrade()?;
        Some(Acquirable::new_raw(
            unsafe { inner.extract_ptr::<T>().unwrap_unchecked() },
            inner,
        ))
    }
}

impl<T: Extractable> Clone for WeakAcquirable<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

unsafe impl<T: Extractable + Send + Sync> Send for WeakAcquirable<T> {}
unsafe impl<T: Extractable + Send + Sync> Sync for WeakAcquirable<T> {}
