use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

// ============================================================================
// Common Components
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
}

#[derive(Debug, Clone)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone)]
pub struct Name {
    pub value: String,
}

// ============================================================================
// structecs implementations
// ============================================================================

mod structecs_bench {
    use super::*;
    use structecs::*;

    #[derive(Debug, Extractable)]
    pub struct StructecsPosition {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    #[derive(Debug, Extractable)]
    pub struct StructecsVelocity {
        pub dx: f32,
        pub dy: f32,
        pub dz: f32,
    }

    #[derive(Debug, Extractable)]
    pub struct StructecsHealth {
        pub current: u32,
        pub max: u32,
    }

    #[derive(Debug, Extractable)]
    pub struct StructecsName {
        pub value: String,
    }

    #[derive(Debug, Extractable)]
    #[extractable(position)]
    pub struct Entity {
        pub position: StructecsPosition,
        pub name: StructecsName,
    }

    #[derive(Debug, Extractable)]
    #[extractable(entity, velocity, health)]
    pub struct Player {
        pub entity: Entity,
        pub velocity: StructecsVelocity,
        pub health: StructecsHealth,
    }

    pub fn add_entities(count: usize) -> World {
        let world = World::new();
        for i in 0..count {
            world.add_entity(Player {
                entity: Entity {
                    position: StructecsPosition {
                        x: i as f32,
                        y: 0.0,
                        z: 0.0,
                    },
                    name: StructecsName {
                        value: format!("Entity {}", i),
                    },
                },
                velocity: StructecsVelocity {
                    dx: 1.0,
                    dy: 0.0,
                    dz: 0.0,
                },
                health: StructecsHealth {
                    current: 100,
                    max: 100,
                },
            });
        }
        world
    }

    pub fn query_all(world: &World) -> usize {
        let mut count = 0;
        for (_, player) in world.query::<Player>() {
            count += 1;
            black_box(&player);
        }
        count
    }

    pub fn query_position_velocity(world: &World) -> usize {
        let mut count = 0;
        for (_, pos) in world.query::<StructecsPosition>() {
            count += 1;
            black_box(&pos);
        }
        count
    }

    pub fn query_nested(world: &World) -> usize {
        let mut count = 0;
        for (_, entity) in world.query::<Entity>() {
            count += 1;
            black_box(&entity);
        }
        count
    }
}

// ============================================================================
// bevy_ecs implementations
// ============================================================================

mod bevy_bench {
    use super::*;
    use bevy_ecs::prelude::*;

    #[derive(Component, Clone)]
    pub struct BevyPosition(pub Position);

    #[derive(Component, Clone)]
    pub struct BevyVelocity(pub Velocity);

    #[derive(Component, Clone)]
    pub struct BevyHealth(pub Health);

    #[derive(Component, Clone)]
    pub struct BevyName(pub Name);

    pub fn add_entities(count: usize) -> World {
        let mut world = World::new();
        for i in 0..count {
            world.spawn((
                BevyPosition(Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                }),
                BevyVelocity(Velocity {
                    dx: 1.0,
                    dy: 0.0,
                    dz: 0.0,
                }),
                BevyHealth(Health {
                    current: 100,
                    max: 100,
                }),
                BevyName(Name {
                    value: format!("Entity {}", i),
                }),
            ));
        }
        world
    }

    pub fn query_all(world: &mut World) -> usize {
        let mut count = 0;
        let mut query = world.query::<(&BevyPosition, &BevyVelocity, &BevyHealth, &BevyName)>();
        for item in query.iter(world) {
            count += 1;
            black_box(item);
        }
        count
    }

    pub fn query_position_velocity(world: &mut World) -> usize {
        let mut count = 0;
        let mut query = world.query::<(&BevyPosition, &BevyVelocity)>();
        for item in query.iter(world) {
            count += 1;
            black_box(item);
        }
        count
    }
}

// ============================================================================
// hecs implementations
// ============================================================================

mod hecs_bench {
    use super::*;
    use hecs::World;

    pub fn add_entities(count: usize) -> World {
        let mut world = World::new();
        for i in 0..count {
            world.spawn((
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
                Velocity {
                    dx: 1.0,
                    dy: 0.0,
                    dz: 0.0,
                },
                Health {
                    current: 100,
                    max: 100,
                },
                Name {
                    value: format!("Entity {}", i),
                },
            ));
        }
        world
    }

    pub fn query_all(world: &World) -> usize {
        let mut count = 0;
        for item in world.query::<(&Position, &Velocity, &Health, &Name)>().iter() {
            count += 1;
            black_box(item);
        }
        count
    }

    pub fn query_position_velocity(world: &World) -> usize {
        let mut count = 0;
        for item in world.query::<(&Position, &Velocity)>().iter() {
            count += 1;
            black_box(item);
        }
        count
    }
}

// ============================================================================
// specs implementations
// ============================================================================

mod specs_bench {
    use super::*;
    use specs::prelude::*;

    #[derive(Clone, Debug)]
    pub struct SpecsPosition(pub Position);
    impl Component for SpecsPosition {
        type Storage = VecStorage<Self>;
    }

    #[derive(Clone, Debug)]
    pub struct SpecsVelocity(pub Velocity);
    impl Component for SpecsVelocity {
        type Storage = VecStorage<Self>;
    }

    #[derive(Clone, Debug)]
    pub struct SpecsHealth(pub Health);
    impl Component for SpecsHealth {
        type Storage = VecStorage<Self>;
    }

