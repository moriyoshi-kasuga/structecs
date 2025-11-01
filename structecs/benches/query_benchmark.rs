use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use structecs::*;

#[derive(Debug, Extractable)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Extractable)]
#[extractable(position)]
pub struct Player {
    pub position: Position,
    pub health: u32,
    pub name: String,
}

#[derive(Debug, Extractable)]
#[extractable(position)]
pub struct Enemy {
    pub position: Position,
    pub damage: u32,
}

fn setup_world(entity_count: usize) -> (World, Vec<EntityId>) {
    let world = World::new();
    let mut ids = Vec::new();

    for i in 0..entity_count / 2 {
        let player = Player {
            position: Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
            health: 100,
            name: format!("Player {}", i),
        };
        ids.push(world.add_entity(player));
    }

    for i in 0..entity_count / 2 {
        let enemy = Enemy {
            position: Position {
                x: i as f32,
                y: 10.0,
                z: 0.0,
            },
            damage: 25,
        };
        ids.push(world.add_entity(enemy));
    }

    (world, ids)
}

fn bench_add_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_entities");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let world = World::new();
                for i in 0..size {
                    let player = Player {
                        position: Position {
                            x: i as f32,
                            y: 0.0,
                            z: 0.0,
                        },
                        health: 100,
                        name: format!("Player {}", i),
                    };
                    world.add_entity(player);
                }
                black_box(world);
            });
        });
    }

    group.finish();
}

fn bench_queryator(c: &mut Criterion) {
    let mut group = c.benchmark_group("queryator");

    for size in [100, 1000, 10000].iter() {
        let (world, _ids) = setup_world(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut count = 0;
                for (_, pos) in world.query::<Position>() {
                    count += 1;
                    black_box(pos);
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

fn bench_query_specific_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_specific_type");

    for size in [100, 1000, 10000].iter() {
        let (world, _ids) = setup_world(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut count = 0;
                for (_, player) in world.query::<Player>() {
                    count += 1;
                    black_box(player);
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

fn bench_extract_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_component");

    let (world, ids) = setup_world(1000);

    group.bench_function("extract_nested", |b| {
        b.iter(|| {
            if let Some(pos) = world.extract_component::<Position>(&ids[0]) {
                black_box(pos);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_add_entities,
    bench_queryator,
    bench_query_specific_type,
    bench_extract_component
);
criterion_main!(benches);
