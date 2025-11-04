use crate::{Acquirable, Extractable, ExtractionMetadata, entity::EntityData};

/// Metadata for debugging handler type information (debug builds only).
#[cfg(debug_assertions)]
struct HandlerMetadata {
    base_type: &'static str,
    concrete_type: &'static str,
    signature: String,
}

/// Type-erased function wrapper that stores a handler function.
struct TypeErasedFn<Args, Return> {
    caller: Box<dyn Fn(EntityData, Args) -> Return + Send + Sync>,
    #[cfg(debug_assertions)]
    metadata: HandlerMetadata,
}

impl<Args, Return> TypeErasedFn<Args, Return> {
    pub fn new<Base, Concrete>(
        func: impl Fn(&Acquirable<Concrete>, Args) -> Return + Send + Sync + 'static,
    ) -> Self
    where
        Base: Extractable,
        Concrete: Extractable,
    {
        let caller = move |data: EntityData, args: Args| -> Return {
            // SAFETY: Type relationship is validated in debug builds during ComponentHandler creation
            #[allow(clippy::expect_used)]
            let entity = data
                .extract::<Concrete>()
                .expect("Handler type mismatch - this is a bug in ComponentHandler");
            func(&entity, args)
        };

        Self {
            caller: Box::new(caller),
            #[cfg(debug_assertions)]
            metadata: HandlerMetadata {
                base_type: std::any::type_name::<Base>(),
                concrete_type: std::any::type_name::<Concrete>(),
                signature: format!(
                    "Fn(&Acquirable<{}>, {}) -> {}",
                    std::any::type_name::<Concrete>(),
                    std::any::type_name::<Args>(),
                    std::any::type_name::<Return>()
                ),
            },
        }
    }

    pub fn call<E: Extractable>(&self, entity: &Acquirable<E>, args: Args) -> Return {
        (self.caller)(entity.inner.clone(), args)
    }
}

// SAFETY: TypeErasedFn only contains a Box<dyn Fn> which is Send,
// and the function itself is bounded by Send + Sync traits.
unsafe impl<Args, Return> Send for TypeErasedFn<Args, Return> {}
// SAFETY: TypeErasedFn only contains a Box<dyn Fn> which is Sync,
// and the function itself is bounded by Send + Sync traits.
unsafe impl<Args, Return> Sync for TypeErasedFn<Args, Return> {}

/// A component handler that enables polymorphic behavior on entity hierarchies.
///
/// This handler allows you to define behavior for concrete entity types (like `Player` or `Zombie`)
/// while storing the handler in a base type (like `Entity`). When querying for the base type,
/// the handler will execute the concrete type's implementation.
///
/// # Type Parameters
///
/// - `Base`: The base struct type used for queries (e.g., `Entity`)
/// - `Args`: The argument tuple type for the handler function (default: `()`)
/// - `Return`: The return type of the handler function (default: `()`)
///
/// # Example
///
/// ```ignore
/// use structecs::*;
///
/// #[derive(Debug, Extractable)]
/// pub struct Entity {
///     pub name: String,
///     pub death_handler: ComponentHandler<Entity>,
/// }
///
/// #[derive(Debug, Extractable)]
/// #[extractable(entity)]
/// pub struct Player {
///     pub entity: Entity,
///     pub level: u32,
/// }
///
/// // Create a handler for Player entities
/// let player_handler = ComponentHandler::<Entity>::for_type::<Player>(|player, ()| {
///     println!("Level {} player {} died!", player.level, player.entity.name);
/// });
///
/// // Query for Entity, but Player's handler will be called
/// for (id, entity) in world.query::<Entity>() {
///     entity.death_handler.call(&entity, ());  // Calls Player-specific logic
/// }
/// ```
pub struct ComponentHandler<Base: Extractable, Args = (), Return = ()> {
    function: TypeErasedFn<Args, Return>,
    _marker: std::marker::PhantomData<Base>,
}

impl<Base: Extractable, Args, Return> ComponentHandler<Base, Args, Return> {
    /// Create a handler for entities of type `Concrete` that can be extracted as `Base`.
    ///
    /// # Type Parameters
    ///
    /// - `Concrete`: The actual entity type (e.g., `Player`, `Zombie`)
    ///
    /// The `Concrete` type must contain the `Base` type in its extraction metadata.
    /// This is typically achieved using `#[extractable(field_name)]` on the concrete type.
    ///
    /// # Panics
    ///
    /// In debug builds, this function will panic if `Concrete` does not contain `Base`
    /// in its extraction metadata. This helps catch type mismatches early during development.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Handler for Player entities stored in Entity base type
    /// let handler = ComponentHandler::<Entity>::for_type::<Player>(|player, ()| {
    ///     println!("Player {} died", player.entity.name);
    /// });
    /// ```
    pub fn for_type<Concrete: Extractable>(
        func: impl Fn(&Acquirable<Concrete>, Args) -> Return + Send + Sync + 'static,
    ) -> Self {
        #[cfg(debug_assertions)]
        Self::validate_type_relationship::<Concrete>();

        Self {
            function: TypeErasedFn::new::<Base, Concrete>(func),
            _marker: std::marker::PhantomData,
        }
    }