    #[derive(Clone, Debug)]
    pub struct SpecsName(pub Name);
    impl Component for SpecsName {
        type Storage = VecStorage<Self>;
    }

    pub fn add_entities(count: usize) -> World {
        let mut world = World::new();
        world.register::<SpecsPosition>();
        world.register::<SpecsVelocity>();
        world.register::<SpecsHealth>();
        world.register::<SpecsName>();

        for i in 0..count {
            world
                .create_entity()
                .with(SpecsPosition(Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                }))
                .with(SpecsVelocity(Velocity {
                    dx: 1.0,
                    dy: 0.0,
                    dz: 0.0,
                }))
                .with(SpecsHealth(Health {
                    current: 100,
                    max: 100,
                }))
                .with(SpecsName(Name {
                    value: format!("Entity {}", i),
                }))
                .build();
        }
        world
    }

    pub fn query_all(world: &World) -> usize {
        let mut count = 0;
        let positions = world.read_storage::<SpecsPosition>();
        let velocities = world.read_storage::<SpecsVelocity>();
        let healths = world.read_storage::<SpecsHealth>();
        let names = world.read_storage::<SpecsName>();

        for (pos, vel, health, name) in
            (&positions, &velocities, &healths, &names).join()
        {
            count += 1;
            black_box((pos, vel, health, name));
        }
        count
    }

    pub fn query_position_velocity(world: &World) -> usize {
        let mut count = 0;
        let positions = world.read_storage::<SpecsPosition>();
        let velocities = world.read_storage::<SpecsVelocity>();

        for (pos, vel) in (&positions, &velocities).join() {
            count += 1;
            black_box((pos, vel));
        }
        count
    }
}

// ============================================================================
// Benchmark Definitions
// ============================================================================

fn bench_add_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_entities");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("structecs", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let world = structecs_bench::add_entities(size);
                    black_box(world);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("bevy_ecs", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let world = bevy_bench::add_entities(size);
                    black_box(world);
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("hecs", size), size, |b, &size| {
            b.iter(|| {
                let world = hecs_bench::add_entities(size);
                black_box(world);
            });
        });

        group.bench_with_input(BenchmarkId::new("specs", size), size, |b, &size| {
            b.iter(|| {
                let world = specs_bench::add_entities(size);
                black_box(world);
            });
        });
    }

    group.finish();
}

fn bench_query_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_all_components");

    for size in [100, 1000, 10000].iter() {
        let structecs_world = structecs_bench::add_entities(*size);
        group.bench_with_input(
            BenchmarkId::new("structecs", size),
            size,
            |b, _| {
                b.iter(|| {
                    let count = structecs_bench::query_all(&structecs_world);
                    black_box(count);
                });
            },
        );

        let mut bevy_world = bevy_bench::add_entities(*size);
        group.bench_with_input(
            BenchmarkId::new("bevy_ecs", size),
            size,
            |b, _| {
                b.iter(|| {
                    let count = bevy_bench::query_all(&mut bevy_world);
                    black_box(count);
                });
            },
        );

        let hecs_world = hecs_bench::add_entities(*size);
        group.bench_with_input(BenchmarkId::new("hecs", size), size, |b, _| {
            b.iter(|| {
                let count = hecs_bench::query_all(&hecs_world);
                black_box(count);
            });
        });

        let specs_world = specs_bench::add_entities(*size);
        group.bench_with_input(BenchmarkId::new("specs", size), size, |b, _| {
            b.iter(|| {
                let count = specs_bench::query_all(&specs_world);
                black_box(count);
            });
        });
    }

    group.finish();
}

fn bench_query_two_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_two_components");

    for size in [100, 1000, 10000].iter() {
        let structecs_world = structecs_bench::add_entities(*size);
        group.bench_with_input(
            BenchmarkId::new("structecs", size),
            size,
            |b, _| {
                b.iter(|| {
                    let count = structecs_bench::query_position_velocity(&structecs_world);
                    black_box(count);
                });
            },
        );

        let mut bevy_world = bevy_bench::add_entities(*size);
        group.bench_with_input(
            BenchmarkId::new("bevy_ecs", size),
            size,
            |b, _| {
                b.iter(|| {
                    let count = bevy_bench::query_position_velocity(&mut bevy_world);
                    black_box(count);
                });
            },
        );

        let hecs_world = hecs_bench::add_entities(*size);
        group.bench_with_input(BenchmarkId::new("hecs", size), size, |b, _| {
            b.iter(|| {
                let count = hecs_bench::query_position_velocity(&hecs_world);
                black_box(count);
            });
        });

        let specs_world = specs_bench::add_entities(*size);
        group.bench_with_input(BenchmarkId::new("specs", size), size, |b, _| {
            b.iter(|| {
                let count = specs_bench::query_position_velocity(&specs_world);
                black_box(count);
            });
        });
    }

    group.finish();
}

fn bench_nested_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_nested_components");

    for size in [100, 1000, 10000].iter() {
        let structecs_world = structecs_bench::add_entities(*size);
        group.bench_with_input(
            BenchmarkId::new("structecs", size),
            size,
            |b, _| {
                b.iter(|| {
                    let count = structecs_bench::query_nested(&structecs_world);
                    black_box(count);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_add_entities,
    bench_query_all,
    bench_query_two_components,
    bench_nested_query
);
criterion_main!(benches);
