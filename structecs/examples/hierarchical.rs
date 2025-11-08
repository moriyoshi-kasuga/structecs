#![allow(clippy::print_stdout, clippy::unwrap_used)]

//! # Hierarchical Component Example
//!
//! This example demonstrates how to create multi-level component hierarchies using structecs.
//! We'll create a 3-level hierarchy: Transform → Visual → Sprite
//!
//! ## Design Pattern
//!
//! Component hierarchies allow you to query entities at different levels of abstraction:
//! - Query `Transform` to get ALL entities with position/rotation
//! - Query `Visual` to get entities with rendering properties
//! - Query `Sprite` to get entities with specific sprite data
//!
//! This pattern is useful for:
//! - Game objects with varying complexity (simple vs. complex entities)
//! - Systems that operate on different levels of abstraction
//! - Type-safe component composition

use structecs::*;

// Level 1: Base component - Transform
// All visual entities have position and rotation
#[derive(Debug, Extractable)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
}

// Level 2: Visual component - extends Transform
// Entities that can be rendered
#[derive(Debug, Extractable)]
#[extractable(transform)]
pub struct Visual {
    pub transform: Transform,
    pub visible: bool,
    pub layer: u32,
}

// Level 3: Sprite component - extends Visual
// Entities with sprite-specific data
#[derive(Debug, Extractable)]
#[extractable(visual)]
pub struct Sprite {
    pub visual: Visual,
    pub texture: String,
    pub width: u32,
    pub height: u32,
}

// Alternative Level 3: ParticleEffect - also extends Visual
#[derive(Debug, Extractable)]
#[extractable(visual)]
pub struct ParticleEffect {
    pub visual: Visual,
    pub particle_count: u32,
    pub lifetime: f32,
}

fn main() {
    println!("=== Hierarchical Component Example ===\n");

    let world = World::default();

    // Add some sprites (3-level hierarchy)
    println!("--- Adding Sprites (3-level hierarchy) ---");
    world.add_entity(Sprite {
        visual: Visual {
            transform: Transform {
                x: 100.0,
                y: 200.0,
                rotation: 0.0,
            },
            visible: true,
            layer: 1,
        },
        texture: "player.png".to_string(),
        width: 32,
        height: 32,
    });

    world.add_entity(Sprite {
        visual: Visual {
            transform: Transform {
                x: 150.0,
                y: 250.0,
                rotation: 45.0,
            },
            visible: true,
            layer: 2,
        },
        texture: "enemy.png".to_string(),
        width: 64,
        height: 64,
    });

    // Add some particle effects (also 3-level hierarchy)
    println!("--- Adding Particle Effects (3-level hierarchy) ---");
    world.add_entity(ParticleEffect {
        visual: Visual {
            transform: Transform {
                x: 300.0,
                y: 400.0,
                rotation: 0.0,
            },
            visible: true,
            layer: 3,
        },
        particle_count: 100,
        lifetime: 2.5,
    });

    // Add a standalone Visual entity (2-level hierarchy)
    println!("--- Adding Standalone Visual (2-level hierarchy) ---");
    world.add_entity(Visual {
        transform: Transform {
            x: 500.0,
            y: 500.0,
            rotation: 90.0,
        },
        visible: false,
        layer: 0,
    });

    println!("\nTotal entities: {}", world.entity_count());
    println!("Total archetypes: {}\n", world.archetype_count());

    // Query at different levels of the hierarchy
    println!("=== Query Level 1: All Transforms ===");
    println!("(This includes ALL entities since everything has a Transform)\n");
    for (id, transform) in world.query::<Transform>() {
        println!(
            "[{}] Transform: pos=({:.1}, {:.1}), rot={:.1}°",
            id, transform.x, transform.y, transform.rotation
        );
    }

    println!("\n=== Query Level 2: All Visuals ===");
    println!("(This includes Sprites, ParticleEffects, and standalone Visuals)\n");
    for (id, visual) in world.query::<Visual>() {
        println!(
            "[{}] Visual: visible={}, layer={}, pos=({:.1}, {:.1})",
            id, visual.visible, visual.layer, visual.transform.x, visual.transform.y
        );
    }

    println!("\n=== Query Level 3a: Only Sprites ===");
    for (id, sprite) in world.query::<Sprite>() {
        println!(
            "[{}] Sprite: texture={}, size={}x{}, pos=({:.1}, {:.1})",
            id, sprite.texture, sprite.width, sprite.height, sprite.visual.transform.x, sprite.visual.transform.y
        );
    }

    println!("\n=== Query Level 3b: Only Particle Effects ===");
    for (id, effect) in world.query::<ParticleEffect>() {
        println!(
            "[{}] ParticleEffect: particles={}, lifetime={:.1}s, pos=({:.1}, {:.1})",
            id, effect.particle_count, effect.lifetime, effect.visual.transform.x, effect.visual.transform.y
        );
    }

    // Demonstrate upward navigation (extracting parent components)
    println!("\n=== Upward Navigation: Extract Parent Components ===");
    println!("Starting from Sprite, extract Visual and Transform\n");

    for (id, sprite) in world.query::<Sprite>() {
        println!("[{}] Sprite: {}", id, sprite.texture);

        // Extract Visual (direct parent)
        if let Some(visual) = sprite.extract::<Visual>() {
            println!("  └─> Visual: layer={}, visible={}", visual.layer, visual.visible);

            // Extract Transform (grandparent)
            if let Some(transform) = visual.extract::<Transform>() {
                println!("      └─> Transform: pos=({:.1}, {:.1})", transform.x, transform.y);
            }
        }
    }

    // Demonstrate system that operates on different levels
    println!("\n=== System Example: Update All Transforms ===");
    println!("(Simulating a physics update that moves all entities)\n");

    for (_id, transform) in world.query::<Transform>() {
        // In a real system, you'd use Mutex/RwLock for mutation
        // This is just demonstrating the query pattern
        println!("Moving entity at ({:.1}, {:.1}) -> ...", transform.x, transform.y);
    }

    // Demonstrate filtering by parent properties
    println!("\n=== Filter Example: Only Visible Sprites on Layer 1+ ===\n");

    for (id, sprite) in world.query::<Sprite>() {
        if sprite.visual.visible && sprite.visual.layer >= 1 {
            println!(
                "[{}] Renderable Sprite: {} (layer {})",
                id, sprite.texture, sprite.visual.layer
            );
        }
    }

    println!("\n✓ Hierarchical component pattern demonstration complete!");
    println!("\nKey Takeaways:");
    println!("  - Use #[extractable(field_name)] to create component hierarchies");
    println!("  - Query at any level to get entities with that component and all derived types");
    println!("  - Extract parent components from child components using extract()");
    println!("  - Design systems that operate on the appropriate level of abstraction");
}