    /// Validate that Concrete can be extracted as Base (debug builds only).
    #[cfg(debug_assertions)]
    fn validate_type_relationship<Concrete: Extractable>() {
        if !can_extract::<Concrete, Base>() {
            panic!(
                "\n╔════════════════════════════════════════════════════════════╗\n\
                 ║ ComponentHandler Type Mismatch                             ║\n\
                 ╠════════════════════════════════════════════════════════════╣\n\
                 ║ Base type:     {:<44}║\n\
                 ║ Concrete type: {:<44}║\n\
                 ╠════════════════════════════════════════════════════════════╣\n\
                 ║ The concrete type must contain the base type in its        ║\n\
                 ║ extraction metadata. Did you forget #[extractable(...)]?   ║\n\
                 ║                                                            ║\n\
                 ║ Example:                                                   ║\n\
                 ║   #[derive(Extractable)]                                   ║\n\
                 ║   #[extractable(entity)]  // <-- Add this!                 ║\n\
                 ║   pub struct Player {{                                      ║\n\
                 ║       pub entity: Entity,                                  ║\n\
                 ║   }}                                                        ║\n\
                 ╚════════════════════════════════════════════════════════════╝\n",
                std::any::type_name::<Base>(),
                std::any::type_name::<Concrete>()
            );
        }
    }

    /// Call the handler with an entity.
    ///
    /// # Type Parameters
    ///
    /// - `E`: The entity type being passed to the handler
    ///
    /// The entity type `E` must be extractable as `Base`. The handler will then
    /// extract the concrete type it was created with and call the appropriate function.
    ///
    /// # Panics
    ///
    /// In debug builds, this function will panic if `E` cannot be extracted as `Base`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let handler = ComponentHandler::<Entity>::for_type::<Player>(|player, ()| {
    ///     println!("Player died");
    /// });
    ///
    /// for (id, entity) in world.query::<Entity>() {
    ///     handler.call(&entity, ());
    /// }
    /// ```
    pub fn call<E: Extractable>(&self, entity: &Acquirable<E>, args: Args) -> Return {
        #[cfg(debug_assertions)]
        self.validate_call::<E>();

        self.function.call(entity, args)
    }

    /// Validate that the entity type can be extracted as Base (debug builds only).
    #[cfg(debug_assertions)]
    fn validate_call<E: Extractable>(&self) {
        if !can_extract::<E, Base>() {
            panic!(
                "\n╔════════════════════════════════════════════════════════════╗\n\
                 ║ ComponentHandler Call Mismatch                             ║\n\
                 ╠════════════════════════════════════════════════════════════╣\n\
                 ║ Expected base:  {:<44}║\n\
                 ║ Actual type:    {:<44}║\n\
                 ║ Handler for:    {:<44}║\n\
                 ╠════════════════════════════════════════════════════════════╣\n\
                 ║ The entity type must be extractable as the base type.      ║\n\
                 ╚════════════════════════════════════════════════════════════╝\n",
                std::any::type_name::<Base>(),
                std::any::type_name::<E>(),
                self.function.metadata.concrete_type
            );
        }
    }

    /// Get debug information about this handler (debug builds only).
    ///
    /// Returns a string containing the base type, concrete type, and function signature.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let handler = ComponentHandler::<Entity>::for_type::<Player>(|player, ()| {
    ///     println!("Player died");
    /// });
    ///
    /// #[cfg(debug_assertions)]
    /// println!("{}", handler.debug_info());
    /// // Output: "ComponentHandler<Entity> for Player (signature: Fn(&Acquirable<Player>, ()) -> ())"
    /// ```
    #[cfg(debug_assertions)]
    pub fn debug_info(&self) -> String {
        format!(
            "ComponentHandler<{}> for {} (signature: {})",
            self.function.metadata.base_type,
            self.function.metadata.concrete_type,
            self.function.metadata.signature
        )
    }
}

impl<Base: Extractable, Args, Return> std::fmt::Debug for ComponentHandler<Base, Args, Return> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ComponentHandler");
        #[cfg(debug_assertions)]
        {
            debug.field("base_type", &self.function.metadata.base_type);
            debug.field("concrete_type", &self.function.metadata.concrete_type);
            debug.field("signature", &self.function.metadata.signature);
        }
        #[cfg(not(debug_assertions))]
        debug.field("function", &"<type erased>");
        debug.finish()
    }
}

/// Helper function to search for a target type in extraction metadata.
#[cfg(debug_assertions)]
fn search_metadata(list: &[ExtractionMetadata], target: std::any::TypeId) -> bool {
    for metadata in list {
        match metadata {
            ExtractionMetadata::Target { type_id, .. } => {
                if *type_id == target {
                    return true;
                }
            }
            ExtractionMetadata::Nested {
                type_id, nested, ..
            } => {
                if *type_id == target || search_metadata(nested, target) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if `Concrete` can be extracted as `Base`.
///
/// Returns `true` if the `Base` type exists in `Concrete`'s extraction metadata.
///
/// Note: [`std::any::TypeId`] is not const evaluable yet, so this can't be a const fn.
#[cfg(debug_assertions)]
fn can_extract<Concrete: Extractable, Base: Extractable>() -> bool {
    let base_type_id = std::any::TypeId::of::<Base>();
    search_metadata(Concrete::METADATA_LIST, base_type_id)
}
