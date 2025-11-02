//! # structecs
//!
//! A flexible entity-component framework without the System.
//!
//! ## Overview
//!
//! structecs provides an ECS-inspired data management system that focuses on flexibility
//! over rigid System architecture. It allows you to:
//!
//! - Store hierarchical component data naturally
//! - Query components efficiently using archetype-based storage
//! - Extract components dynamically at runtime
//! - Write game logic however you want (no forced System pattern)
//!
//! ## Example
//!
//! ```rust
//! use structecs::*;
//!
//! #[derive(Debug, Extractable)]
//! pub struct Entity {
//!     pub name: String,
//! }
//!
//! #[derive(Debug, Extractable)]
//! #[extractable(entity)]
//! pub struct Player {
//!     pub entity: Entity,
//!     pub health: u32,
//! }
//!
//! let mut world = World::default();
//!
//! let player = Player {
//!     entity: Entity {
//!         name: "Hero".to_string(),
//!     },
//!     health: 100,
//! };
//!
//! let player_id = world.add_entity(player);
//!
//! // Iterator-based query (efficient, no allocation)
//! for (id, entity) in world.query::<Entity>() {
//!     println!("Entity: {:?}", *entity);
//! }
//!
//! // Extract specific component
//! if let Ok(health) = world.extract_component::<u32>(&player_id) {
//!     println!("Health: {}", *health);
//! }
//! ```

// Re-export the derive macro
pub use structecs_macros::Extractable;

// Module declarations
mod acquirable;
mod archetype;
mod entity;
mod error;
mod extractable;
mod extractor;
mod world;

// Public exports
pub use acquirable::Acquirable;
pub use entity::EntityId;
pub use error::WorldError;
pub use extractable::{Extractable, ExtractionMetadata};
pub use world::World;
