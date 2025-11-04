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

**Recent Updates:**

- ✅ Archetype-based storage implemented
- ✅ Snapshot-based queries with type index
- ✅ Thread-safe operations using DashMap
- ✅ Comprehensive test suite (integration, concurrency, memory safety, edge cases)
- 🔄 Query composition and filtering (planned)

---

## What is structecs?

`structecs` is an ECS-inspired data management framework designed for complex applications like game servers where traditional ECS systems can be limiting.

Unlike conventional ECS frameworks (Bevy, specs, hecs), structecs:

- ✅ **No rigid System architecture** - Write your logic however you want
- ✅ **Hierarchical components** - Nest components naturally like OOP
- ✅ **Dynamic type extraction** - Query for any component type on-the-fly
- ✅ **Zero-cost abstractions** - Uses compile-time offsets for component access
- ✅ **Thread-safe** - Concurrent operations via lock-free maps + short locks

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


    // Batch remove entities efficiently (silent, no error tracking)
    let ids_to_remove = vec![player_id /* ... */];
    world.remove_entities(&ids_to_remove);

    // Check entity existence
    if world.contains_entity(&player_id) {
        println!("Player still exists");
    }

    // Clear all entities
    world.clear();
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
- Use cases requiring absolute maximum performance (though structecs is quite fast!)

---

## Core Concepts

### 1. Entity

An `Entity` is just an ID - a lightweight handle to your data.

```rust
pub struct EntityId {
    id: u32,
}

impl EntityId {
    pub fn from_raw(id: u32) -> Self;  // Create from raw u32
}

// EntityId implements Display: "Entity(123)"
println!("{}", entity_id);
```

Entities don't "own" components. Instead, they reference structured data stored in the `World`.

### 2. Component (via Extractable)

In structecs, components are **fields within structs**. The `Extractable` trait allows the framework to understand your data structure and extract specific types.

```rust
use structecs::*;

#[derive(Debug, Extractable)]
pub struct Entity {
    pub name: String,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Player {
    pub entity: Entity,
    pub health: u32,
}
```

**Key insight:** Components are hierarchical. A `Player` might contain an `Entity`, which itself is extractable.

### 3. World

The central data store that manages all entities and their data.

```rust
let world = World::default();

// Add entities
let player_id = world.add_entity(Player {
    entity: Entity {
        name: "Hero".to_string(),
    },
    health: 100,
});

// Query entities
for (id, player) in world.query::<Player>() {
    println!("[{}] {}: {} HP", id, player.entity.name, player.health);
}

// Batch operations
let ids = vec![id1, id2, id3];
world.remove_entities(&ids);  // Efficient batch removal (silent)

// Check existence
if world.contains_entity(&player_id) {
    world.remove_entity(&player_id);
}
```

**Core operations:**

- `add_entity<E: Extractable>(entity: E) -> EntityId` - Register new entity
- `remove_entity(entity_id: &EntityId) -> Result<(), WorldError>` - Remove single entity from world
- `try_remove_entities(entity_ids: &[EntityId]) -> Result<(), WorldError>` - Batch remove entities with error tracking
- `remove_entities(entity_ids: &[EntityId]) -> ()` - Fast batch removal (silently skips not found)
- `contains_entity(entity_id: &EntityId) -> bool` - Check if entity exists in world
- `clear()` - Remove all entities from world
- `query<T: 'static>() -> Vec<(EntityId, Acquirable<T>)>` - Snapshot of entities with component T
- `extract_component<T>(entity_id: &EntityId) -> Result<Acquirable<T>, WorldError>` - Get specific component

### 4. Acquirable

A smart reference to a component that keeps the underlying entity data alive.

```rust
pub struct Acquirable<T: 'static> {
    target: NonNull<T>,
    inner: EntityDataInner,
    // ...
}
```

**Features:**

- Implements `Deref<Target = T>` for transparent access
- Reference-counted to prevent use-after-free
- Can `extract()` other component types from the same entity

This enables OOP-like method chaining:

```rust
entity.extract::<Player>()?.extract::<Health>()?
```

### 5. Extractor

The engine that performs type extraction using pre-computed offsets.

```rust
pub struct Extractor {
    offsets: HashMap<TypeId, usize>,
    dropper: unsafe fn(NonNull<u8>),
}
```

Each unique entity structure gets one `Extractor` (cached in `World`), which knows:

- Where each component type lives in memory (offset)
- How to safely drop the entity when done

---

## Architecture

For detailed architecture documentation, see [Architecture.md](Architecture.md).

### Memory Layout

**Archetype-based Storage:**

Entities with the same structure (type) are grouped into archetypes for better cache locality:

```
World:
  Archetype<Player>:
    [Entity 0] Player { entity: Entity { name: "A" }, health: 100 }
    [Entity 1] Player { entity: Entity { name: "B" }, health: 80 }
    [Entity 2] Player { entity: Entity { name: "C" }, health: 90 }
  
  Archetype<Monster>:
    [Entity 3] Monster { entity: Entity { name: "X" }, damage: 20 }
    [Entity 4] Monster { entity: Entity { name: "Y" }, damage: 30 }
```

**Component Extraction:**

