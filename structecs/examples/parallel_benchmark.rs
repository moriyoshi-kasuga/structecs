use rayon::prelude::*;
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
    pub score: u32,
    pub level: u32,
}

/// Simulate expensive computation per entity
fn calculate_combat_power(player: &Player) -> u64 {
    let mut power = 0u64;

    // Simulate complex calculation
    for _ in 0..100 {
        power += player.health as u64;
        power = power.wrapping_mul(player.score as u64);
        power = power.wrapping_add(player.level as u64);
        power %= 1_000_000_007;
    }

    power
}

fn main() {
    let world = World::default();

    println!("=== Setting up dataset ===");

    // Add players with varying stats
    for i in 0..100_000 {
        let player = Player {
            entity: Entity {
                name: format!("Player {}", i),
            },
            health: 80 + (i % 50),
            score: i,
            level: 1 + (i / 1000),
        };
        world.add_entity(player);
    }

    println!("Total entities: {}", world.entity_count());
    println!("Total archetypes: {}", world.archetype_count());

    // Sequential query with expensive operation
    println!("\n=== Sequential Query (with expensive computation) ===");
    let start = std::time::Instant::now();
    let total_power: u64 = world
        .query_iter::<Player>()
        .map(|(_, player)| calculate_combat_power(&player))
        .sum();
    let seq_time = start.elapsed();
    println!(
        "Total combat power (sequential): {} in {:?}",
        total_power, seq_time
    );

    // Parallel query with expensive operation
    println!("\n=== Parallel Query (with expensive computation) ===");
    let start = std::time::Instant::now();
    let total_power_par: u64 = world
        .par_query_iter::<Player>()
        .map(|(_, player)| calculate_combat_power(&player))
        .sum();
    let par_time = start.elapsed();
    println!(
        "Total combat power (parallel): {} in {:?}",
        total_power_par, par_time
    );

    // Speedup calculation
    let speedup = seq_time.as_secs_f64() / par_time.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);

    if speedup > 1.0 {
        println!("✓ Parallel query is faster for this workload!");
    } else {
        println!("✗ Sequential query is faster (parallel overhead exceeds benefit)");
    }

    // Show when to use each approach
    println!("\n=== Performance Guidelines ===");
    println!("Simple operations (e.g., sum of a field):");
    println!("  Sequential: {:?}", seq_time);

    println!("\nComplex operations (e.g., heavy computation per entity):");
    println!("  Parallel recommended when processing >10k entities with non-trivial work");
}
