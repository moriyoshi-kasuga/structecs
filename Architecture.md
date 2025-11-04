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
    offsets: HashMap<TypeId, usize>,
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
    pub(crate) entities: Vec<(EntityId, EntityData)>,
}
```

### 5. Acquirable: スマートポインタ

```rust
pub struct Acquirable<T: 'static> {
    target: NonNull<T>,
    inner: EntityDataInner,  // 参照カウント
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
    archetypes: DashMap<ArchetypeId, Arc<Archetype>>,
    entity_index: DashMap<EntityId, ArchetypeId>,
    type_index: DashMap<TypeId, FxHashSet<ArchetypeId>>,  // 型からアーキタイプを高速検索
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
    pub fn remove_entity(&self, entity_id: &EntityId) -> Result<(), WorldError>;
    pub fn remove_entities(&self, entity_ids: &[EntityId]);
    pub fn contains_entity(&self, entity_id: &EntityId) -> bool;
    pub fn clear(&self);
    pub fn extract_component<T: 'static>(&self, entity_id: &EntityId) 
        -> Result<Acquirable<T>, WorldError>;
    pub fn query<T: 'static>(&self) 
        -> Vec<(EntityId, Acquirable<T>)>;
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
    pub fn query<T: 'static>(&self) -> Vec<(EntityId, Acquirable<T>)> {
        let type_id = TypeId::of::<T>();
        
        // Type Indexから該当アーキタイプのみを取得
        let archetype_ids: FxHashSet<ArchetypeId> = self.type_index.get(&type_id).map(|ids| ids.clone()).unwrap_or_default();
        
        // 集計用ベクタにスナップショット収集
        let mut results = Vec::new();
        
        for arch_id in archetype_ids {
            if let Some(archetype) = self.archetypes.get(&arch_id) {
                // 安全: Type Indexにより T を含むアーキタイプのみ
                results.extend(unsafe { archetype.iter_component_unchecked::<T>() });
            }
        }
        results
    }
}
```

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
  2. 各Archetypeから iter_component_unchecked() でスナップショット収集
  3. Vecに収集して返却（イテレータではない）
           ↓
ユーザーコード:
  for (id, health) in iter {
    // この時点でロックは一切保持していない
  }
```

**スナップショット戦略:**

- クエリ時に短時間だけロック
- データをコピー（EntityIdと参照カウント増加）
- ロック解放後、イテレータ消費

**メリット:**

- クエリ中に他のスレッドがエンティティ追加可能
- クエリ同士も並列実行可能
- デッドロックのリスクゼロ

### 3. バッチ削除フロー

```
ユーザーコード:
  world.remove_entities(&[id1, id2, id3])
           ↓
World::remove_entities():
  1. entity_idsをアーキタイプごとにグループ化（HashMap）
  2. 各アーキタイプに対して:
     - write lock取得
     - エンティティをバッチ削除
     - write lock解放
  3. entity_indexから削除
  4. 削除成功数を返却
```

**効率性:**

- アーキタイプごとに1回のロック（個別削除はN回ロック）
- HashMap使用で高速グループ化

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
    // 読み取りロック（短時間、スナップショット後解放）
}
```

**ロック競合:** なし（スナップショットは短時間の内部ロック/なし）

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
3. **デッドロックの防止:** ロック順序の一貫性、スナップショット戦略
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
impl Drop for EntityDataInner {
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

### ベンチマーク結果（Release mode）

**基本操作（10,000エンティティ）:**

| 操作 | 時間 | 備考 |
|------|------|------|
| エンティティ追加 | ~16ms | Vec拡張含む |
| 単純クエリ（iter） | ~4ms | アロケーションなし |
| 型指定クエリ | ~3.4ms | フィルタリング込み |
| コンポーネント抽出 | ~100ns | HashMap + ポインタ演算 |

### 最適化のポイント

1. **アーキタイプベースストレージ** - 同じ型のエンティティは連続配置
2. **Extractorキャッシング** - 各型につき1つのExtractor（共有）
3. **イテレータベースAPI** - アロケーションなし
4. **スナップショット戦略** - 短時間のロック保持
5. **細粒度ロック** - アーキタイプ単位の並行処理

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

### 2. スナップショット vs ライブビュー

**判断:** クエリは**スナップショット**を返す。

**採用理由:**

- 並行処理を最優先
- ゲームサーバーでは「少し前の状態」で十分
- メモリは比較的潤沢

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
