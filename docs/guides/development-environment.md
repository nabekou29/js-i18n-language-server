# 開発環境セットアップガイド

このドキュメントでは、JS i18n Language Serverの開発環境構築手順を説明します。

## 必要なツール

### 基本ツール

| ツール | バージョン | 用途 | インストール方法 |
|-------|----------|------|----------------|
| Rust | 1.78.0+ | プログラミング言語 | [rustup.rs](https://rustup.rs/) |
| mise | 最新 | 開発環境管理 | `curl https://mise.run | sh` |
| Git | 2.25+ | バージョン管理 | OS標準パッケージマネージャー |

### 開発支援ツール

| ツール | 用途 | 設定ファイル |
|-------|------|-------------|
| rustfmt | コードフォーマッター | `rustfmt.toml` |
| clippy | Lintツール | Rust標準 |
| cargo-watch | ファイル監視・自動ビルド | オプション |

## セットアップ手順

### 1. リポジトリのクローン

```bash
git clone https://github.com/nabekou29/js-i18n-language-server
cd js-i18n-language-server
```

### 2. 開発環境の自動セットアップ

miseを使用した自動セットアップ：

```bash
# miseのインストール（未インストールの場合）
curl https://mise.run | sh

# 開発環境のセットアップ
mise install
```

### 3. プロジェクトのビルド

```bash
# デバッグビルド
cargo build

# リリースビルド
cargo build --release

# テストの実行
cargo test
```

## 設定ファイル

### mise.toml

開発環境の統一管理を行います。以下のツールが自動的にインストールされます：

- Rust (指定バージョン)
- 必要な開発ツール

### rustfmt.toml

Rustコードの自動フォーマット設定：

```toml
# プロジェクトで統一されたコードスタイル
edition = "2021"
```

使用方法：

```bash
# ファイルをフォーマット
cargo fmt

# フォーマットチェック（CIで使用）
cargo fmt -- --check
```

### hk.pkl

プロジェクト固有の設定ファイル（用途に応じて編集）

## 開発ワークフロー

### 1. 開発サーバーの起動

```bash
# ログ付きで起動
RUST_LOG=debug cargo run
```

### 2. テストの実行

```bash
# 全テスト実行
cargo test

# 特定のテストのみ
cargo test hover_test

# ログ出力付き
cargo test -- --nocapture
```

### 3. エディタ連携テスト

#### Neovim

`playground/nvim/playground.lua`を使用：

```lua
-- Neovimで以下を実行
:source playground/nvim/playground.lua
```

#### VSCode

1. LSPサーバーをビルド
2. VSCode拡張機能として設定（別途拡張機能プロジェクトが必要）

## トラブルシューティング

### ビルドエラー

```bash
# 依存関係のクリーンアップ
cargo clean

# 依存関係の再取得
cargo update
```

### LSPサーバーが起動しない

1. ログレベルを上げて詳細確認：
   ```bash
   RUST_LOG=trace cargo run
   ```

2. 標準入出力の確認（LSPは標準入出力で通信）

### テストが失敗する

1. 最新のmainブランチを取得
2. `cargo clean && cargo test`を実行

## 推奨される開発環境

### エディタ

- **VS Code**: rust-analyzer拡張機能
- **Neovim**: rust-tools.nvim
- **IntelliJ IDEA**: Rustプラグイン

### デバッグツール

- **lldb/gdb**: Rustデバッガー
- **cargo-expand**: マクロ展開の確認
- **cargo-tree**: 依存関係の可視化

## 次のステップ

1. [アーキテクチャドキュメント](/docs/design/l1-system/architecture.md)を読む
2. [テスト戦略](/docs/adr/ADR-002-test-strategy-and-coverage.md)を理解する
3. [段階的実装アプローチ](/docs/adr/ADR-003-incremental-implementation-approach.md)を確認する