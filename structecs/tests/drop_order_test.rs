//! Drop Order and Correctness Tests
//!
//! コンポーネントのドロップ順序と正確性を検証するテストスイート

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use structecs::*;

// Macro to create test-specific drop-tracked types
macro_rules! make_drop_tracked {
    ($name:ident, $counter:ident) => {
        static $counter: AtomicUsize = AtomicUsize::new(0);

        #[derive(Debug, Extractable)]
        struct $name {
            id: usize,
            _data: Vec<u8>,
        }

        impl $name {
            fn new(id: usize) -> Self {
                Self {
                    id,
                    _data: vec![42; 10],
                }
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                $counter.fetch_add(1, Ordering::SeqCst);
            }
        }
    };
}

#[test]
fn test_component_drops_on_entity_removal() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();
    let entity_id = world.add_entity(Component::new(1));

    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 0);

    world.remove_entity(&entity_id).unwrap();

    // コンポーネントが正しくドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1);
}

#[test]
fn test_component_drops_on_world_drop() {
    make_drop_tracked!(Component, DROP_COUNTER);

    {
        let world = World::new();
        world.add_entity(Component::new(1));
        world.add_entity(Component::new(2));
        world.add_entity(Component::new(3));

        assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 0);
    } // Worldがドロップ

    // すべてのコンポーネントがドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 3);
}

#[test]
fn test_extracted_component_drops_correctly() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();
    let entity_id = world.add_entity(Component::new(1));

    {
        let extracted = world.extract_component::<Component>(&entity_id);
        assert!(extracted.is_ok());
        assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 0);
    } // extractedがドロップ

    // 抽出されたコンポーネントはドロップされない（Arcで共有されているため）
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 0);

    // Worldをドロップすると実際にドロップされる
    drop(world);
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1);
}

#[test]
fn test_multiple_entities_drop_all_components() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();

    for i in 0..100 {
        world.add_entity(Component::new(i));
    }

    drop(world);

    // すべてのコンポーネントがドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 100);
}

#[test]
fn test_partial_entity_removal_drops_only_removed() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();

    let id1 = world.add_entity(Component::new(1));
    let id2 = world.add_entity(Component::new(2));
    let _id3 = world.add_entity(Component::new(3));

    world.remove_entity(&id2).unwrap();

    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1);

    // 他のエンティティはまだ存在する
    let comp1 = world.extract_component::<Component>(&id1);
    assert!(comp1.is_ok());

    drop(world);

    // comp1がまだ参照を保持しているため、まだドロップされていない
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 2);

    drop(comp1);

    // comp1をドロップすると、残りの2つもドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 3);
}

#[test]
fn test_nested_arc_drops_correctly() {
    let outer_arc = Arc::new(vec![1, 2, 3, 4, 5]);
    let weak_ref = Arc::downgrade(&outer_arc);

    #[derive(Debug, Extractable)]
    struct EntityWithArc {
        data: Arc<Vec<i32>>,
    }

    {
        let world = World::new();
        world.add_entity(EntityWithArc {
            data: outer_arc.clone(),
        });

        // Arcの参照カウントは2（outer_arcとエンティティ内）
        assert_eq!(Arc::strong_count(&outer_arc), 2);
    } // Worldがドロップ

    // Worldがドロップされた後、参照カウントは1に戻る
    assert_eq!(Arc::strong_count(&outer_arc), 1);

    drop(outer_arc);

    // すべての強参照が解放される
    assert!(weak_ref.upgrade().is_none());
}

#[test]
fn test_query_iteration_does_not_affect_drops() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();

    for i in 0..10 {
        world.add_entity(Component::new(i));
    }

    // クエリでイテレートしてもドロップは発生しない
    for (_id, _comp) in world.query::<Component>() {
        // 何もしない
    }

    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 0);

    drop(world);

    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 10);
}

#[test]
fn test_extraction_then_drop_world() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();

    let id1 = world.add_entity(Component::new(1));
    let _id2 = world.add_entity(Component::new(2));

    let extracted = world.extract_component::<Component>(&id1).unwrap();

    // Worldをドロップ
    drop(world);

    // Worldがドロップされると、extractedしていないid2がドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1);

    drop(extracted);

    // extractedをドロップすると、id1も解放される
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 2);
}

#[test]
fn test_large_component_drops_without_leak() {
    static DROP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct LargeComponent {
        id: usize,
        large_data: Vec<u8>,
    }

    impl LargeComponent {
        fn new(id: usize) -> Self {
            Self {
                id,
                large_data: vec![0; 1024 * 1024], // 1MB
            }
        }
    }

    impl Drop for LargeComponent {
        fn drop(&mut self) {
            DROP_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
    }

    {
        let world = World::new();

        for i in 0..10 {
            world.add_entity(LargeComponent::new(i));
        }
    }

    // すべての大きなコンポーネントが正しくドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 10);
}

#[test]
fn test_drop_order_with_mixed_operations() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();

    let id1 = world.add_entity(Component::new(1));
    let id2 = world.add_entity(Component::new(2));
    let _id3 = world.add_entity(Component::new(3));

    // 一つ削除
    world.remove_entity(&id2).unwrap();
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1);

    // 一つ抽出
    let _extracted = world.extract_component::<Component>(&id1).unwrap();

    // 新しく追加
    let _id4 = world.add_entity(Component::new(4));

    // Worldをドロップ
    drop(world);

    // Arcで管理されているため、まだドロップされていない可能性がある
    // extractedがまだ保持している
    assert!(DROP_COUNTER.load(Ordering::SeqCst) >= 1);

    // extractedをドロップ
    drop(_extracted);

    // すべてドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 4);
}

#[test]
fn test_zst_component_drops() {
    static ZST_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct ZstComponent;

    impl Drop for ZstComponent {
        fn drop(&mut self) {
            ZST_DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    {
        let world = World::new();

        for _ in 0..50 {
            world.add_entity(ZstComponent);
        }
    }

    // ZSTコンポーネントも正しくドロップされる
    assert_eq!(ZST_DROP_COUNT.load(Ordering::SeqCst), 50);
}

#[test]
fn test_drop_with_panic_recovery() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();

    for i in 0..10 {
        world.add_entity(Component::new(i));
    }

    // パニックが発生してもドロップは実行される
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(world);
        panic!("Test panic");
    }));

    assert!(result.is_err());

    // パニック後もコンポーネントはドロップされている
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 10);
}

#[test]
fn test_empty_world_drops_cleanly() {
    let world = World::new();
    drop(world); // エンプティでもクリーンにドロップされる
}

#[test]
fn test_drop_many_entities_stress() {
    make_drop_tracked!(Component, DROP_COUNTER);

    {
        let world = World::new();

        for i in 0..1000 {
            world.add_entity(Component::new(i));
        }
    }

    // 1000個すべてがドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1000);
}

#[test]
fn test_alternating_add_remove_drops() {
    make_drop_tracked!(Component, DROP_COUNTER);

    let world = World::new();
    let mut ids = Vec::new();

    // 交互に追加と削除
    for i in 0..50 {
        let id = world.add_entity(Component::new(i));
        ids.push(id);

        if i % 2 == 0 && !ids.is_empty() {
            let remove_id = ids.remove(0);
            world.remove_entity(&remove_id).unwrap();
        }
    }

    // いくつかドロップされている
    let dropped_so_far = DROP_COUNTER.load(Ordering::SeqCst);
    assert!(dropped_so_far > 0);

    drop(world);

    // すべてがドロップされる
    assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 50);
}
