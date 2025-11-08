# structecs

[![Crates.io](https://img.shields.io/crates/v/structecs.svg)](https://crates.io/crates/structecs)
[![Documentation](https://docs.rs/structecs/badge.svg)](https://docs.rs/structecs)
[![License](https://img.shields.io/crates/l/structecs.svg)](https://github.com/moriyoshi-kasuga/structecs/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

**A flexible entity-component framework without the System.**

Manage your data like ECS, control your logic like OOP.

---

## ⚠️ Development Status

This crate is currently under active development. The API is not stable and may change significantly.
Currently, performance is quite poor compared to established ECS frameworks.
Use at your own risk. Feedback and contributions are welcome!

---

## What is structecs?

`structecs` is an ECS-inspired data management framework designed for complex applications like game servers where traditional ECS systems can be limiting.

Unlike conventional ECS frameworks (Bevy, specs, hecs), structecs:

- ✅ **No rigid System architecture** - Write your logic however you want
- ✅ **Hierarchical components** - Nest components naturally like OOP
- ✅ **Dynamic type extraction** - Query for any component type on-the-fly
- ✅ **Zero-cost abstractions** - Uses compile-time offsets for component access
- ✅ **Thread-safe** - Concurrent operations via lock-free maps + short locks

---

## Quick Start

```rust
use structecs::*;

// Define your entities with the Extractable derive macro
#[derive(Debug, Extractable)]
pub struct Entity {
    pub name: String,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]  // Mark nested extractable fields
pub struct Player {
    pub entity: Entity,
    pub health: u32,
}

fn main() {
    // Create the world
    let world = World::default();

    // Add entities
    let player_id = world.add_entity(Player {
        entity: Entity {
            name: "Hero".to_string(),
        },
        health: 100,
    });

    // Query all players
    for (id, player) in world.query::<Player>() {
        println!("[{}] {}: {} HP", id, player.entity.name, player.health);
    }

    // Extract nested components
    for (id, entity) in world.query::<Entity>() {
        println!("Entity: {}", entity.name);
        
        // Try to extract as Player
        if let Some(player) = entity.extract::<Player>() {
            println!("  -> Player with {} HP", player.health);
        }
    }

    // Remove entities
    world.remove_entity(&player_id);
}
```

### When to use structecs?

**Good for:**

- Complex game servers (Minecraft, MMOs) with intricate entity relationships
- Applications where game logic doesn't fit cleanly into Systems
- Projects transitioning from OOP to data-oriented design
- Scenarios requiring flexible, ad-hoc data access patterns

**Not ideal for:**

- Simple games where traditional ECS works well
- Projects heavily invested in existing ECS ecosystems
- Use cases requiring absolute maximum performance

---

## Core Concepts

### Hierarchical Components

The key innovation of structecs is **hierarchical components with flat access**:

```rust
#[derive(Extractable)]
pub struct Entity {
    pub name: String,
}

#[derive(Extractable)]
#[extractable(entity)]  // Mark Entity as extractable
pub struct LivingEntity {
    pub entity: Entity,
    pub health: u32,
}

#[derive(Extractable)]
#[extractable(living)]  // Mark LivingEntity as extractable
pub struct Player {
    pub living: LivingEntity,
    pub inventory: Inventory,
}

// Query any level of the hierarchy
for (id, entity) in world.query::<Entity>() { /* ... */ }
for (id, living) in world.query::<LivingEntity>() { /* ... */ }
for (id, player) in world.query::<Player>() { /* ... */ }
```

See `examples/` for more detailed usage patterns.

### Mutability Control

structecs provides **read-only access** by default. Users control mutability explicitly using interior mutability:

```rust
use std::sync::{Mutex, RwLock};
use std::sync::atomic::AtomicU32;

#[derive(Extractable)]
pub struct Player {
    pub name: String,
    pub health: AtomicU32,  // Lock-free concurrent updates
}

#[derive(Extractable)]
pub struct Inventory {
    pub items: Mutex<Vec<Item>>,  // Fine-grained locking
}

// Usage
for (id, player) in world.query::<Player>() {
    player.health.fetch_add(10, Ordering::Relaxed);
}
```

**Why no `query_mut()`?**

- Prevents lock contention on the entire World
- Allows fine-grained control over locking strategy
- Essential for high-concurrency scenarios

See `examples/mutability.rs` for detailed patterns.

---

## API Overview

### World

The central data store that manages all entities.

```rust
let world = World::default();

// Add entities
let id = world.add_entity(Player { /* ... */ });
let (id, player) = world.add_entity_with_acquirable(Player { /* ... */ });
let ids = world.add_entities(vec![player1, player2, player3]);

// Query entities
for (id, player) in world.query::<Player>() { /* ... */ }

// Extract components
let health = world.extract_component::<Health>(&entity_id)?;

// Remove entities
world.remove_entity(&entity_id)?;                    // Single removal
world.remove_entities(&[id1, id2, id3]);             // Batch removal (silent)
world.try_remove_entities(&[id1, id2, id3])?;        // Batch removal (error tracking)

// Utilities
world.contains_entity(&entity_id);
world.entity_count();
world.archetype_count();
world.clear();
```

> **Full API documentation**: Run `cargo doc --open` to view detailed API docs.

### Error Handling

```rust
pub enum WorldError {
    EntityNotFound(EntityId),
    ComponentNotFound { entity_id: EntityId, component_name: &'static str },
    PartialRemoval { succeeded: Vec<EntityId>, failed: Vec<EntityId> },
    ArchetypeNotFound(EntityId),
}
```

Example:

```rust
match world.try_remove_entities(&entity_ids) {
    Ok(()) => println!("All removed"),
    Err(WorldError::PartialRemoval { succeeded, failed }) => {
        println!("Removed: {:?}, Failed: {:?}", succeeded, failed);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Examples

The `examples/` directory contains practical usage patterns:

- **`examples/normal.rs`** - Basic usage and entity management
- **`examples/concurrent.rs`** - Multi-threaded operations
- **`examples/handler.rs`** - Polymorphic behavior with ComponentHandler
- **`examples/hierarchical.rs`** - Hierarchical component queries *(coming soon)*
- **`examples/mutability.rs`** - Interior mutability patterns *(coming soon)*
- **`examples/batch_operations.rs`** - Efficient batch operations *(coming soon)*

Run examples:

```bash
cargo run --example normal
cargo run --example concurrent
cargo run --example handler
```

---

## Architecture

For detailed architecture documentation, see [Architecture.md](Architecture.md).

### Design Philosophy

**"Data is hierarchical, access is flat"**

- Store entities as natural Rust structs (hierarchical)
- Query any component type regardless of nesting (flat access)
- No forced System architecture (user controls logic flow)

This gives you:

- **Expressiveness** of OOP (nested data, clear relationships)
- **Flexibility** of procedural code (write systems however you want)
- **Performance** of data-oriented design (archetype-based storage)

### Memory Layout

Entities with the same structure are grouped into archetypes for better cache locality:

```
World:
  Archetype<Player>:
    [Entity 0] Player { entity: Entity { ... }, health: 100 }
    [Entity 1] Player { entity: Entity { ... }, health: 80 }
  
  Archetype<Monster>:
    [Entity 3] Monster { entity: Entity { ... }, damage: 20 }
```

Component extraction uses pre-computed offsets for zero-cost access.

### Concurrency Model

structecs uses a hierarchical lock-free design:

1. **World**: All operations use `&self` (no locks)
2. **DashMap**: Lock-free reads, fine-grained write locks
3. **Archetype**: Concurrent access via DashMap
4. **Components**: User-controlled (Atomic, Mutex, RwLock)

This enables high concurrency with minimal lock contention.

---

## Performance

Benchmark results (10,000 entities, Release mode):

| Operation | bevy_ecs | hecs | specs | **structecs** |
|-----------|----------|------|-------|---------------|
| Add entities | 707µs | **578µs** | 890µs | 958µs (1.66x) |
| Query all | **5.5µs** | 19µs | 16µs | 74µs (13.5x) |
| Query 2 components | **4.1µs** | 4.9µs | 14µs | 76µs (18.3x) |

**Trade-offs:**

- ✅ Competitive entity addition performance
- ✅ Unique hierarchical component feature
- ⚠️ Slower queries due to dynamic type extraction and flexibility

**Use structecs when:**

- Hierarchical entity structures are essential
- Query performance is not the primary bottleneck
- Flexibility and expressiveness matter more than raw speed

**Use bevy_ecs/hecs when:**

- Maximum query performance is critical
- Traditional flat ECS patterns suffice
- Processing millions of entities per frame

---

## Testing

structecs has a comprehensive test suite covering:

- Integration tests
- Concurrent operations
- Memory safety
- Edge cases
- Drop order verification
- Reference counting

Run tests:

```bash
# Run all tests
cargo test --all

# Run specific test suite
cargo test --test integration_test
cargo test --test concurrent_test
cargo test --test memory_safety_test
```

---

## Comparison with Traditional ECS

| Aspect | Traditional ECS | structecs |
|--------|----------------|-----------|
| **Entity** | Opaque ID | Opaque ID ✓ |
| **Component** | Standalone data types | Fields in structs |
| **System** | First-class with scheduling | User implements freely |
| **Data Layout** | Archetype/sparse sets | Archetype-based ✓ |
| **Query** | Compile-time parameters | Runtime extraction |
| **Nesting** | Components are flat | Components can nest ✓ |
| **Flexibility** | Constrained by System API | Maximum flexibility ✓ |

---

## Motivation

This framework was created for building a Minecraft server in Rust, where:

- Entity relationships are complex (Player ⊂ LivingEntity ⊂ Entity)
- Game logic is too varied to fit into rigid Systems
- OOP patterns are familiar but Rust's ownership makes traditional OOP difficult

structecs bridges the gap: data-oriented storage with OOP-like access patterns.

---

## Resources

- **[Architecture Documentation](Architecture.md)** - Design philosophy and implementation details
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
