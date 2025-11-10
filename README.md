# structecs

[![Crates.io](https://img.shields.io/crates/v/structecs.svg)](https://crates.io/crates/structecs)
[![Documentation](https://docs.rs/structecs/badge.svg)](https://docs.rs/structecs)
[![License](https://img.shields.io/crates/l/structecs.svg)](https://github.com/moriyoshi-kasuga/structecs/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

**A structural data access framework for Rust.**

Access your data structures with OOP-like ergonomics, manage them however you want.

---

## ⚠️ Development Status

This crate is currently under active development and undergoing major refactoring.

**Version 0.3.x is a complete breaking change from earlier versions:**

- Removed: `World`, `Archetype`, `EntityID` (centralized ECS management)
- Focus: Structural type access with user-managed collections

The API is not stable and may change significantly. Use at your own risk. Feedback and contributions are welcome!

---

## What is structecs?

`structecs` is a structural data access framework that lets you work with nested data structures using type-safe extraction and smart pointers.

Instead of managing entities in a central ECS World, structecs gives you the tools to build your own data management patterns:

- **`Acquirable<T>`** - Arc-based smart pointers for shared ownership
- **`Extractable`** - Derive macro for structural type extraction from nested types
- **Type-safe extraction** - Access nested components through their parent structures

You manage your data structures (HashMap, BTree, custom collections) however you want. structecs just makes accessing nested types ergonomic and type-safe.

## Motivation

This framework was created for building a Minecraft server in Rust, where:

- Entity relationships are complex (Player ⊂ LivingEntity ⊂ Entity)
- Game logic is too varied to fit into rigid Systems
- OOP patterns are familiar but Rust's ownership makes traditional OOP difficult

**Why no World/Archetype/EntityID?**

Early versions included ECS-style centralized management, but this was removed because:

1. **No global entity tracking needed** - In a Minecraft server, entities are managed per-Chunk or per-World
2. **Arc wrapping makes centralized management redundant** - Since `Acquirable` uses `Arc` internally, you can store and share references however you want
3. **User-defined organization is better** - Different use cases need different organization:
   - `Arc<RwLock<HashMap<UUID, Acquirable<Entity>>>>` for entities
   - `Arc<RwLock<HashMap<BlockPos, Acquirable<Block>>>>` for blocks
   - Chunk-local collections, World-level collections, etc.

structecs provides the primitives (`Acquirable`, `Extractable`). You build the architecture.

## Core Concepts

### `Acquirable<T>` - Smart Pointers with Shared Ownership

`Acquirable<T>` is an Arc-based smart pointer that allows shared ownership of data with transparent access through `Deref`.

```rust
use structecs::*;

#[derive(Extractable)]
struct Player {
    name: String,
    health: u32,
}

let player = Acquirable::new(Player {
    name: "Steve".to_string(),
    health: 100,
});

// Access through Deref
println!("Player: {}, Health: {}", player.name, player.health);

// Clone creates a new reference to the same data
let player_ref = player.clone();
assert!(player.ptr_eq(&player_ref));
```

### `WeakAcquirable<T>` - Weak References

Prevent circular references and implement cache-like structures with weak references:

```rust
use structecs::*;

#[derive(Extractable)]
struct Player {
    name: String,
    health: u32,
}

let player = Acquirable::new(Player { name: "Alex".to_string(), health: 80 });
let weak = player.downgrade();

// Upgrade when needed
if let Some(player_ref) = weak.upgrade() {
    println!("Player still alive: {}", player_ref.name);
}
```

### `Extractable` - Structural Type Extraction

The `Extractable` derive macro enables type-safe extraction of nested components:

```rust
use structecs::*;

#[derive(Extractable)]
struct Health {
    current: u32,
    max: u32,
}

#[derive(Extractable)]
#[extractable(health)]  // Mark nested Extractable fields
struct LivingEntity {
    id: u32,
    health: Health,
}

#[derive(Extractable)]
#[extractable(living)]
struct Player {
    name: String,
    living: LivingEntity,
}

let player = Acquirable::new(Player {
    name: "Steve".to_string(),
    living: LivingEntity {
        id: 42,
        health: Health { current: 80, max: 100 },
    },
});

// Extract nested types
let health: Acquirable<Health> = player.extract::<Health>().unwrap();
assert_eq!(health.current, 80);

let living: Acquirable<LivingEntity> = player.extract::<LivingEntity>().unwrap();
assert_eq!(living.id, 42);
```

## Design Philosophy

- **No centralized storage** - You manage your own collections and data structures
- **OOP-like structural access** - Access nested types through parent structures naturally
- **User-controlled concurrency** - Wrap your collections in `Arc<RwLock<...>>` as needed
- **Type-safe extraction** - The derive macro ensures compile-time safety for nested type access
- **Minimal runtime overhead** - Offset-based extraction with zero-cost abstractions

## Usage Examples

### Basic Example

```rust
use structecs::*;

#[derive(Extractable)]
struct Entity {
    id: u32,
}

#[derive(Extractable)]
#[extractable(entity)]
struct NamedEntity {
    name: String,
    entity: Entity,
}

#[derive(Extractable)]
#[extractable(entity)]
struct BlockEntity {
    block_type: String,
    entity: Entity,
}

let named = Acquirable::new(NamedEntity {
    name: "Steve".to_string(),
    entity: Entity { id: 42 },
});

let block = Acquirable::new(BlockEntity {
    block_type: "Stone".to_string(),
    entity: Entity { id: 43 },
});

let entities: Vec<Acquirable<Entity>> = vec![named.extract().unwrap(), block.extract().unwrap()];

for entity in entities {
    println!("Entity ID: {}", entity.id);
}

```

The key insight: **You decide how to organize your data**. Per-chunk HashMap? Per-world BTreeMap? Custom spatial index? It's all up to you.

---

## Resources

- **[API Documentation](https://docs.rs/structecs)** - Full API reference
- **[Examples](examples/)** - Practical usage patterns
- **[Crates.io](https://crates.io/crates/structecs)** - Published versions

---

## License

Licensed under MIT License.

---

## Contributing

This project is in early development. Feedback, ideas, and contributions are welcome!

If you have suggestions or find issues, please open an issue or pull request on GitHub.
