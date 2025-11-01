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
//! for (id, entity) in world.query_iter::<Entity>() {
//!     println!("Entity: {:?}", *entity);
//! }
//! 
//! // Extract specific component
//! if let Some(health) = world.extract_component::<u32>(&player_id) {
//!     println!("Health: {}", *health);
//! }
//! ```

// Re-export the derive macro
pub use structecs_macros::Extractable;

// Module declarations
mod entity;
mod extractable;
mod extractor;
mod acquirable;
mod archetype;
mod query;
mod world;

// Public exports
pub use entity::{EntityId, EntityData};
pub use extractable::{ExtractionMetadata, Extractable};
pub use extractor::Extractor;
pub use acquirable::Acquirable;
pub use world::World;
pub use query::Query;

// Re-export commonly used traits
pub use std::ops::Deref;
