use structecs::World;

#[derive(Debug, structecs::Extractable)]
struct Player {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    health: u32,
}

#[derive(Debug, structecs::Extractable)]
struct PlayerBuff {
    buff_type: String,
    duration: u32,
}

#[derive(Debug, structecs::Extractable)]
struct PlayerDeathed {
    death_count: u32,
}

#[derive(Debug, structecs::Extractable)]
struct PlayerTag {
    tag: String,
}

#[test]
fn test_add_and_extract_additional() {
    let world = World::new();

    let player = Player {
        name: "Alice".to_string(),
        health: 100,
    };

    let player_id = world.add_entity(player);

    // additionalを追加
    let buff = PlayerBuff {
        buff_type: "Speed".to_string(),
        duration: 10,
    };

    assert!(world.add_additional(&player_id, buff));

    // additionalを抽出
    let extracted_buff = world.extract_additional::<PlayerBuff>(&player_id);
    assert!(extracted_buff.is_some());

    let buff = extracted_buff.unwrap();
    assert_eq!(buff.buff_type, "Speed");
    assert_eq!(buff.duration, 10);
}

#[test]
fn test_has_additional() {
    let world = World::new();

    let player = Player {
        name: "Bob".to_string(),
        health: 80,
    };

    let player_id = world.add_entity(player);

    // 最初はadditionalがない
    assert!(!world.has_additional::<PlayerBuff>(&player_id));

    // additionalを追加
    let buff = PlayerBuff {
        buff_type: "Strength".to_string(),
        duration: 5,
    };
    world.add_additional(&player_id, buff);

    // 追加後はある
    assert!(world.has_additional::<PlayerBuff>(&player_id));
    assert!(!world.has_additional::<PlayerDeathed>(&player_id));
}

#[test]
fn test_remove_additional() {
    let world = World::new();

    let player = Player {
        name: "Charlie".to_string(),
        health: 90,
    };

    let player_id = world.add_entity(player);

    let buff = PlayerBuff {
        buff_type: "Defense".to_string(),
        duration: 15,
    };
    world.add_additional(&player_id, buff);

    assert!(world.has_additional::<PlayerBuff>(&player_id));

    // additionalを削除
    let removed = world.remove_additional::<PlayerBuff>(&player_id);
    assert!(removed.is_some());

    let buff = removed.unwrap();
    assert_eq!(buff.buff_type, "Defense");

    // 削除後はない
    assert!(!world.has_additional::<PlayerBuff>(&player_id));
}

#[test]
fn test_replace_additional() {
    let world = World::new();

    let player = Player {
        name: "David".to_string(),
        health: 70,
    };

    let player_id = world.add_entity(player);

    // 最初のadditional
    let buff1 = PlayerBuff {
        buff_type: "Fire".to_string(),
        duration: 10,
    };
    world.add_additional(&player_id, buff1);

    // 同じ型で上書き
    let buff2 = PlayerBuff {
        buff_type: "Ice".to_string(),
        duration: 20,
    };
    world.add_additional(&player_id, buff2);

    // 新しい値が取得できる
    let extracted = world.extract_additional::<PlayerBuff>(&player_id).unwrap();
    assert_eq!(extracted.buff_type, "Ice");
    assert_eq!(extracted.duration, 20);
}

#[test]
fn test_multiple_additionals() {
    let world = World::new();

    let player = Player {
        name: "Eve".to_string(),
        health: 100,
    };

    let player_id = world.add_entity(player);

    // 複数のadditionalを追加
    let buff = PlayerBuff {
        buff_type: "Haste".to_string(),
        duration: 30,
    };
    world.add_additional(&player_id, buff);

    let deathed = PlayerDeathed { death_count: 3 };
    world.add_additional(&player_id, deathed);

    let tag = PlayerTag {
        tag: "VIP".to_string(),
    };
    world.add_additional(&player_id, tag);

    // 全て取得できる
    assert!(world.has_additional::<PlayerBuff>(&player_id));
    assert!(world.has_additional::<PlayerDeathed>(&player_id));
    assert!(world.has_additional::<PlayerTag>(&player_id));

    let buff = world.extract_additional::<PlayerBuff>(&player_id).unwrap();
    assert_eq!(buff.buff_type, "Haste");

    let deathed = world
        .extract_additional::<PlayerDeathed>(&player_id)
        .unwrap();
    assert_eq!(deathed.death_count, 3);

    let tag = world.extract_additional::<PlayerTag>(&player_id).unwrap();
    assert_eq!(tag.tag, "VIP");
}

