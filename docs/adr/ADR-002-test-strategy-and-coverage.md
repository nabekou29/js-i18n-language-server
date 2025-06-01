# ADR-002: テスト戦略とカバレッジ目標

- **日付**: 2025-06-02
- **ステータス**: 承認済み
- **タグ**: #テスト #品質保証 #CI/CD #開発プロセス

## 要約

**1行要約**: テストトロフィーアプローチとAI支援により、実用的なテストバランスと高品質（コア90%+、全体80%+）を効率的に達成する

## 問題

### 解決したい課題

LSPサーバーは開発者の日常的なワークフローに直接影響するため、高い品質と信頼性が求められる。以下の課題に対処する必要がある：

1. **品質保証の不足**: 体系的なテスト戦略がないと、バグが本番環境に流出するリスクが高い
2. **リグレッションの防止**: 新機能追加時に既存機能が壊れることを防ぐ必要がある
3. **開発速度の維持**: テストが開発の足かせにならないよう、効率的なテスト環境が必要
4. **カバレッジの可視化**: 品質指標として、テストカバレッジを定量的に測定・追跡する必要がある
5. **AI支援の最大活用**: AIエージェントの能力を活かし、従来困難だった高品質テストを効率的に実現する必要がある

### 影響範囲

- **影響を受けるコンポーネント**: 全モジュール（parser、analyzer、LSPハンドラー等）
- **ユーザーへの影響**: 高品質なLSPサーバーによる開発体験の向上
- **開発への影響**: TDD実践とCI/CDパイプラインの構築

### 制約条件

| 種類     | 制約                     | 理由                                           |
| -------- | ------------------------ | ---------------------------------------------- |
| 技術     | Rust エコシステム        | cargo-llvm-cov等のツールへの依存               |
| リソース | 開発者1名+AIエージェント | AI支援により効率的なテスト開発が可能           |
| 実用性   | 段階的実装               | 完璧を目指さず、重要な部分から着実に進める必要 |

## 選択肢

### 評価基準

| 基準                 | 重み | 説明                                           |
| -------------------- | ---- | ---------------------------------------------- |
| カバレッジ達成可能性 | 高   | AI支援により高カバレッジを現実的に達成できるか |
| 開発効率             | 高   | AI支援でテスト作成が開発を加速するか           |
| 品質向上             | 高   | エッジケース・堅牢性テストを包括できるか       |
| CI/CD統合            | 高   | 自動化パイプラインとの統合が容易か             |

### 選択肢の比較

| 選択肢                                          | カバレッジ達成 | 開発効率 | 品質向上 | CI/CD統合 | 総合評価 |
| ----------------------------------------------- | -------------- | -------- | -------- | --------- | -------- |
| **選択肢1: テストトロフィー・AI支援アプローチ** | ⭐⭐⭐         | ⭐⭐⭐   | ⭐⭐⭐   | ⭐⭐⭐    | **推奨** |
| 選択肢2: テストピラミッド・従来型アプローチ     | ⭐⭐           | ⭐⭐     | ⭐⭐     | ⭐⭐⭐    | 次点     |
| 選択肢3: 最小限のテスト                         | ⭐             | ⭐⭐⭐   | ⭐       | ⭐⭐      | 非推奨   |

### 詳細分析

#### 選択肢1: テストトロフィー・AI支援アプローチ

**アプローチ**: テストトロフィー戦略でモックを最小化し、実際の統合を重視。AI支援により効率的に実装

```
        🏆 Manual/Exploratory Tests
              /          \
           E2E Tests      Performance
         /         \          Tests
    Integration   Property-Based
      Tests          Tests
   /        \
Unit    Snapshot
Tests     Tests
```

**配分**:

- Unit Tests: 40% - 個別機能の正確性
- Integration Tests: 30% - モジュール間連携
- E2E Tests: 10% - ユーザーシナリオ
- Property Tests: 10% - エッジケース発見
- Snapshot Tests: 5% - リグレッション防止
- Performance Tests: 5% - パフォーマンス保証

**トレードオフ**:

- ✅ 実際の統合を重視し、本番環境に近い動作を保証
- ✅ モックの削減によりメンテナンス性向上
- ✅ AI支援により各種テストを効率的に作成
- ✅ スナップショットとパフォーマンステストで品質を多角的に保証
- ❌ 統合テストの実行時間がやや長い
- ❌ 初期セットアップの複雑性

#### 選択肢2: テストピラミッド・従来型アプローチ

**アプローチ**: 従来のテストピラミッドでユニットテストを最重視

**配分**:

- Unit Tests: 70% - モック多用
- Integration Tests: 20% - 限定的な統合
- E2E Tests: 10% - 最小限

**トレードオフ**:

- ✅ 高速なテスト実行
- ✅ 従来の知見を活用
- ❌ モック多用による実際の動作との乖離
- ❌ 統合時の問題発見が遅れる
- ❌ メンテナンスコストが高い

## 決定

**選択肢1: テストトロフィー・AI支援アプローチ** を採用する。

### 決定理由