```
Player struct in memory:
┌─────────────────────────────┐
│ Entity { name: String }     │ ← offset 0: Entity
│  ├─ name: String            │ ← offset 0: String
├─────────────────────────────┤
│ health: u32                 │ ← offset X: u32
└─────────────────────────────┘

The Extractor knows:
- TypeId(Entity) -> offset 0
- TypeId(String) -> offset 0  (flattened from Entity)
- TypeId(u32) -> offset X
```

### Data Flow

1. **Entity Registration:**

   ```
   User creates struct → Derive macro generates METADATA_LIST
   → add_entity() creates Extractor → Data stored in World
   ```

2. **Component Query:**

   ```
   query<T>() → Iterate all entities → Check if Extractor has TypeId(T)
   → Calculate pointer via offset → Wrap in Acquirable
   ```

3. **Component Extraction:**

   ```
   Acquirable<A>.extract<B>() → Reuse same Extractor
   → Get offset for TypeId(B) → Return new Acquirable<B>
   ```

### Design Philosophy

**"Data is hierarchical, access is flat"**

- Store entities as natural Rust structs (hierarchical)
- Query any component type regardless of nesting (flat access)
- No forced System architecture (user controls logic flow)

This gives you:

- **Expressiveness** of OOP (nested data, clear relationships)
- **Performance** of data-oriented design (offset-based access, no virtual dispatch)
- **Flexibility** of procedural code (write systems however you want)

### Mutability Design

structecs provides **read-only access** by default. Users control mutability explicitly using Rust's interior mutability patterns:

```rust
use std::sync::{Mutex, RwLock};
use std::sync::atomic::AtomicU32;

// Pattern 1: Lock-free with Atomics
#[derive(Extractable)]
pub struct Player {
    pub name: String,
    pub health: AtomicU32,  // Lock-free concurrent updates
}

// Pattern 2: Fine-grained locking with Mutex
#[derive(Extractable)]
pub struct Inventory {
    pub items: Mutex<Vec<Item>>,  // Lock only when accessing items
}

// Pattern 3: Read/write separation with RwLock
#[derive(Extractable)]
pub struct Position {
    pub coords: RwLock<Vec3>,  // Multiple readers, single writer
}

// Usage
for (id, player) in world.query::<Player>() {
    player.health.fetch_add(10, Ordering::Relaxed);
}
```

**Why no `query_mut()`?**

- **Prevents lock contention**: Locking the entire World would block all operations
- **Fine-grained control**: Users choose the optimal locking strategy per component
- **Multi-threading first**: Essential for high-concurrency scenarios like game servers

This design philosophy prioritizes flexibility and performance in concurrent environments.

---

## Performance

structecs is designed for high performance with real-world workloads:

### Benchmark Results (Release mode)

**Basic Operations:**

- Adding 10,000 entities: ~16ms
- Querying 10,000 entities: ~4ms
- Querying specific type (10,000): ~3.4ms

**Key Optimizations:**

1. **Archetype Storage**: Entities with the same type are stored contiguously
2. **Type-indexed Snapshot Queries**: Avoid scanning unrelated archetypes; short-lived locks
3. **Extractor Caching**: Each type gets one shared extractor
4. **Compile-time Offsets**: Component access via direct pointer arithmetic

---

## Testing

structecs has a comprehensive test suite:

**Run tests:**

```bash
# Run all tests
cargo test --all

# Run specific test suite
cargo test --test integration_test
cargo test --test concurrent_test
cargo test --test memory_safety_test
cargo test --test edge_cases_test
```

---

## Comparison with Traditional ECS

| Aspect | Traditional ECS | structecs |
|--------|----------------|-----------|
| **Entity** | Opaque ID | Opaque ID ✓ |
| **Component** | Standalone data types | Fields in structs |
| **System** | First-class concept with scheduling | User implements freely |
| **Data Layout** | Archetype/sparse sets | Archetype-based ✓ |
| **Query Pattern** | Compile-time system parameters | Runtime extraction |
| **Nesting** | Components are flat | Components can nest ✓ |
| **Cache Coherency** | Excellent (packed arrays) | Good (archetype storage) |
| **Flexibility** | Constrained by System API | Maximum flexibility ✓ |

---

## Development Roadmap

### Phase 1: Performance ✅ (Completed)

- [x] Archetype-based storage for better cache locality
- [x] Snapshot-based queries (Vec snapshot per call)
- [x] Extractor caching for zero-cost component access
- [x] Type index for fast archetype lookup
- [x] Modular codebase structure

### Phase 2: Multi-threading (Partially Completed)

- [ ] Parallel query execution with Rayon (planned)
- [x] Thread-safe World operations with DashMap
- [x] Fine-grained locking per archetype (short-lived)
- [x] Comprehensive concurrency tests


### Phase 3: Quality & Testing ✅ (Completed)

- [x] Entity removal
- [x] Memory safety verification
- [x] Comprehensive test suite (118 tests)

### Phase 4: Features (In Progress)

- [ ] Query filtering and composition (planned)
- [ ] Error handling improvements (planned)

---

## Motivation

This framework was created for building a Minecraft server in Rust, where:

- Entity relationships are complex (Player ⊂ LivingEntity ⊂ Entity)
- Game logic is too varied to fit into rigid Systems
- OOP patterns are familiar but Rust's ownership makes traditional OOP difficult

structecs bridges the gap: data-oriented storage with OOP-like access patterns.

---

## License

Licensed under:

- MIT License

---

## Contributing

This project is in early development. Feedback, ideas, and contributions are welcome!
