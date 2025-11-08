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
9. [まとめ](#まとめ)

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
```

> **詳細なコード例**: `examples/hierarchical.rs` を参照してください。

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
```

> **詳細な使用例**: `examples/mutability.rs` を参照してください。

**なぜ`query_mut()`を提供しないのか:**

- Worldの**すべてのアーキタイプ**がロックされる
- 細粒度制御が不可能
- デッドロックのリスク増加

### 3. Systemを強制しない

**哲学:** フレームワークはデータ管理に徹し、ロジックの構造はユーザーに委ねる。

従来のECSフレームワークでは、Systemという特定のパターンを強制することで、ユーザーのロジック記述方法が制限されます。structecsでは、データ管理とロジック記述を分離し、ユーザーが自由に記述できるようにしています。

---

## コアコンセプト

> **API詳細**: 各型の詳細なドキュメントは `cargo doc --open` で確認してください。

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
```

コンパイル時に生成されるメタデータで、型抽出に必要なオフセット情報を保持。`#[derive(Extractable)]`マクロで自動実装されます。

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

**動作原理の概要:**

Extractorは、構造体のメモリレイアウトを解析し、各型のオフセットをマップとして保持します。クエリ時にはこのオフセット情報を使ってゼロコストで型を抽出できます。

> **実装詳細**: `src/extractor.rs` および `cargo doc --open` を参照してください。

### 4. Archetype: 同一構造のエンティティ群

```rust
pub struct Archetype {
    pub(crate) extractor: &'static Extractor,
    pub(crate) entities: Arc<DashMap<EntityId, EntityData, FxBuildHasher>>,
}
```

同じ型のエンティティは同じArchetypeに格納され、キャッシュ効率が向上します。

### 5. Acquirable: スマートポインタ

```rust
pub struct Acquirable<T: 'static> {
    target: NonNull<T>,
    inner: EntityData,  // 参照カウント（Arc）
}
```

**責務:**

1. コンポーネントへの安全な参照
2. エンティティデータのライフタイム管理
3. 同一エンティティからの追加抽出

### 6. World: 中央ストレージ

```rust
pub struct World {
    archetypes: DashMap<ArchetypeId, Archetype, FxBuildHasher>,
    entity_index: DashMap<EntityId, ArchetypeId, FxBuildHasher>,
    type_index: DashMap<TypeId, FxHashSet<ArchetypeId>, FxBuildHasher>,
    next_entity_id: AtomicU32,
}
```

**設計の核心:**

1. **DashMap**: 並行HashMap（ロックフリー読み取り）
2. **Archetype内部にDashMap**: スレッド安全な並行マップで管理
3. **AtomicU32**: ロックフリーなID生成
4. **Type Index**: クエリ最適化のための逆引きマップ

> **主要API**: README.mdまたは `cargo doc --open` を参照してください。

**重要:** すべてのメソッドが`&self`（共有参照）で動作します。

### 7. Type Index: クエリ最適化

**Type Index**は、特定の型を持つアーキタイプを高速に検索するための逆引きマップです。

```rust
type_index: DashMap<TypeId, FxHashSet<ArchetypeId>>
```

エンティティ追加時に、その型が持つすべての抽出可能な型についてインデックスを更新します。クエリ実行時には、Type Indexを使って該当するアーキタイプのみを直接取得できます。

**パフォーマンス向上:**

- アーキタイプ数が多い場合（100+）に特に効果的
- クエリ時間を O(N) → O(M) に削減（N = 全アーキタイプ数、M = 該当アーキタイプ数）

### 8. QueryIter: 遅延評価イテレータ

**QueryIter**は、エンティティを遅延的（オンデマンド）にイテレートする機能を提供します。

```rust
pub struct QueryIter<T: 'static> {
    _phantom: std::marker::PhantomData<T>,
    matching: Vec<(usize, Arc<DashMap<EntityId, EntityData, FxBuildHasher>>)>,
    current: Option<(usize, DashMapIter<'static>)>,
}
```

**query()の特性:**

| 特性 | `query()` |
|------|-----------|
| 戻り値 | `QueryIter<T>` |
| メモリ確保 | 必要なときだけ取得 |
| 遅延評価 | ✅ イテレート時に取得 |
| 大量クエリ | メモリ効率的 |
| 早期終了 | 即座に終了可能 |

**メリット:**

- ✅ 遅延評価: エンティティは`Iterator::next()`呼び出し時に取得される
- ✅ 早期終了: `break`で即座にイテレーションを終了できる
- ✅ メモリ効率: 必要なエンティティのみをオンデマンドで確保

### 9. ComponentHandler: ポリモーフィック動作

**ComponentHandler**は、エンティティ階層に対してポリモーフィックな動作を実現するための仕組みです。

```rust
pub struct ComponentHandler<Base: Extractable, Args = (), Return = ()> {
    function: TypeErasedFn<Args, Return>,
    _marker: std::marker::PhantomData<Base>,
}
```

従来のECSでは、基底型（`Entity`）でクエリしながら実際の型（`Player`、`Zombie`など）に応じた異なる処理を実行することが困難でした。`ComponentHandler`はこれを可能にします。

> **詳細な使用例**: `examples/handler.rs` を参照してください。

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
  2. グローバルキャッシュからExtractorを取得（&'static）
  3. EntityDataをBox確保してポインタ化
  4. ArchetypeIdを計算（TypeId）
  5. Archetypeを取得または作成（DashMap）
  6. Archetype内のDashMapにエンティティ追加
  7. entity_indexに登録（DashMap）
  8. type_indexを更新（該当する全TypeIdに対して）
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
- デッドロックのリスクゼロ

### 3. バッチ削除

structecsは2つのバッチ削除メソッドを提供しています：

#### `remove_entities()` - サイレント削除

存在しないエンティティを**無視**し、エラーを返しません。クリーンアップ処理など、削除失敗を気にしない場合に使用します。

#### `try_remove_entities()` - エラートラッキング削除

存在しないエンティティを**検出**し、`WorldError::PartialRemoval`でエラー情報を返します。削除失敗を追跡する必要がある場合に使用します。

**パフォーマンス比較:**

| 操作 | `remove_entity()` × N | `remove_entities()` | `try_remove_entities()` |
|------|----------------------|---------------------|------------------------|
| ロック回数 | N回 | アーキタイプ数回 | アーキタイプ数回 |
| エラー追跡 | ❌ | ❌ | ✅ |
| オーバーヘッド | 高 | 低 | 中（エラー記録） |

**ベストプラクティス:**

```rust
// ❌ 非効率
for id in entity_ids {
    world.remove_entity(&id).ok();  // N回のロック
}

// ✅ 効率的
world.remove_entities(&entity_ids);  // アーキタイプごとに1回のロック
```

> **詳細な使用例**: `examples/batch_operations.rs` を参照してください。

---

## 並行処理モデル

### ロック戦略

**階層的ロックフリー設計:**

```
Level 1: World構造体自体
  → ロックなし（すべて &self API）

Level 2: DashMap（archetypes, entity_index, type_index）
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
world.query::<Item>();             // Item archetype を読み取り
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

同じアーキタイプへの追加は、Archetype内部のDashMapによって短時間だけ直列化されます。

**ロック競合:** あり（必要最小限、add_entity内部のみ）

### スレッドセーフティ保証

1. **データ競合の防止:** すべての共有状態は`Sync`型
2. **use-after-freeの防止:** `Acquirable`による参照カウント（Arc）
3. **デッドロックの防止:** ロック順序の一貫性、遅延評価による短時間ロック
4. **メモリ安全性:** `T`の`Send`/`Sync`を尊重

> **並行処理の例**: `examples/concurrent.rs` を参照してください。

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

**2. Extractor:**

```rust
pub(crate) extractor: &'static Extractor
```

- グローバルキャッシュに`&'static`として保存
- `inventory` crateを使ってコンパイル時に登録
- 各型につき1つのExtractorを共有（メモリ効率的）

**3. EntityData:**

```rust
pub struct EntityData {
    inner: Arc<EntityDataInner>,
}
```

- Arc（参照カウント）でライフタイム管理
- クローン時は参照カウントのみ増加（軽量）

### メモリ解放

**参照カウントによる遅延解放:**

`EntityData`は`Arc<EntityDataInner>`でラップされているため、エンティティ削除時も`Acquirable`が生きていればデータは保持されます。最後の参照がドロップされた時点で、Extractorの`dropper`関数が呼ばれて安全にメモリが解放されます。

### メモリレイアウト最適化

```rust
#[repr(C)]
pub(crate) struct EntityDataInner {
    pub(crate) data: NonNull<u8>,           // 8 bytes
    pub(crate) extractor: &'static Extractor,  // 8 bytes
}
```

**メモリ効率:**

- **総サイズ**: 16 bytes (Arc のオーバーヘッド除く)
- **アライメント**: 8 bytes
- 前バージョン（`Arc<Extractor>`）から約33%削減（24 bytes → 16 bytes）

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
2. **Extractorキャッシング** - 各型につき1つのExtractor（`&'static`で共有）
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

これにより、ユーザーは自分のユースケースに最適なロック戦略（Atomic、Mutex、RwLockなど）を選択できます。

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

ただし、`ComponentHandler`はデバッグビルドで型関係を検証するため、開発時に型ミスを検出できます。

### 4. Archetype変更の非サポート

**現状:** エンティティ追加後、構造変更不可。

**理由:**

- **ポインタ無効化** - アーキタイプ移動で`Acquirable`が無効化
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
2. 型消去とドロップ（entity.rs）
3. イテレータライフタイム操作（query.rs）

**安全性の保証:**

- ✅ **オフセット計算**: コンパイル時メタデータで検証済み
- ✅ **参照カウント**: Arc パターンを使用（標準ライブラリと同等）
- ✅ **ドロップ**: Extractor生成時に型情報保持

すべての`unsafe`コードは、コメントで安全性の根拠を明示しており、包括的なテストスイート（`tests/`ディレクトリ）で検証されています。

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

### 次のステップ

- **クイックスタート**: [README.md](README.md) を参照
- **API詳細**: `cargo doc --open` でローカルドキュメントを生成
- **実装例**: `examples/` ディレクトリのサンプルコードを確認
- **テスト**: `cargo test --all` で包括的なテストを実行

---

*このドキュメントは、structecsの設計思想と実装の概要を説明しています。詳細な実装情報やAPI仕様は、ソースコードおよび `cargo doc` で生成されるドキュメントを参照してください。*