1. **実動作の保証**: 統合テスト重視により、実際の使用時の動作を確実に保証
2. **メンテナンス性**: モックの最小化により、テストの脆弱性を回避
3. **包括的品質**: スナップショット・パフォーマンステストで多角的な品質保証
4. **AI活用**: 各種テストの効率的な作成・保守をAIが支援
5. **実績のある戦略**: 現代的なテスト戦略として業界で広く採用

### 実装方針

```rust
// テストトロフィーアプローチによるテスト種類と割合
pub enum TestType {
    Unit,        // 40% - 個別機能の正確性
    Integration, // 30% - モジュール間連携
    E2E,         // 10% - ユーザーシナリオ
    Property,    // 10% - エッジケース発見
    Snapshot,    // 5%  - リグレッション防止
    Performance, // 5%  - パフォーマンス保証
}

// テストインフラストラクチャ（モック最小化）
pub struct TestInfrastructure {
    test_server: LspTestServer,     // 実際のLSPサーバーを使用
    helpers: TestHelpers,
    fixtures: TestFixtures,
    coverage: CoverageReporter,
    snapshot: SnapshotManager,       // insta統合
    benchmark: BenchmarkRunner,      // criterion統合
}
```

### 実装への影響

```diff
# Cargo.toml
+ [dev-dependencies]
+ # テストトロフィーアプローチ用ライブラリ
+ tokio = { version = "1.39", features = ["full", "test-util"] }
+ tower = { version = "0.4", features = ["util"] }
+ tower-lsp = { version = "0.20", features = ["proposed"] }
+ tempfile = "3.10"
+ proptest = "1.4"         # プロパティベーステスト
+ insta = { version = "1.39", features = ["yaml", "json"] }  # スナップショット
+ criterion = { version = "0.5", features = ["html_reports"] }  # パフォーマンス
+
+ [[bench]]
+ name = "parser_benchmark"
+ harness = false

# .github/workflows/test.yml
+ - name: Test Suite
+   strategy:
+     matrix:
+       test-type: [unit, integration, property, snapshot]
+   run: |
+     cargo install cargo-llvm-cov
+     case "${{ matrix.test-type }}" in
+       unit)
+         cargo test --lib
+         ;;
+       integration)
+         cargo test --test '*'
+         ;;
+       property)
+         PROPTEST_CASES=1000 cargo test property
+         ;;
+       snapshot)
+         cargo test snapshot
+         ;;
+     esac
```

## 結果

### ポジティブな結果

- 段階的に品質が向上し、実用的なカバレッジを達成
- CI/CDパイプラインによる自動品質チェック
- テスト作成が容易になり、開発者の生産性向上
- リグレッションの早期発見と防止

### ネガティブな結果

- 統合テスト重視による実行時間の増加
- 初期セットアップの複雑性（複数のテストライブラリ）
- AI生成テストの品質維持に継続的なレビューが必要
- スナップショット更新時の慎重な確認が必要

### 測定可能な成功基準

| 指標                   | 目標値            | 測定方法           |
| ---------------------- | ----------------- | ------------------ |
| 全体カバレッジ         | 80%以上           | cargo-llvm-cov     |
| コア機能カバレッジ     | 90%以上           | cargo-llvm-cov     |
| クリティカルパス       | 95%以上           | cargo-llvm-cov     |
| プロパティテスト成功率 | 98%以上           | proptest           |
| スナップショット一致率 | 100%              | insta              |
| パフォーマンス劣化     | 5%以内            | criterion          |
| CI実行時間             | 10分以内          | GitHub Actions     |
| バグ検出率             | リリース前90%以上 | Issue トラッキング |

## 実装計画

### フェーズ1: テストインフラ構築

- cargo-llvm-cov、insta、criterionのセットアップ
- モック最小化TestServerインフラストラクチャ実装
- マトリックスCI/CDパイプライン構築
- 共通テストヘルパー・ビルダーの整備

### フェーズ2: テストトロフィー実装

- ユニットテスト（40%）- パーサー、キー抽出、翻訳ファイル処理
- 統合テスト（30%）- LSPプロトコル、実際のコンポーネント連携
- E2Eテスト（10%）- Neovim プラグインとの統合
- プロパティテスト（10%）- エッジケース発見、不変条件検証
- スナップショットテスト（5%）- LSPレスポンスフォーマット検証
- パフォーマンステスト（5%）- パーサー、キー検索のベンチマーク

## 今後の検討事項

- ベンチマーク結果の継続的追跡と回帰検出自動化
- ミューテーションテストの試験導入（テスト品質評価）
- プロパティテストの高度化（状態マシンモデル）
- スナップショットテストの拡張（エラーメッセージの検証）
- VSCode拡張機能のE2Eテスト自動化強化

## 参考資料

- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [Property-based testing in Rust with Proptest](https://altsysrq.github.io/proptest-book/)
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [Tower-LSP Testing Guide](https://github.com/ebkalderon/tower-lsp)
- [Testing Trophy - Kent C. Dodds](https://kentcdodds.com/blog/the-testing-trophy-and-testing-classifications)
- [insta - Snapshot Testing](https://insta.rs/)
- [Criterion.rs - Statistics-driven Benchmarking](https://bheisler.github.io/criterion.rs/book/)
- ADR-001: 技術スタック選定（tower-lspの採用決定）
