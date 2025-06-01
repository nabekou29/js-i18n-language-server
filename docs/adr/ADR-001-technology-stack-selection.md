# ADR-001: i18n LSP技術スタック選定

- **日付**: 2025-06-01
- **ステータス**: 承認済み
- **タグ**: #アーキテクチャ #技術選定 #LSP #Rust

## 要約

**1行要約**: i18n LSPサーバーの開発にRust + tower-lsp + tokio + tree-sitterを採用し、高性能で拡張可能なLSP実装を実現する。

## 問題

### 解決したい課題

多言語対応プロジェクトにおける開発効率を向上させるため、i18n（国際化）専用のLanguage Server Protocol (LSP) サーバーを開発する必要がある。このLSPサーバーは、翻訳キーの補完、検証、リファクタリング、バーチャルテキスト表示などの機能を提供し、開発者の多言語対応作業を支援する。

### 影響範囲

- **影響を受けるコンポーネント**: LSPサーバー全体のアーキテクチャ
- **ユーザーへの影響**: エディタでのi18n開発体験の向上
- **開発への影響**: Rust非同期プログラミングの知識が必要

### 制約条件

| 種類           | 制約                        | 理由                         |
| -------------- | --------------------------- | ---------------------------- |
| 技術           | Rust stable (1.75+)         | 安定性とエコシステムの成熟度 |
| パフォーマンス | 1000+キーでリアルタイム補完 | 大規模プロジェクトでの実用性 |
| 機能           | LSP標準準拠                 | エディタ互換性の確保         |

## 選択肢

### 評価基準

| 基準           | 重み | 説明                               |
| -------------- | ---- | ---------------------------------- |
| パフォーマンス | 高   | 大規模プロジェクトでの応答性が重要 |
| 実装コスト     | 中   | 開発効率とメンテナンス性           |
| 保守性         | 高   | 長期的な機能拡張と保守             |
| 拡張性         | 高   | カスタム機能の追加容易性           |

### 選択肢の比較

| 選択肢                                 | パフォーマンス | 実装コスト | 保守性 | 拡張性 | 総合評価 |
| -------------------------------------- | -------------- | ---------- | ------ | ------ | -------- |
| **選択肢1: tower-lsp + tokio**         | ⭐⭐⭐         | ⭐⭐       | ⭐⭐⭐ | ⭐⭐⭐ | **推奨** |
| 選択肢2: lsp-server + 手動非同期       | ⭐⭐           | ⭐         | ⭐⭐   | ⭐⭐   | 次点     |
| 選択肢3: TypeScript + @vscode/lsp-node | ⭐             | ⭐⭐⭐     | ⭐⭐   | ⭐⭐   | 非推奨   |

### 詳細分析

#### 選択肢1: tower-lsp + tokio + tree-sitter

**アプローチ**: 非同期ファーストのLSPフレームワークとインクリメンタル構文解析の組み合わせ

```rust
#[async_trait]
impl LanguageServer for I18nLspServer {
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // 非同期処理が自然に書ける
        let translations = self.load_translations().await?;
        let completions = self.compute_completions(&params, &translations).await?;
        Ok(Some(completions))
    }
}
```

**トレードオフ**:

- ✅ 非同期I/O処理の効率的な実装
- ✅ Denoでの採用実績（パフォーマンス改善）
- ✅ インクリメンタル解析による高速化
- ❌ tokioエコシステムへの依存
- ❌ バイナリサイズ約10-15MB

#### 選択肢2: lsp-server + 手動非同期実装

**アプローチ**: 同期的なLSPフレームワークに手動で非同期処理を追加

```rust
fn handle_completion(params: CompletionParams) -> Result<CompletionResponse> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // 非同期処理を同期的にラップ
    })
}
```

**トレードオフ**:

- ✅ より細かい制御が可能
- ❌ 実装の複雑化
- ❌ 保守性の低下
- ❌ エラーハンドリングの困難さ

#### 選択肢3: TypeScript + @vscode/lsp-node

**アプローチ**: TypeScriptでの実装によるプロトタイピング

```typescript
// TypeScriptでの実装例
connection.onCompletion(
  (_textDocumentPosition: TextDocumentPositionParams): CompletionItem[] => {
    // JavaScript/TypeScriptネイティブの実装
    return getI18nCompletions();
  },
);
```

**トレードオフ**:

- ✅ 開発速度が速い
- ✅ 既存のTypeScriptエコシステムを活用可能
- ❌ パフォーマンスの限界
- ❌ 大規模プロジェクトでのスケーラビリティ問題

## 決定

**選択肢1: tower-lsp + tokio + tree-sitter** を採用する。

### 決定理由

1. **非同期処理の必要性**: i18n LSPは多数のファイルI/O操作を含むため、非同期処理が必須
2. **実績**: Denoプロジェクトでの採用により、劇的なパフォーマンス改善が実証済み
3. **開発効率**: 非同期処理、エラーハンドリング、ミドルウェア機能が組み込み済み
4. **拡張性**: カスタム通知/リクエストの実装が容易

### 実装方針

```toml
[dependencies]
# LSPフレームワーク
tower-lsp = "0.20"
tokio = { version = "1.35", features = ["full"] }

# 構文解析
tree-sitter = "0.20"
tree-sitter-javascript = "0.20"
tree-sitter-typescript = "0.20"

# その他
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dashmap = "5.5"  # 並行アクセス可能なHashMap
```

### 実装への影響

```rust
// ビジネスロジックをLSP実装から分離
pub trait I18nAnalyzer {
    async fn analyze_keys(&self, uri: &Url) -> Result<Vec<TranslationKey>>;
    async fn validate_translations(&self) -> Result<Vec<Diagnostic>>;
}

// LSP固有の実装を抽象化
pub struct I18nLspServer<A: I18nAnalyzer> {
    analyzer: A,
    // LSP固有の状態
}
```

## 結果

### ポジティブな結果

- 高速な開発: tower-lspの豊富な機能により開発速度向上
- スケーラビリティ: 大規模プロジェクトでも高性能を維持
- 保守性: 非同期コードが自然に書ける
- 将来性: アクティブに開発されているエコシステム

### ネガティブな結果

- tokioエコシステムへの依存
- 非同期Rustの学習曲線
- バイナリサイズの増加（約10-15MB）

### 測定可能な成功基準

| 指標                               | 現在値 | 目標値 | 測定方法            |
| ---------------------------------- | ------ | ------ | ------------------- |
| 補完レスポンスタイム               | -      | <100ms | LSPクライアントログ |
| 1000キープロジェクトでの初期化時間 | -      | <3秒   | ベンチマーク        |
| メモリ使用量（1000キー）           | -      | <50MB  | プロファイラー      |

## 実装計画

### フェーズ1: 基盤構築

- [ ] tower-lspベースのLSPサーバー骨格実装
- [ ] 基本的なリクエスト/レスポンスハンドリング
- [ ] ログとトレーシングの設定

### フェーズ2: コア機能実装

- [ ] tree-sitterによる構文解析統合
- [ ] 翻訳ファイルのパース（JSON/YAML/TOML）
- [ ] 補完・検証機能の実装

### 完了予定: 2025-07-31

## 今後の検討事項

- WebAssemblyへのコンパイル可能性（ブラウザ内実行）
- より高度なキャッシング戦略
- プラグインシステムの設計

## 参考資料

- [tower-lsp Documentation](https://github.com/tower-lsp/tower-lsp)
- [Deno's migration to tower-lsp](https://github.com/denoland/deno/pull/11966)
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)
