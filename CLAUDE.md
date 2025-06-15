# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 行動規範

### ドキュメント作成時

- 不明瞭な点はユーザーに確認すること。ユーザーの意図を確認するために、質問をすること。
- ユーザーの指示に対して、表面的な理解をせず深く理解すること。
- ドキュメントの作成方法は @docs/README.md を参照すること。
- ドキュメントの作成には必ずテンプレートを使用すること。
- `created_date` や `modified_date` を更新する場合は time:get_current_time から現在時間を取得すること。

## 用語

| 用語                 | 説明                                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------------- |
| LSP                  | Language Server Protocolの略。エディタとサーバー間の通信プロトコル。                              |
| i18n                 | 国際化（Internationalization）の略。多言語対応の実装。                                            |
| tree-sitter          | 構文解析のためのライブラリ。コードの構造を解析する。                                              |
| translation          | 文言。                                                                                            |
| translation_resource | 文言リソース。文言キーと文言を定義する。基本的には JSON で表現される。                            |
| trans_key, key       | 文言キー。                                                                                        |
| trans_f              | 文言取得関数。 `i18n.t`, `t`, `Trans`。                                                           |
| trans_f_call         | 文言取得関数の呼び出し。`i18n.t('key')`, `t('key', { ... })`, `<Trans key='key' />`               |
| reference            | 文言取得関数の呼び出しの参照。呼び出しがどこでどのような引数で行われているか。                    |
| namespace            | 名前空間。i18next などにある、文言のグループ。                                                    |
| library              | ライブラリ。i18n関連の機能を提供する JS のライブラリ。i18next, react-i18next, next-i18next など。 |
| file_id              | ファイルパスを数値で表現したID。メモリ効率化のために使用。                                        |
| indexer              | 翻訳キーの参照を管理するコアシステム。I18nIndexer。                                               |
| incremental_update   | 増分更新。ファイルの変更部分のみを再解析する仕組み。                                              |
| tree_sitter_query    | tree-sitterのクエリ言語。ASTパターンを記述してノードを抽出。                                     |
| query_set            | 特定の言語用のtree-sitterクエリ集合。関数呼び出し、JSX、import文等。                             |

## 開発コマンド

### ビルド・実行

```bash
# プロジェクトのビルド
mise run build

# バイナリをビルドしてインストール
mise run build-install
```

### 品質チェック

```bash
# リントチェック
mise run lint

# リントの自動修正
mise run lint-fix

# フォーマットチェック
mise run format

# フォーマット実行
cargo fmt --all

# テスト実行
cargo test

# ドキュメント生成
cargo doc --open
```

## アーキテクチャ概要

このプロジェクトは **JavaScript/TypeScript向けi18n（国際化）Language Server** です。

### 主要コンポーネント

- **Language Server**: tower-lspを使用したLSPサーバー実装
- **Parser**: tree-sitterベースの構文解析
- **i18n Analyzer**: 翻訳キーの抽出と分析
- **Translation Cache**: 翻訳ファイルのメモリキャッシュ（dashmap使用）
- **Validator**: 翻訳キーの検証と整合性チェック

### 主要機能

- 翻訳キーの自動補完
- 存在しない翻訳キーの診断
- ホバー時の翻訳内容表示
- 翻訳キーの定義ジャンプ
- 未使用翻訳キーの検出

## 重要な制約事項

### 厳格なコーディング要件

- **unsafe_code = "forbid"** - unsafeコードは絶対禁止
- **panic = "deny"** - panic!の使用禁止
- **unwrap_used = "deny"** - unwrap()の使用禁止
- **expect_used = "deny"** - expect()の使用禁止
- **missing_docs = "deny"** - 文書化必須（パブリック・プライベート問わず）
- **print_stdout/stderr = "deny"** - 標準出力への直接出力禁止（tracingを使用）

### エラーハンドリング

- `Result<T, E>`型の適切な使用
- `?`演算子での伝播
- `match`文での明示的な処理

### ログ出力

```rust
// tracing crateを使用
tracing::info!("正常な処理");
tracing::warn!("警告メッセージ");
tracing::error!("エラーメッセージ");
```

## 対応i18nライブラリ

- i18next/react-i18next/next-i18next
- next-intl
- vue-i18n
- その他の一般的なi18nライブラリ

## プロジェクト構造

```
src/
├── lib.rs        # ライブラリのメイン実装（Backend構造体）
├── main.rs       # LSPサーバーのエントリーポイント
└── ...           # その他のモジュール（開発中）

docs/
├── prd.md        # プロダクト要件定義
├── design/       # システム設計文書
└── adr/          # アーキテクチャ決定記録
```

## 開発時の注意点

1. **文書化**: すべてのパブリック・プライベート要素にdocコメントが必要
2. **エラーハンドリング**: panicしない設計、適切なResult型の使用
3. **パフォーマンス**: 補完レスポンス<100ms、初期化<3秒を目標
4. **非同期処理**: tokioを使用した効率的な処理
5. **テスト**: 高いテストカバレッジ（80%以上目標）
