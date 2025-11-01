use structecs::{Extractable, World};

#[derive(Debug, Clone, PartialEq, Eq, Extractable)]
struct Player {
    name: String,
    score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Extractable)]
struct Enemy {
    health: u32,
}

#[test]
fn test_empty_world_operations() {
    let world = World::default();
    
    // Query empty world
    assert_eq!(world.query_iter::<Player>().count(), 0);
    
    // Parallel query empty world
    use rayon::prelude::*;
    let count: usize = world.par_query_iter::<Player>().count();
    assert_eq!(count, 0);
}

#[test]
fn test_single_entity() {
    let world = World::default();
    
    let id = world.add_entity(Player {
        name: "Solo".to_string(),
        score: 100,
    });
    
    assert_eq!(world.query_iter::<Player>().count(), 1);
    
    let (_id, player) = world.query_iter::<Player>().next().unwrap();
    assert_eq!(player.name, "Solo");
    assert_eq!(player.score, 100);
    
    let removed = world.remove_entity(&id);
    assert!(removed);
    assert_eq!(world.query_iter::<Player>().count(), 0);
}

#[test]
fn test_multiple_same_type_entities() {
    let world = World::default();
    
    // structecsは自動的にIDを生成するので、同じ型の複数エンティティを追加可能
    let id1 = world.add_entity(Player {
        name: "First".to_string(),
        score: 100,
    });
    
    let id2 = world.add_entity(Player {
        name: "Second".to_string(),
        score: 200,
    });
    
    let id3 = world.add_entity(Player {
        name: "Third".to_string(),
        score: 300,
    });
    
    // 3つのエンティティが存在するはず
    assert_eq!(world.query_iter::<Player>().count(), 3);
    
    // それぞれのIDは異なるはず
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
    
    // 1つ削除
    let removed = world.remove_entity(&id2);
    assert!(removed);
    assert_eq!(world.query_iter::<Player>().count(), 2);
    
    // 残りのエンティティを確認
    let names: Vec<_> = world.query_iter::<Player>()
        .map(|(_, p)| p.name.clone())
        .collect();
    assert!(names.contains(&"First".to_string()));
    assert!(names.contains(&"Third".to_string()));
    assert!(!names.contains(&"Second".to_string()));
}

#[test]
fn test_empty_string_name() {
    let world = World::default();
    
    world.add_entity(Player {
        name: String::new(),
        score: 0,
    });
    
    let (_id, player) = world.query_iter::<Player>().next().unwrap();
    assert_eq!(player.name, "");
}

#[test]
fn test_very_long_string() {
    let world = World::default();
    
    let long_name = "a".repeat(10_000);
    world.add_entity(Player {
        name: long_name.clone(),
        score: 0,
    });
    
    let (_id, player) = world.query_iter::<Player>().next().unwrap();
    assert_eq!(player.name, long_name);
    assert_eq!(player.name.len(), 10_000);
}

#[test]
fn test_unicode_string() {
    let world = World::default();
    
    world.add_entity(Player {
        name: "こんにちは世界🌍".to_string(),
        score: 42,
    });
    
    let (_id, player) = world.query_iter::<Player>().next().unwrap();
    assert_eq!(player.name, "こんにちは世界🌍");
}

#[test]
fn test_remove_twice() {
    let world = World::default();
    
    let id = world.add_entity(Player {
        name: "Test".to_string(),
        score: 0,
    });
    
    // 最初の削除は成功
    let first_remove = world.remove_entity(&id);
    assert!(first_remove);
    
    // 2回目の削除は失敗（既に削除済み）
    let second_remove = world.remove_entity(&id);
    assert!(!second_remove);
}

#[test]
fn test_query_after_all_removed() {
    let world = World::default();
    
    let mut ids = Vec::new();
    for i in 0..100 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
        ids.push(id);
    }
    
    assert_eq!(world.query_iter::<Player>().count(), 100);
    
    // 全て削除
    for id in ids {
        world.remove_entity(&id);
    }
    
    assert_eq!(world.query_iter::<Player>().count(), 0);
}

#[test]
fn test_parallel_query_single_entity() {
    use rayon::prelude::*;
    
    let world = World::default();
    
    world.add_entity(Player {
        name: "Solo".to_string(),
        score: 100,
    });
    
    let count: usize = world.par_query_iter::<Player>().count();
    assert_eq!(count, 1);
}

#[test]
fn test_interleaved_insert_remove() {
    let world = World::default();
    
    let mut active_ids = Vec::new();
    
    for i in 0..100 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
        active_ids.push(id);
        
        // 2つごとに古いものを削除
        if active_ids.len() > 2 {
            let old_id = active_ids.remove(0);
            world.remove_entity(&old_id);
        }
    }
    
    // 最後の2つだけが残っているはず
    assert_eq!(world.query_iter::<Player>().count(), active_ids.len());
}

#[test]
fn test_zero_score() {
    let world = World::default();
    
    world.add_entity(Player {
        name: "Zero".to_string(),
        score: 0,
    });
    
    let (_id, player) = world.query_iter::<Player>().next().unwrap();
    assert_eq!(player.score, 0);
}

