# structecs アーキテクチャドキュメント

---

## 📖 目次

1. [概要](#概要)
2. [設計思想](#設計思想)
3. [コアコンセプト](#コアコンセプト)
4. [データフロー](#データフロー)
5. [並行処理モデル](#並行処理モデル)
6. [メモリモデル](#メモリモデル)
7. [パフォーマンス特性](#パフォーマンス特性)
8. [技術的制約と設計判断](#技術的制約と設計判断)

---

## 概要

**structecs**は、従来のECS（Entity Component System）の柔軟性を犠牲にしない、階層的データ構造対応のエンティティ管理フレームワークです。

### 核心的特徴

- **階層的コンポーネント**: OOPのようにデータをネスト可能
- **フラットなアクセス**: ネストの深さに関わらず任意の型を直接クエリ
- **細粒度ロック**: アーキタイプ単位の高並行性
- **ゼロコスト抽象化**: コンパイル時オフセット計算による直接メモリアクセス
- **Systemの押し付けなし**: ユーザーが自由にロジックを記述

### 他のECSとの違い

```
従来のECS (Bevy, specs, hecs):
├─ Entity: ID
├─ Component: 独立した型（フラット）
├─ System: 強制的なアーキテクチャ
└─ Query: コンパイル時型安全

structecs:
├─ Entity: ID
├─ Component: 構造体のフィールド（階層可）
├─ System: なし（ユーザーが自由に実装）
└─ Query: 実行時型抽出（動的かつ柔軟）
```

---

## 設計思想

### 1. データは階層的、アクセスはフラット

**問題意識:**
ゲームサーバー（特にMinecraftのような複雑な階層を持つもの）では、エンティティの関係性が自然に階層構造を形成します。

```rust
Entity
  ├─ name: String
  └─ position: Vec3

LivingEntity
  ├─ entity: Entity     // 継承のような関係
  ├─ health: u32
  └─ max_health: u32

Player
  ├─ living: LivingEntity
  ├─ inventory: Inventory
  └─ game_mode: GameMode
```

**structecsの解決策:**

```rust
#[derive(Extractable)]
pub struct Entity {
    pub name: String,
    pub position: Vec3,
}

#[derive(Extractable)]
#[extractable(entity)]  // ← Entityを明示的に抽出可能としてマーク
pub struct LivingEntity {
    pub entity: Entity,
    pub health: u32,
    pub max_health: u32,
}

#[derive(Extractable)]
#[extractable(living)]  // ← LivingEntityを明示的に抽出可能としてマーク
pub struct Player {
    pub living: LivingEntity,
    pub inventory: Inventory,
    pub game_mode: GameMode,
}

// struct/enum単位でクエリ可能（階層内の明示的にマークされた型）
for (id, entity) in world.query::<Entity>() {
    println!("Name: {}", entity.name);
}

for (id, living) in world.query::<LivingEntity>() {
    println!("Health: {}/{}", living.health, living.max_health);
}

for (id, player) in world.query::<Player>() {
    println!("Player: {}", player.living.entity.name);
}
```

**重要な制約:**

- デフォルトでは**struct/enum単位**でのみ抽出可能
- 個別のフィールド（`u32`, `String`など）は抽出できない
- ネストした型も`#[extractable(field_name)]`で明示的にマークしない限り抽出不可

**この設計の理由:**

1. **New type patternとの衝突回避** - `Health(u32)`と`Mana(u32)`を区別
2. **明確な意図** - 型に意味を持たせる
3. **型安全性** - プリミティブ型のクエリは曖昧

### 2. ユーザーが可変性を制御する

**設計判断:** Worldは**読み取り専用アクセス**のみを提供し、可変性はユーザーが管理する。

**実装パターン:**

```rust
// パターン1: Atomicを使う（ロックフリー）
#[derive(Extractable)]
pub struct Player {
    pub name: String,
    pub health: AtomicU32,  // ← ロックフリーな変更
}

// パターン2: Mutexを使う（細粒度ロック）
#[derive(Extractable)]
pub struct Inventory {
    pub items: Mutex<Vec<Item>>,  // ← 必要な時だけロック
}

// パターン3: RwLockを使う（読み取り/書き込み分離）
#[derive(Extractable)]
pub struct Position {
    pub coords: RwLock<Vec3>,
}
```

**なぜ`query_mut()`を提供しないのか:**

- Worldの**すべてのアーキタイプ**がロックされる
- 細粒度制御が不可能
- デッドロックのリスク増加

### 3. Systemを強制しない

**哲学:** フレームワークはデータ管理に徹し、ロジックの構造はユーザーに委ねる。

```rust
// 好きなように書ける
fn update_physics(world: &World, delta: f32) {
    for (id, pos) in world.query::<Position>() {
        let vel = world.extract_component::<Vec3>(&id).unwrap();
        let mut pos = pos.write().unwrap();
        pos.x += vel.x * delta;
    }
}
```

---

## コアコンセプト

### 1. Entity: エンティティ識別子

```rust
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct EntityId {
    pub(crate) id: u32,
}
```

**特性:**

- `Copy`: 軽量、スタックコピー可能
- `Hash`: HashMap/DashMapのキーとして使用
- 32bit: 40億エンティティまでサポート

### 2. Component: 抽出可能な型

```rust
pub trait Extractable: 'static + Sized {
    const METADATA_LIST: &'static [ExtractionMetadata];
}

pub enum ExtractionMetadata {
    Target {
        type_id: TypeId,
        offset: usize,
    },
    Nested {
        type_id: TypeId,
        offset: usize,
        nested: &'static [ExtractionMetadata],
    },
}
```

コンパイル時に生成されるメタデータで、型抽出に必要なオフセット情報を保持。

### 3. Extractor: 型抽出エンジン

```rust
pub struct Extractor {
    offsets: FxHashMap<TypeId, usize>,
    dropper: unsafe fn(NonNull<u8>),
}
```

**責務:**

1. 型からメモリオフセットを計算（事前計算済み）
2. ポインタ演算でコンポーネントにアクセス
3. エンティティの安全なドロップ

**動作原理:**

```rust
// Player構造体のメモリレイアウト
Player {
    entity: Entity {      // offset: 0
        name: String,     // offset: 0
    },
    health: u32,          // offset: 24
}

// Extractorが保持するオフセットマップ
offsets = {
    TypeId(Entity): 0,
    TypeId(u32): 24,
}

// 抽出時（ゼロコスト！）
let player_ptr: *const Player = ...;
let health_ptr = player_ptr.offset(24) as *const u32;
```

### 4. Archetype: 同一構造のエンティティ群

```rust
pub struct Archetype {
    pub(crate) extractor: Arc<Extractor>,
    pub(crate) entities: Arc<DashMap<EntityId, EntityData, FxBuildHasher>>,
}
```

### 5. Acquirable: スマートポインタ

```rust
pub struct Acquirable<T: 'static> {
    target: NonNull<T>,
    inner: EntityData,  // 参照カウント
}

impl<T> Deref for Acquirable<T> {
    type Target = T;
    fn deref(&self) -> &T { ... }
}
```

**責務:**

1. コンポーネントへの安全な参照
2. エンティティデータのライフタイム管理（Arc的な動作）
3. 同一エンティティからの追加抽出

### 6. World: 中央ストレージ

```rust
pub struct World {
    archetypes: DashMap<ArchetypeId, Archetype, FxBuildHasher>,
    entity_index: DashMap<EntityId, ArchetypeId, FxBuildHasher>,
    type_index: DashMap<TypeId, FxHashSet<ArchetypeId>, FxBuildHasher>,  // 型からアーキタイプを高速検索
    next_entity_id: AtomicU32,
}
```

**設計の核心:**

1. **DashMap**: 並行HashMap（ロックフリー読み取り）
2. **Archetype内部にDashMap**: アーキタイプはスレッド安全な並行マップで管理
3. **AtomicU32**: ロックフリーなID生成
4. **Type Index**: クエリ最適化のための逆引きマップ

**主要API:**

```rust
impl World {
    pub fn add_entity<E: Extractable>(&self, entity: E) -> EntityId;
    pub fn add_entity_with_acquirable<E: Extractable>(&self, entity: E) -> (EntityId, Acquirable<E>);
    pub fn add_entities<E: Extractable>(&self, entities: impl IntoIterator<Item = E>) -> Vec<EntityId>;
    pub fn remove_entity(&self, entity_id: &EntityId) -> Result<(), WorldError>;
    pub fn try_remove_entities(&self, entity_ids: &[EntityId]) -> Result<(), WorldError>;
    pub fn remove_entities(&self, entity_ids: &[EntityId]);
    pub fn contains_entity(&self, entity_id: &EntityId) -> bool;
    pub fn clear(&self);
    pub fn extract_component<T: 'static>(&self, entity_id: &EntityId) 
        -> Result<Acquirable<T>, WorldError>;
    pub fn query<T: 'static>(&self) -> QueryIter<T>;
    pub fn entity_count(&self) -> usize;
    pub fn archetype_count(&self) -> usize;
}
```

**重要:** すべてのメソッドが`&self`（共有参照）で動作。

### 7. Type Index: クエリ最適化

**Type Index**は、特定の型を持つアーキタイプを高速に検索するための逆引きマップです。

```rust
type_index: DashMap<TypeId, FxHashSet<ArchetypeId>>
```

**動作原理:**

```rust
// エンティティ追加時に更新
world.add_entity(Player { ... });
  ↓
// Playerが持つすべての型に対してインデックス更新
type_index.entry(TypeId::of::<Player>()).or_default().push(archetype_id);
type_index.entry(TypeId::of::<Entity>()).or_default().push(archetype_id);
type_index.entry(TypeId::of::<String>()).or_default().push(archetype_id);
// ... (Playerが持つすべての抽出可能な型)

// クエリ実行時に活用
world.query::<Health>();
  ↓
// Type Indexで直接該当アーキタイプ集合を取得
let archetype_ids: FxHashSet<ArchetypeId> = type_index.get(&TypeId::of::<Health>()).cloned().unwrap_or_default();
for archetype_id in &archetype_ids {
    if let Some(archetype) = archetypes.get(archetype_id) {
        // ...
    }
}
```

**パフォーマンス向上:**

- アーキタイプ数が多い場合（100+）に特に効果的
- クエリ時間を O(N) → O(M) に削減（N = 全アーキタイプ数、M = 該当アーキタイプ数）
- メモリオーバーヘッドは最小限（各型につき小さなVec）

**実装例:**

```rust
impl World {
    pub fn query<T: 'static>(&self) -> QueryIter<T> {
        let type_id = TypeId::of::<T>();
        
        // Type Indexから該当アーキタイプのみを取得
        let archetype_ids: FxHashSet<ArchetypeId> = self.type_index.get(&type_id).map(|ids| ids.clone()).unwrap_or_default();
        
        // イテレータを構築
        let mut matching = Vec::new();
        
        for arch_id in archetype_ids {
            if let Some(archetype) = self.archetypes.get(&arch_id) {
                // 安全: Type Indexにより T を含むアーキタイプのみ
                let offset = archetype.extractor.offsets.get(&type_id).copied().unwrap();
                matching.push((offset, archetype.entities.clone()));
            }
        }
        
        QueryIter {
            _phantom: std::marker::PhantomData,
            matching,
            current: None,
        }
    }
}
```

### 8. QueryIter: 遅延評価イテレータ

**QueryIter**は、`query()`とは異なり、エンティティを遅延的（オンデマンド）にイテレートする機能を提供します。

```rust
pub struct QueryIter<T: 'static> {
    _phantom: std::marker::PhantomData<T>,
    matching: Vec<(usize, Arc<DashMap<EntityId, EntityData, FxBuildHasher>>)>,
    current: Option<(usize, DashMapIter<'static>)>,
}
```

**query():**

`query()`は`QueryIter<T>`を返す遅延評価イテレータです。

| 特性 | `query()` |
|------|-----------|
| 戻り値 | `QueryIter<T>` |
| メモリ確保 | 必要なときだけ取得 |
| 遅延評価 | ✅ イテレート時に取得 |
| 大量クエリ | メモリ効率的 |
| 早期終了 | 即座に終了可能 |

**使用例:**

```rust
// query(): 遅延評価でPlayerを取得（メモリ効率的）
for (id, player) in world.query::<Player>() {
    if player.name == "Hero" {
        break;  // 即座に終了、残りは未確保
    }
}
```

**動作原理:**

```rust
impl<T: Extractable> Iterator for QueryIter<T> {
    type Item = (EntityId, Acquirable<T>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 現在のアーキタイプのイテレータから次の要素を取得
            if let Some((offset, current_iter)) = &mut self.current {
                if let Some(entry) = current_iter.next() {
                    let entity_id = *entry.key();
                    let entity_data = entry.value();
                    // SAFETY: オフセットは事前計算済み
                    return Some((entity_id, unsafe { 
                        entity_data.extract_by_offset(*offset) 
                    }));
                } else {
                    self.current = None;  // 次のアーキタイプへ
                }
            } else if let Some((offset, next_map)) = self.matching.pop() {
                // 次のアーキタイプのイテレータを取得
                let iter = next_map.iter();
                // SAFETY: Arcで保持しているため、ライフタイムは安全
                let iter = unsafe { 
                    std::mem::transmute::<DashMapIter<'_>, DashMapIter<'static>>(iter) 
                };
                self.current = Some((offset, iter));
            } else {
                return None;  // すべて消費済み
            }
        }
    }
}
```

**メモリ効率性:**

```rust
// シナリオ: 10,000体のPlayerから1体を検索

// query(): 効率的
for (id, player) in world.query::<Player>() {
    if player.level > 100 {
        break;  // 必要な分だけ確保して即座に終了
    }
}
```

**イテレータの特性:**

- ✅ 遅延評価: エンティティは`Iterator::next()`呼び出し時に取得される
- ✅ 早期終了: `break`で即座にイテレーションを終了できる
- ✅ メモリ効率: 必要なエンティティのみをオンデマンドで確保
- ✅ 大規模クエリ: 数万〜数十万エンティティでもメモリ使用量は最小限

### 9. ComponentHandler: ポリモーフィック動作

**ComponentHandler**は、エンティティ階層に対してポリモーフィックな動作を実現するための仕組みです。

```rust
pub struct ComponentHandler<Base: Extractable, Args = (), Return = ()> {
    function: TypeErasedFn<Args, Return>,
    _marker: std::marker::PhantomData<Base>,
}
```

**目的:**

従来のECSでは、`Entity`型でクエリしながら実際の型（`Player`、`Zombie`など）に応じた異なる処理を実行することが困難でした。`ComponentHandler`はこれを可能にします。

**使用例:**

```rust
#[derive(Extractable)]
pub struct Entity {
    pub name: String,
}

#[derive(Extractable)]
#[extractable(entity)]  // ← Entityを抽出可能にする
pub struct Player {
    pub entity: Entity,
    pub level: u32,
}

#[derive(Extractable)]
#[extractable(entity)]
pub struct Zombie {
    pub entity: Entity,
    pub health: u32,
}

// Player用のハンドラ
let player_handler = ComponentHandler::<Entity>::for_type::<Player>(|player, ()| {
    println!("Player {} died!", player.entity.name);
});

// Zombie用のハンドラ
let zombie_handler = ComponentHandler::<Entity>::for_type::<Zombie>(|zombie, ()| {
    println!("Zombie {} was killed!", zombie.entity.name);
});

// Entityでクエリして、実際の型に応じた処理を実行
for (id, entity) in world.query::<Entity>() {
    // 実行時に適切なハンドラを選択
    if let Ok(player) = world.extract_component::<Player>(&id) {
        player_handler.call(&player, ());
    } else if let Ok(zombie) = world.extract_component::<Zombie>(&id) {
        zombie_handler.call(&zombie, ());
    }
}
```

**型安全性の保証:**

`ComponentHandler`はデバッグビルドで型関係を検証します：

```rust
#[cfg(debug_assertions)]
fn validate_type_relationship<Concrete: Extractable>() {
    if !can_extract::<Concrete, Base>() {
        panic!(
            "The concrete type must contain the base type in its \
             extraction metadata. Did you forget #[extractable(...)]?"
        );
    }
}
```

**動作原理:**

1. **型消去（Type Erasure）**: 具体的な関数を`Box<dyn Fn>`に変換
2. **実行時抽出**: `EntityData`から動的に`Concrete`型を抽出
3. **型安全検証**: デバッグビルドで型関係を事前検証

```rust
struct TypeErasedFn<Args, Return> {
    caller: Box<dyn Fn(EntityData, Args) -> Return + Send + Sync>,
}

impl<Args, Return> TypeErasedFn<Args, Return> {
    pub fn new<Base, Concrete>(
        func: impl Fn(&Acquirable<Concrete>, Args) -> Return + Send + Sync + 'static,
    ) -> Self {
        let caller = move |data: EntityData, args: Args| -> Return {
            // SAFETY: デバッグビルドで検証済み
            let entity = data.extract::<Concrete>()
                .expect("Handler type mismatch");
            func(&entity, args)
        };
        
        Self { caller: Box::new(caller) }
    }
}
```

**実用例: ダメージシステム**

```rust
// 汎用的なダメージハンドラを定義
type DamageHandler = ComponentHandler<Entity, u32, ()>;

let player_damage = DamageHandler::for_type::<Player>(|player, damage| {
    let new_health = player.health.saturating_sub(damage);
    println!("Player took {} damage! Health: {}", damage, new_health);
});

let zombie_damage = DamageHandler::for_type::<Zombie>(|zombie, damage| {
    let new_health = zombie.health.saturating_sub(damage);
    println!("Zombie took {} damage! Health: {}", damage, new_health);
});

// すべてのエンティティにダメージを適用
for (id, entity) in world.query::<Entity>() {
    if let Ok(player) = world.extract_component::<Player>(&id) {
        player_damage.call(&player, 10);
    } else if let Ok(zombie) = world.extract_component::<Zombie>(&id) {
        zombie_damage.call(&zombie, 5);
    }
}
```

**メリット:**

- ✅ **型安全**: デバッグビルドで型ミスを検出
- ✅ **柔軟性**: 実行時にハンドラを選択可能
- ✅ **ゼロコスト（Release）**: 型検証はデバッグビルドのみ
- ✅ **並行安全**: `Send + Sync`で複数スレッドから利用可能

---

## データフロー

### 1. エンティティ登録フロー

```
ユーザーコード:
  Player { entity, health } を作成
           ↓
  world.add_entity(player)
           ↓
World::add_entity():
  1. AtomicU32でEntityId生成（ロックフリー）
  2. Extractorを取得またはキャッシュから取得（DashMap）
  3. EntityDataをBox確保してポインタ化
  4. ArchetypeIdを計算（TypeId）
  5. Archetypeを取得または作成（DashMap）
  6. Archetype.write().add_entity() （細粒度ロック）
  7. entity_indexに登録（DashMap）
           ↓
結果: EntityId返却
```

**並行性:**

- 異なるアーキタイプへの追加 → 完全並列
- 同じアーキタイプへの追加 → Archetype内部の並行マップで短時間の排他制御

### 2. クエリ実行フロー

```
ユーザーコード:
  world.query::<Health>()
           ↓
World::query():
  1. Type Indexから該当Archetype集合を取得
  2. 各ArchetypeのDashMapへの参照（Arc）を収集
  3. QueryIterを構築して返却（遅延評価イテレータ）
           ↓
ユーザーコード:
  for (id, health) in world.query::<Health>() {
    // Iterator::next()呼び出し時に初めてエンティティを取得
    // この時点でロックは一切保持していない
  }
```

**遅延評価戦略:**

- クエリ時はArchetypeの参照のみを収集（軽量）
- イテレート時に必要なエンティティだけを取得（オンデマンド）
- 各エンティティ取得時に短時間だけロック、即座に解放

**メリット:**

- メモリ使用量が最小限（必要なエンティティのみ確保）
- 早期終了が可能（`break`で即座に終了）
- クエリ中に他のスレッドがエンティティ追加可能
- クエリ同士も並列実行可能
- デッドロックのリスクゼロ

### 3. バッチ削除フロー

structecsは2つのバッチ削除メソッドを提供しています：

#### `remove_entities()` - サイレント削除

```rust
pub fn remove_entities(&self, entity_ids: &[EntityId])
```

**特性:**

- ✅ 存在しないエンティティを**無視**する
- ✅ エラーを返さない（`void`）
- ✅ 削除失敗を気にしない場合に使用

**実装フロー:**

```
ユーザーコード:
  world.remove_entities(&[id1, id2, id3])
           ↓
World::remove_entities():
  1. entity_idsをアーキタイプごとにグループ化（FxHashMap）
     - 存在しないIDは無視（entity_indexに存在チェック）
  2. 各アーキタイプに対して:
     - Archetype.remove_entity()を呼び出し
     - 削除失敗を無視（let _ = ...）
  3. entity_indexから削除（存在するもののみ）
```

**コード例:**

```rust
// 実装（簡略版）
pub fn remove_entities(&self, entity_ids: &[EntityId]) {
    let mut archetype_groups: FxHashMap<ArchetypeId, Vec<EntityId>> = FxHashMap::default();
    
    for entity_id in entity_ids {
        if let Some((_, archetype_id)) = self.entity_index.remove(entity_id) {
            archetype_groups
                .entry(archetype_id)
                .or_default()
                .push(*entity_id);
        }
        // 存在しないエンティティは無視
    }
    
    for (archetype_id, entities) in archetype_groups {
        if let Some(archetype) = self.archetypes.get(&archetype_id) {
            for entity_id in entities {
                let _ = archetype.remove_entity(&entity_id);  // エラーを無視
            }
        }
    }
}
```

**使用例:**

```rust
// クリーンアップ処理（削除失敗を気にしない）
let dead_entities = vec![id1, id2, id3];
world.remove_entities(&dead_entities);  // 既に削除済みでもOK
```

#### `try_remove_entities()` - エラートラッキング削除

```rust
pub fn try_remove_entities(&self, entity_ids: &[EntityId]) -> Result<(), WorldError>
```

**特性:**

- ✅ 存在しないエンティティを**検出**する
- ✅ エラー情報を返す（`Result`）
- ✅ 削除失敗を追跡する必要がある場合に使用

**実装フロー:**

```
ユーザーコード:
  world.try_remove_entities(&[id1, id2, id3])?
           ↓
World::try_remove_entities():
  1. entity_idsをアーキタイプごとにグループ化（FxHashMap）
     - 存在しないIDを`not_found`ベクタに記録
  2. 各アーキタイプに対して:
     - Archetype.remove_entity()を呼び出し
     - 削除失敗を記録
  3. entity_indexから削除
  4. エラーがあれば`WorldError::PartialRemoval`を返却
```

**コード例:**

```rust
// 実装（簡略版）
pub fn try_remove_entities(&self, entity_ids: &[EntityId]) -> Result<(), WorldError> {
    let mut archetype_groups: FxHashMap<ArchetypeId, Vec<EntityId>> = FxHashMap::default();
    let mut not_found = Vec::new();
    
    for entity_id in entity_ids {
        if let Some((_, archetype_id)) = self.entity_index.remove(entity_id) {
            archetype_groups
                .entry(archetype_id)
                .or_default()
                .push(*entity_id);
        } else {
            not_found.push(*entity_id);  // 記録する
        }
    }
    
    let mut removed = Vec::new();
    let mut failed = not_found;
    
    for (archetype_id, entities) in archetype_groups {
        if let Some(archetype) = self.archetypes.get(&archetype_id) {
            for entity_id in entities {
                match archetype.remove_entity(&entity_id) {
                    Ok(()) => removed.push(entity_id),
                    Err(_) => failed.push(entity_id),  // 失敗を記録
                }
            }
        }
    }
    
    if !failed.is_empty() {
        return Err(WorldError::PartialRemoval { removed, failed });
    }
    Ok(())
}
```

**使用例:**

```rust
// 厳密な削除処理（失敗を検出したい）
match world.try_remove_entities(&entity_ids) {
    Ok(()) => println!("すべて削除成功"),
    Err(WorldError::PartialRemoval { removed, failed }) => {
        println!("削除成功: {:?}", removed);
        println!("削除失敗: {:?}", failed);
        // エラーハンドリング...
    }
    Err(e) => eprintln!("エラー: {:?}", e),
}
```

#### パフォーマンス比較

| 操作 | `remove_entity()` × N | `remove_entities()` | `try_remove_entities()` |
|------|----------------------|---------------------|------------------------|
| ロック回数 | N回 | アーキタイプ数回 | アーキタイプ数回 |
| エラー追跡 | ❌ | ❌ | ✅ |
| オーバーヘッド | 高 | 低 | 中（エラー記録） |
| 使用例 | 単一削除 | 大量削除（エラー無視） | 大量削除（エラー検出） |

**効率性:**

- アーキタイプごとに1回のロック（個別削除はN回ロック）
- FxHashMap使用で高速グループ化
- エンティティ数が多いほど効率向上

**ベストプラクティス:**

```rust
// ❌ 非効率
for id in entity_ids {
    world.remove_entity(&id).ok();  // N回のロック
}

// ✅ 効率的
world.remove_entities(&entity_ids);  // アーキタイプごとに1回のロック

// ✅ エラー検出が必要な場合
world.try_remove_entities(&entity_ids)?;
```

---

## 並行処理モデル

### ロック戦略

**階層的ロックフリー設計:**

```
Level 1: World構造体自体
  → ロックなし（すべて &self API）

Level 2: DashMap（archetypes, extractors, entity_index）
  → 内部シャーディング、ロックフリー読み取り

Level 3: Archetype
  → 内部はDashMap（並列対応、短時間アクセス）

Level 4: コンポーネント内部
  → ユーザー制御（Atomic, Mutex, RwLock）
```

### 並行性のパターン

#### パターン1: 異なるアーキタイプへの操作（完全並列）

```rust
// スレッド1
world.add_entity(Player { ... });  // Player archetype をロック

// スレッド2（同時実行）
world.add_entity(Monster { ... }); // Monster archetype をロック

// スレッド3（同時実行）
world.query::<Item>();             // Item archetype を読み取りロック
```

**ロック競合:** なし

#### パターン2: 同一アーキタイプへの読み取り（並列可能）

```rust
// スレッド1、2、3すべて同時実行可能
for (id, player) in world.query::<Player>() {
    // Iterator::next()呼び出し時に短時間だけロック、即座に解放
}
```

**ロック競合:** なし（エンティティ取得時のみ短時間ロック）

#### パターン3: 同一アーキタイプへの書き込み（直列化）

```rust
// スレッド1
world.add_entity(Player { ... });
// Player archetype の write() ロック取得

// スレッド2（待機）
world.add_entity(Player { ... });
// スレッド1のロック解放待ち
```

**ロック競合:** あり（必要最小限、add_entity内部のみ）

### スレッドセーフティ保証

1. **データ競合の防止:** すべての共有状態は`Sync`型
2. **use-after-freeの防止:** `Acquirable`による参照カウント
3. **デッドロックの防止:** ロック順序の一貫性、遅延評価による短時間ロック
4. **メモリ安全性:** `T`の`Send`/`Sync`を尊重

---

## メモリモデル

### メモリ確保戦略

**1. エンティティデータ:**

```rust
let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
```

- ヒープ確保（Box）
- ポインタ化して`NonNull<u8>`で保持
- 型消去（type erasure）だが、Extractorが型情報を保持

**2. 参照カウンタ:**

```rust
let counter = Box::leak(Box::new(AtomicUsize::new(1))).into();
```

- ヒープ確保（独立したBox）
- `leak`して寿命管理を手動化
- すべての`Acquirable`で共有

**3. Archetype:**

```rust
pub(crate) entities: Vec<(EntityId, EntityData)>,
```

- 動的拡張（capacity倍増戦略）

### メモリ解放

**参照カウントによる遅延解放:**

```rust
impl Drop for EntityData {
    fn drop(&mut self) {
        if self.counter.fetch_sub(1, Ordering::Release) > 1 {
            return;  // まだ他にAcquirableが存在
        }
        // 最後の参照がドロップされた
        unsafe { (self.extractor.dropper)(self.data) };
        unsafe { drop(Box::from_raw(self.counter.as_ptr())) };
    }
}
```

エンティティ削除時も`Acquirable`が生きていればデータは保持されます。

### メモリレイアウト最適化

```rust
#[repr(C)]
pub(crate) struct EntityDataInner {
    pub(crate) counter: AtomicUsize,  // 8 bytes
    pub(crate) data: NonNull<u8>,     // 8 bytes
    pub(crate) extractor: Arc<Extractor>,  // 8 bytes
}
```

**メモリ効率:**

- **総サイズ**: 24 bytes (padding: 0 bytes)
- **アライメント**: 8 bytes

---

## パフォーマンス特性

### 主要ECSフレームワークとの比較ベンチマーク（Release mode）

**Bevy ECS**, **hecs**, **specs**との性能比較。

#### エンティティ追加（10,000エンティティ）

| フレームワーク | 時間 (µs) | 相対速度 |
|---------------|-----------|----------|
| **hecs** | 577.72 | 1.00x（最速） |
| **bevy_ecs** | 707.38 | 1.22x |
| **specs** | 890.30 | 1.54x |
| **structecs** | 958.05 | 1.66x |

#### 全コンポーネントクエリ（10,000エンティティ）

| フレームワーク | 時間 (µs) | 相対速度 |
|---------------|-----------|----------|
| **bevy_ecs** | 5.46 | 1.00x（最速） |
| **specs** | 15.56 | 2.85x |
| **hecs** | 19.19 | 3.52x |
| **structecs** | 73.50 | 13.46x |

#### 2コンポーネントクエリ（10,000エンティティ）

| フレームワーク | 時間 (µs) | 相対速度 |
|---------------|-----------|----------|
| **bevy_ecs** | 4.14 | 1.00x（最速） |
| **hecs** | 4.94 | 1.19x |
| **specs** | 14.24 | 3.44x |
| **structecs** | 75.95 | 18.34x |

#### ネストしたコンポーネントクエリ（structecsのみ）

structecsの独自機能である階層的コンポーネントのクエリ性能：

| エンティティ数 | 時間 (µs) |
|---------------|-----------|
| 100 | 1.10 |
| 1,000 | 7.64 |
| 10,000 | 77.38 |

### パフォーマンス分析

**強み:**

- ✅ **競争力のある追加性能**: 最速のhecsと比較して約1.66倍遅い程度で、階層的コンポーネントの追加メタデータを考慮すれば妥当な範囲
- ✅ **スケーラビリティ**: エンティティ数が増加しても線形的な性能低下
- ✅ **独自機能**: 階層的コンポーネントのクエリは他のフレームワークでは実現不可能

**トレードオフ:**

- ⚠️ **クエリ性能**: 従来のECSフレームワークと比較してクエリが遅い理由：
  1. **動的型抽出オーバーヘッド**: 実行時の`TypeId`ベース抽出
  2. **階層的コンポーネントサポート**: 他のフレームワークにはない機能のコスト
  3. **Type Indexルックアップ**: 柔軟なクエリのための追加オーバーヘッド

**使い分けの指針:**

| シナリオ | 推奨フレームワーク |
|---------|-------------------|
| 最高のクエリ性能が必要 | bevy_ecs / hecs |
| 階層的エンティティ構造が必要 | **structecs** |
| 従来のECSパターンで十分 | bevy_ecs / hecs / specs |
| 複雑なゲームサーバー（Minecraft等） | **structecs** |
| フレーム単位で数百万エンティティを処理 | bevy_ecs / hecs |
| エンティティ関係性が重要 | **structecs** |

### 最適化のポイント

1. **アーキタイプベースストレージ** - 同じ型のエンティティは連続配置
2. **Extractorキャッシング** - 各型につき1つのExtractor（共有）
3. **遅延評価イテレータ** - 必要なエンティティのみをオンデマンドで確保
4. **短時間ロック** - エンティティ取得時のみロック、即座に解放
5. **細粒度ロック** - アーキタイプ単位の並行処理
6. **Type Index** - クエリ時のアーキタイプ検索を高速化

### 今後の最適化方針

1. **クエリパフォーマンス改善**
   - Type Indexのさらなる最適化
   - キャッシュ局所性の向上
   - 並列イテレーションのサポート

2. **メモリ効率化**
   - アーキタイプストレージの圧縮
   - EntityDataの最適化

3. **並行性能向上**
   - より細かい粒度のロック戦略
   - ロックフリーアルゴリズムの導入

---

## 技術的制約と設計判断

### 1. なぜwrite APIを提供しないのか

**判断:** `query_mut()` や `extract_component_mut()` は**提供しない**。

**理由:**

- **World全体のロック競合** - すべてのアーキタイプがロック
- **柔軟性の喪失** - 細かいロック戦略を選択できない
- **デッドロックのリスク** - RwLockは再入不可

**代替案:**

```rust
// ユーザーがロック粒度を制御
let player = world.extract_component::<Mutex<PlayerState>>(&id)?;
let mut state = player.lock().unwrap();
```

### 2. 遅延評価イテレータ

**判断:** クエリは**遅延評価イテレータ**を返す。

**採用理由:**

- メモリ効率性（必要なエンティティのみ確保）
- 早期終了が可能（`break`で即座に終了）
- 並行処理を最優先（短時間ロックで即座に解放）
- 大規模クエリでもメモリ使用量が一定

### 3. 動的型抽出 vs コンパイル時型安全

**判断:** 実行時の`TypeId`ベース抽出を採用。

**採用理由:**

- 柔軟性（任意の型を動的に抽出可能）
- 階層構造のサポート
- ユーザーが型を知らなくても良い（プラグインシステム等）

**代償:**

- `Option`で失敗可能
- 型ミスがコンパイル時に検出されない

### 4. Archetype変更の非サポート

**現状:** エンティティ追加後、構造変更不可。

**理由:**

- **ポインタ無効化** - アーキタイプ移動でAcquirableが無効化
- **実装複雑性** - 世代番号管理が必要

**現在の回避策:**

```rust
struct Player {
    health: u32,
    buff: Option<Buff>,  // ← Optionで表現
}
```

ユーザーが独自のシステムで動的なコンポーネントを管理することが推奨されます。

### 5. unsafe コードの使用

**使用箇所:**

1. ポインタ演算（extractor.rs）
2. 参照カウント操作（entity.rs）
3. 型消去とドロップ（entity.rs）

**安全性の保証:**

- ✅ **オフセット計算**: コンパイル時`offset_of!`で検証済み
- ✅ **参照カウント**: Arc パターンを手動実装（well-tested）
- ✅ **ドロップ**: Extractor生成時に型情報保持

---

## まとめ

structecsは、**階層的データ構造**と**高並行性**を両立させる、新しいアプローチのECSフレームワークです。

### 核心的価値

1. **データは階層的、アクセスはフラット** - OOPとECSの良いとこ取り
2. **ユーザーが可変性を制御** - 最適なロック戦略を選択可能
3. **細粒度ロック** - アーキタイプ単位の並行処理
4. **Systemの押し付けなし** - 自由なロジック記述

### 向いているプロジェクト

- ✅ 複雑なゲームサーバー（Minecraft, MMO）
- ✅ 階層的エンティティ構造
- ✅ 高並行処理要求
- ✅ 柔軟なロジック記述

### 向いていないプロジェクト

- ❌ シンプルなゲーム（従来のECSで十分）
- ❌ 最大パフォーマンス追求（マイクロ秒単位の最適化）
- ❌ 既存ECSエコシステムに依存
- ❌ 完全なコンパイル時型安全性が必須

---

*このドキュメントは、structecsの設計思想・実装詳細を説明しています。詳細なテスト情報は`cargo test`で確認してください。*
