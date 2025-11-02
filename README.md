# structecs

**A flexible entity-component framework without the System.**

Manage your data like ECS, control your logic like OOP.

---

## ⚠️ Development Status

This crate is currently under active development. The API is not stable and may change significantly.

**Recent Updates:**

- ✅ Archetype-based storage implemented
- ✅ Iterator-based queries (zero-allocation)
- ✅ Parallel query support with Rayon
- ✅ Additional components system (optional runtime data)
- ✅ Comprehensive test suite (69 tests covering integration, concurrency, memory safety, edge cases)
- ✅ Thread-safe operations with fine-grained locking
- 🔄 Query composition and filtering (planned)

---

## What is structecs?

`structecs` is an ECS-inspired data management framework designed for complex applications like game servers where traditional ECS systems can be limiting.

Unlike conventional ECS frameworks (Bevy, specs, hecs), structecs:

- ✅ **No rigid System architecture** - Write your logic however you want
- ✅ **Hierarchical components** - Nest components naturally like OOP
- ✅ **Dynamic type extraction** - Query for any component type on-the-fly
- ✅ **Zero-cost abstractions** - Uses compile-time offsets for component access
- ✅ **Thread-safe** - Concurrent operations with fine-grained locking

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
        println!("[{}] {}: {} HP", id.id(), player.entity.name, player.health);
    }

    // Extract nested components
    for (id, entity) in world.query::<Entity>() {
        println!("Entity: {}", entity.name);
        
        // Try to extract as Player
        if let Some(player) = entity.extract::<Player>() {
            println!("  -> Player with {} HP", player.health);
        }
    }

    // Parallel queries for large datasets
    use rayon::prelude::*;
    world.par_query::<Player>()
        .for_each(|(id, player)| {
            // Process players in parallel
        });

    // Batch remove entities efficiently
    let ids_to_remove = vec![player_id /* ... */];
    let removed_count = world.remove_entities(&ids_to_remove);
    println!("Removed {} entities", removed_count);

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
    pub fn id(&self) -> u32;
    pub fn as_usize(&self) -> usize;  // For array indexing
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
    println!("[{}] {}: {} HP", id.id(), player.entity.name, player.health);
}

// Batch operations
let ids = vec![id1, id2, id3];
let count = world.remove_entities(&ids);  // Efficient batch removal

// Check existence
if world.contains_entity(&player_id) {
    world.remove_entity(&player_id);
}
```

**Core operations:**

- `add_entity<E: Extractable>(entity: E) -> EntityId` - Register new entity
- `remove_entity(entity_id: &EntityId) -> bool` - Remove single entity from world
- `remove_entities(entity_ids: &[EntityId]) -> usize` - Batch remove entities, returns count of removed
- `contains_entity(entity_id: &EntityId) -> bool` - Check if entity exists in world
- `clear()` - Remove all entities from world
- `query<T: 'static>() -> Vec(EntityId, Acquirable<T>)>` - Efficiently iterate entities with component T
- `extract_component<T>(entity_id: &EntityId) -> Option<Acquirable<T>>` - Get specific component

**Additional component operations:**

- `add_additional<E: Extractable>(entity_id, additional: E) -> bool` - Add or replace additional component
- `extract_additional<T>(entity_id) -> Option<Acquirable<T>>` - Get additional component
- `has_additional<T>(entity_id) -> bool` - Check if entity has additional
- `remove_additional<T>(entity_id) -> Option<Acquirable<T>>` - Remove and return additional
- `query_with<T, A>() -> QueryWith<T, A>` - Query with optional additional components

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

### 6. Additional Components

Additional components are optional, dynamic data that can be attached to entities at runtime without modifying the base entity structure. Think of them as "tags" or "markers" that can be added and removed freely.

**Use cases:**

- **Temporary states**: Buffs, debuffs, status effects
- **Optional metadata**: Tags, flags, markers
- **Runtime data**: Quest progress, achievements, dynamic attributes

**Key operations:**

```rust
// Add or replace an additional component
world.add_additional(&entity_id, Buff { power: 50, duration: 30 });

// Check if entity has an additional
if world.has_additional::<Buff>(&entity_id) {
    // ...
}

// Extract an additional component
if let Some(buff) = world.extract_additional::<Buff>(&entity_id) {
    println!("Buff power: {}", buff.power);
}

// Remove an additional component
world.remove_additional::<Buff>(&entity_id);
```

**Query with additionals:**

The `query_with` API allows you to query base entities and optionally extract additional components in a single iteration:

```rust
// Query players with optional buffs and poison status
for (id, player, (buff, poison)) in world
    .query_with::<Player, (Buff, Poisoned)>()
    .iter()
{
    // buff: Option<Acquirable<Buff>>
    // poison: Option<Acquirable<Poisoned>>
    
    if let Some(b) = buff {
        println!("Player {} has buff: {}", player.name, b.name);
    }
    
    if let Some(p) = poison {
        println!("Player {} is poisoned!", player.name);
    }
}

// Supports up to 6 additional types in a tuple
world.query_with::<Player, (Buff, Poisoned, QuestProgress, Tag, Marker, Flag)>()
```

**Storage:**

- Additionals are stored separately from the base entity structure
- Each entity has a `Vec<(TypeId, Data, Extractor)>` for its additionals
- Linear search is used (efficient for 2-3 additionals per entity)
- Thread-safe with `RwLock` for concurrent access

See `examples/additional.rs` for a complete example.

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
- Querying 10,000 entities (iterator): ~4ms
- Querying specific type (10,000): ~3.4ms

**Key Optimizations:**

1. **Archetype Storage**: Entities with the same type are stored contiguously
2. **Iterator-based Queries**: Zero allocation, lazy evaluation
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
- [x] Iterator-based queries (eliminate Vec allocation)
- [x] Extractor caching for zero-cost component access
- [x] Type index for fast archetype lookup
- [x] Modular codebase structure

### Phase 2: Multi-threading ✅ (Completed)

- [x] Parallel query execution with Rayon
- [x] Thread-safe World operations with DashMap and RwLock
- [x] Fine-grained locking per archetype
- [x] Comprehensive concurrency tests

### Phase 3: Quality & Testing ✅ (Completed)

- [x] Entity removal
- [x] Additional components system

### Phase 4: Features (In Progress)

- [x] Additional components (optional runtime data)
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