#[test]
fn test_max_score() {
    let world = World::default();
    
    world.add_entity(Player {
        name: "Max".to_string(),
        score: u32::MAX,
    });
    
    let (_id, player) = world.query_iter::<Player>().next().unwrap();
    assert_eq!(player.score, u32::MAX);
}

#[test]
fn test_consecutive_queries() {
    let world = World::default();
    
    for i in 0..100 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
    }
    
    // 連続してクエリを実行
    for _ in 0..10 {
        assert_eq!(world.query_iter::<Player>().count(), 100);
    }
}

#[test]
fn test_query_during_modifications() {
    let world = World::default();
    
    for i in 0..50 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
    }
    
    let initial_count = world.query_iter::<Player>().count();
    assert_eq!(initial_count, 50);
    
    // クエリ後に追加
    for i in 50..100 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
    }
    
    // 新しいクエリで確認
    let final_count = world.query_iter::<Player>().count();
    assert_eq!(final_count, 100);
}

#[test]
fn test_different_types_same_world() {
    let world = World::default();
    
    // 異なる型を同じWorldに追加
    let player_id = world.add_entity(Player {
        name: "Hero".to_string(),
        score: 100,
    });
    
    let enemy_id = world.add_entity(Enemy {
        health: 50,
    });
    
    // それぞれのクエリで確認
    assert_eq!(world.query_iter::<Player>().count(), 1);
    assert_eq!(world.query_iter::<Enemy>().count(), 1);
    
    // 片方を削除
    world.remove_entity(&player_id);
    
    // もう片方は残っているはず
    assert_eq!(world.query_iter::<Player>().count(), 0);
    assert_eq!(world.query_iter::<Enemy>().count(), 1);
    
    world.remove_entity(&enemy_id);
    assert_eq!(world.query_iter::<Enemy>().count(), 0);
}

#[test]
fn test_rapid_add_remove_cycle() {
    let world = World::default();
    
    for iteration in 0..100 {
        let id = world.add_entity(Player {
            name: format!("Iteration{}", iteration),
            score: iteration,
        });
        
        if iteration % 2 == 0 {
            world.remove_entity(&id);
        }
    }
    
    // 奇数回のiterationsのみ残っているはず（50個）
    assert_eq!(world.query_iter::<Player>().count(), 50);
}

#[test]
fn test_many_entities_same_type() {
    let world = World::default();
    
    let count = 1_000;
    let mut ids = Vec::new();
    
    for i in 0..count {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
        ids.push(id);
    }
    
    assert_eq!(world.query_iter::<Player>().count(), count as usize);
    
    // 半分を削除
    for i in (0..count).step_by(2) {
        world.remove_entity(&ids[i as usize]);
    }
    
    assert_eq!(world.query_iter::<Player>().count(), (count / 2) as usize);
}

#[test]
fn test_archetype_with_clones() {
    let world = World::default();
    
    let template = Player {
        name: "Template".to_string(),
        score: 100,
    };
    
    // 同じデータを持つ複数のエンティティを追加
    for _ in 0..10 {
        world.add_entity(template.clone());
    }
    
    assert_eq!(world.query_iter::<Player>().count(), 10);
    
    // 全て同じデータを持っているはず
    for (_id, player) in world.query_iter::<Player>() {
        assert_eq!(player.name, "Template");
        assert_eq!(player.score, 100);
    }
}

#[test]
fn test_parallel_query_after_modifications() {
    use rayon::prelude::*;
    
    let world = World::default();
    
    for i in 0..100 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
    }
    
    let initial_count: usize = world.par_query_iter::<Player>().count();
    assert_eq!(initial_count, 100);
    
    // いくつか追加
    for i in 100..150 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            score: i,
        });
    }
    
    // パラレルクエリで再度カウント
    let final_count: usize = world.par_query_iter::<Player>().count();
    assert_eq!(final_count, 150);
}

#[test]
fn test_entity_count_tracking() {
    let world = World::default();
    
    assert_eq!(world.entity_count(), 0);
    
    let id1 = world.add_entity(Player {
        name: "Player1".to_string(),
        score: 100,
    });
    assert_eq!(world.entity_count(), 1);
    
    let id2 = world.add_entity(Enemy {
        health: 50,
    });
    assert_eq!(world.entity_count(), 2);
    
    world.remove_entity(&id1);
    assert_eq!(world.entity_count(), 1);
    
    world.remove_entity(&id2);
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn test_archetype_count_tracking() {
    let world = World::default();
    
    // 初期状態ではアーキタイプは0
    assert_eq!(world.archetype_count(), 0);
    
    // Player追加でアーキタイプ+1
    world.add_entity(Player {
        name: "Player".to_string(),
        score: 100,
    });
    assert_eq!(world.archetype_count(), 1);
    
    // 同じ型を追加してもアーキタイプは増えない
    world.add_entity(Player {
        name: "Player2".to_string(),
        score: 200,
    });
    assert_eq!(world.archetype_count(), 1);
    
    // 別の型を追加するとアーキタイプ+1
    world.add_entity(Enemy {
        health: 50,
    });
    assert_eq!(world.archetype_count(), 2);
}
