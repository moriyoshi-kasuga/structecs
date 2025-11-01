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
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Monster {
    pub entity: Entity,
    pub damage: u32,
}

fn main() {
    let mut world = World::default();

    println!("=== Setting up large dataset ===");
    
    // Add many players
    for i in 0..50_000 {
        let player = Player {
            entity: Entity {
                name: format!("Player {}", i),
            },
            health: 100,
            score: i,
        };
        world.add_entity(player);
    }

    // Add some monsters
    for i in 0..10_000 {
        let monster = Monster {
            entity: Entity {
                name: format!("Monster {}", i),
            },
            damage: 10 + (i % 50),
        };
        world.add_entity(monster);
    }

    println!("Total entities: {}", world.entity_count());
    println!("Total archetypes: {}", world.archetype_count());

    // Sequential query benchmark
    println!("\n=== Sequential Query ===");
    let start = std::time::Instant::now();
    let total_health: u32 = world.query_iter::<Player>()
        .map(|(_, player)| player.health)
        .sum();
    let seq_time = start.elapsed();
    println!("Total health (sequential): {} in {:?}", total_health, seq_time);

    // Parallel query benchmark
    println!("\n=== Parallel Query ===");
    let start = std::time::Instant::now();
    let total_health_par: u32 = world.par_query_iter::<Player>()
        .map(|(_, player)| player.health)
        .sum();
    let par_time = start.elapsed();
    println!("Total health (parallel): {} in {:?}", total_health_par, par_time);

    // Speedup calculation
    let speedup = seq_time.as_secs_f64() / par_time.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);

    // More complex parallel operation
    println!("\n=== Complex Parallel Operation ===");
    let start = std::time::Instant::now();
    
    let high_scorers: Vec<_> = world.par_query_iter::<Player>()
        .filter(|(_, player)| player.score > 40_000)
        .map(|(id, player)| (id.id(), player.entity.name.clone(), player.score))
        .collect();
    
    let complex_time = start.elapsed();
    println!("Found {} high scorers in {:?}", high_scorers.len(), complex_time);
    
    // Show first few
    println!("Top 5:");
    for (id, name, score) in high_scorers.iter().take(5) {
        println!("  [{}] {}: {}", id, name, score);
    }

    // Parallel mutation simulation (count entities with specific properties)
    println!("\n=== Parallel Aggregation ===");
    let start = std::time::Instant::now();
    
    let stats = world.par_query_iter::<Player>()
        .map(|(_, player)| (1, player.score as u64))
        .reduce(
            || (0, 0),
            |(count1, sum1), (count2, sum2)| (count1 + count2, sum1 + sum2)
        );
    
    let agg_time = start.elapsed();
    let avg_score = stats.1 / stats.0;
    println!("Player stats: count={}, total_score={}, avg_score={} in {:?}", 
             stats.0, stats.1, avg_score, agg_time);
}