#[test]
fn test_query_with_single_additional() {
    let world = World::new();

    let player1_id = world.add_entity(Player {
        name: "Henry".to_string(),
        health: 100,
    });

    world.add_entity(Player {
        name: "Iris".to_string(),
        health: 80,
    });

    // player1にだけbuffを追加
    world.add_additional(
        &player1_id,
        PlayerBuff {
            buff_type: "Power".to_string(),
            duration: 10,
        },
    );

    // クエリで取得
    let results: Vec<_> = world
        .query_with::<Player, (PlayerBuff,)>()
        .query()
        .collect();
    assert_eq!(results.len(), 2);

    for (id, _, (buff,)) in results {
        if id == player1_id {
            assert!(buff.is_some());
            let b = buff.unwrap();
            assert_eq!(b.buff_type, "Power");
        } else {
            assert!(buff.is_none());
        }
    }
}

#[test]
fn test_query_with_multiple_additionals() {
    let world = World::new();

    let player1_id = world.add_entity(Player {
        name: "Jack".to_string(),
        health: 100,
    });

    let player2_id = world.add_entity(Player {
        name: "Kate".to_string(),
        health: 80,
    });

    let player3_id = world.add_entity(Player {
        name: "Leo".to_string(),
        health: 60,
    });

    // player1: buff + deathed
    world.add_additional(
        &player1_id,
        PlayerBuff {
            buff_type: "Agility".to_string(),
            duration: 5,
        },
    );
    world.add_additional(&player1_id, PlayerDeathed { death_count: 1 });

    // player2: buffのみ
    world.add_additional(
        &player2_id,
        PlayerBuff {
            buff_type: "Wisdom".to_string(),
            duration: 15,
        },
    );

    // player3: deathedのみ
    world.add_additional(&player3_id, PlayerDeathed { death_count: 5 });

    // クエリで取得
    let results: Vec<_> = world
        .query_with::<Player, (PlayerBuff, PlayerDeathed)>()
        .query()
        .collect();

    assert_eq!(results.len(), 3);

    for (id, _, (buff, deathed)) in results {
        if id == player1_id {
            assert!(buff.is_some());
            assert!(deathed.is_some());
            assert_eq!(buff.unwrap().buff_type, "Agility");
            assert_eq!(deathed.unwrap().death_count, 1);
        } else if id == player2_id {
            assert!(buff.is_some());
            assert!(deathed.is_none());
            assert_eq!(buff.unwrap().buff_type, "Wisdom");
        } else if id == player3_id {
            assert!(buff.is_none());
            assert!(deathed.is_some());
            assert_eq!(deathed.unwrap().death_count, 5);
        }
    }
}

#[test]
fn test_additional_drop_safety() {
    use std::sync::atomic::{AtomicU32, Ordering};

    static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

    #[derive(Debug, structecs::Extractable)]
    struct DropTracker {
        #[allow(dead_code)]
        value: u32,
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    {
        let world = World::new();
        let player = Player {
            name: "Mike".to_string(),
            health: 100,
        };
        let player_id = world.add_entity(player);

        // DropTrackerを追加
        world.add_additional(&player_id, DropTracker { value: 42 });

        // 確認
        assert!(world.has_additional::<DropTracker>(&player_id));
    } // worldがdrop

    // DropTrackerが正しくdropされたことを確認
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
}
