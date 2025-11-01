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
- ✅ Comprehensive test suite (60 tests covering integration, concurrency, memory safety, edge cases)
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
    for (id, player) in world.query_iter::<Player>() {
        println!("[{}] {}: {} HP", id.id(), player.entity.name, player.health);
    }

    // Extract nested components
    for (id, entity) in world.query_iter::<Entity>() {
        println!("Entity: {}", entity.name);
        
        // Try to extract as Player
        if let Some(player) = entity.extract::<Player>() {
            println!("  -> Player with {} HP", player.health);
        }
    }

    // Parallel queries for large datasets
    use rayon::prelude::*;
    world.par_query_iter::<Player>()
        .for_each(|(id, player)| {
            // Process players in parallel
        });

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
- Use cases requiring absolute maximum performance (though structecs is quite fast!)

---

## Core Concepts

### 1. Entity

An `Entity` is just an ID - a lightweight handle to your data.

```rust
pub struct EntityId {
    id: u32,
}
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
for (id, player) in world.query_iter::<Player>() {
    println!("[{}] {}: {} HP", id.id(), player.entity.name, player.health);
}

// Remove entities
world.remove_entity(&player_id);
```

**Core operations:**

- `add_entity<E: Extractable>(entity: E) -> EntityId` - Register new entity
- `remove_entity(entity_id: &EntityId) -> bool` - Remove entity from world
- `query_iter<T: 'static>() -> impl Iterator<Item = (EntityId, Acquirable<T>)>` - Efficiently iterate entities with component T
- `par_query_iter<T: 'static>() -> impl ParallelIterator` - Parallel iteration for large datasets
- `extract_component<T>(entity_id: &EntityId) -> Option<Acquirable<T>>` - Get specific component

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

---

## Performance

structecs is designed for high performance with real-world workloads:

### Benchmark Results (Release mode)

**Basic Operations:**
- Adding 10,000 entities: ~16ms
- Querying 10,000 entities (iterator): ~4ms
- Querying specific type (10,000): ~3.4ms

**Parallel Queries:**
- Sequential query with computation: 37ms
- Parallel query with computation: 27ms (1.35x speedup)

**Key Optimizations:**

1. **Archetype Storage**: Entities with the same type are stored contiguously
2. **Iterator-based Queries**: Zero allocation, lazy evaluation
3. **Extractor Caching**: Each type gets one shared extractor
4. **Compile-time Offsets**: Component access via direct pointer arithmetic

**When to Use Parallel Queries:**

Use `par_query_iter()` instead of `query_iter()` when:
- Processing >10,000 entities
- Performing non-trivial work per entity
- CPU-bound operations benefit from parallelism

For simple operations (e.g., summing a field), sequential queries may be faster due to parallelization overhead.

---

## Testing

structecs has a comprehensive test suite with **60+ tests** covering:

### Integration Tests (19 tests)
- World creation and management
- Entity addition and removal
- Query operations (sequential and parallel)
- Component extraction
- Multiple entity types
- Large entity sets (10,000+ entities)

### Concurrent Tests (10 tests)
- 10-100 thread concurrent insertions
- Concurrent reads while writing
- Data race detection
- Mixed operations (insert/remove/read/extract)
- Heavy load stress tests (5,000+ entities across 100 threads)

### Memory Safety Tests (10 tests)
- Memory leak detection
- Drop behavior verification
- Large entity lifecycle management
- Parallel query memory safety
- Massive insertion/removal (50,000 entities)

### Edge Case Tests (21 tests)
- Empty world operations
- Single entity operations
- Empty and Unicode strings
- Maximum values
- Rapid add/remove cycles
- Multiple entity types in same world
- Archetype tracking

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

**Test results:**
- ✅ 60 tests passed
- ✅ 0 failures
- ✅ 0 warnings
- ✅ All tests complete in ~12 seconds

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
| **Parallelism** | Built-in parallel systems | Parallel queries (Rayon) ✓ |
| **Flexibility** | Constrained by System API | Maximum flexibility ✓ |

---

## Development Roadmap

### Phase 1: Performance ✅ (Completed)

- [x] Archetype-based storage for better cache locality
- [x] Iterator-based queries (eliminate Vec allocation)
- [x] Extractor caching for zero-cost component access
- [x] Modular codebase structure

### Phase 2: Multi-threading ✅ (Completed)

- [x] Parallel query execution with Rayon
- [x] Thread-safe World operations with DashMap and RwLock
- [x] Fine-grained locking per archetype
- [x] Comprehensive concurrency tests

### Phase 3: Quality & Testing ✅ (Completed)

- [x] Entity removal
- [x] 60+ comprehensive tests covering:
  - Integration tests (19 tests)
  - Concurrent operations (10 tests)
  - Memory safety (10 tests)
  - Edge cases (21 tests)

### Phase 4: Features (In Progress)

- [ ] Dynamic component add/remove
- [ ] Event system
- [ ] Query filtering and composition
- [ ] Advanced query builders

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
