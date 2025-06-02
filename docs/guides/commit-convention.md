# コミットメッセージ規約ガイド

このプロジェクトでは、すべてのコミットメッセージに[Conventional Commits](https://www.conventionalcommits.org/)仕様を採用しています。

## 基本フォーマット

```
<type>(<scope>): <subject>

[optional body]

[optional footer(s)]
```

**重要**: コミットメッセージは**英語**で記述します。

## Type（種別）

| Type       | 説明                                           | 例                                      |
| ---------- | ---------------------------------------------- | --------------------------------------- |
| `feat`     | 新機能の追加                                   | `feat: add hover support for i18n keys` |
| `fix`      | バグ修正                                       | `fix: correct completion position`      |
| `docs`     | ドキュメントのみの変更                         | `docs: update README installation`      |
| `style`    | コードの意味に影響しない変更（フォーマット等） | `style: format with rustfmt`            |
| `refactor` | バグ修正や機能追加を伴わないコード変更         | `refactor: extract parser module`       |
| `perf`     | パフォーマンス改善                             | `perf: cache parsed translation files`  |
| `test`     | テストの追加・修正                             | `test: add parser edge cases`           |
| `build`    | ビルドシステムや外部依存の変更                 | `build: update tower-lsp to 0.20`       |
| `ci`       | CI設定の変更                                   | `ci: add rust clippy check`             |
| `chore`    | その他の変更（srcやtestに影響しない）          | `chore: update gitignore`               |
| `revert`   | 以前のコミットの取り消し                       | `revert: feat: add hover support`       |

## Scope（スコープ）

このプロジェクトでよく使用されるスコープ：

- `lsp` - LSPサーバーのコア機能
- `parser` - 構文解析・AST関連
- `analyzer` - i18nキーの分析・抽出
- `completion` - コード補完機能
- `validation` - 検証・診断機能
- `hover` - ホバー情報機能
- `config` - 設定処理
- `test` - テスト基盤
- `docs` - ドキュメント
- `ci` - 継続的インテグレーション

## 良いコミットメッセージの書き方

### 件名（Subject）のルール

1. **命令形を使用する**

   - ✅ `fix: resolve memory leak`
   - ❌ `fix: resolved memory leak`
   - ❌ `fix: resolves memory leak`

2. **type後の最初の文字は小文字**

   - ✅ `feat: add new parser`
   - ❌ `feat: Add new parser`

3. **末尾にピリオドを付けない**

   - ✅ `docs: update installation guide`
   - ❌ `docs: update installation guide.`

4. **72文字以内に収める**
   - 簡潔かつ説明的に

### 本文（Body）のガイドライン

本文では以下を説明します：

- **なぜ**この変更を行ったか
- **何の**問題を解決するか
- システムに**どのような**影響があるか

例：

```
fix(parser): handle multi-byte characters in key extraction

Previous implementation used byte offsets which caused incorrect
positioning when the source contained emoji or other UTF-8
multi-byte characters. This fix converts to character-based
indexing before processing.

Fixes #123
```

### コミットの粒度

1. **1コミット1目的**

   - 機能追加と修正を混ぜない
   - 無関係なリファクタリングを含めない

2. **原子性を保つ**

   - 各コミットは自己完結的であるべき
   - 各コミット後もプロジェクトはビルド・テストが通る状態

3. **論理的な単位で分割**

   ```
   # 良い例 - 目的別に分離
   feat(completion): add basic i18n key completion
   test(completion): add completion provider tests
   docs: document completion feature usage

   # 悪い例 - 複数の目的が混在
   feat(completion): add completion with tests and docs
   ```

## Breaking Changes（破壊的変更）

**重要**: Breaking Changes（`!`マーク）は、最初の安定版リリース（v1.0.0）以降にのみ使用します。

```
feat(parser)!: change AST structure for better performance

BREAKING CHANGE: The AST node structure has changed.
Consumers need to update their visitor implementations.
```

## 具体例

### 機能追加

```
feat(analyzer): support Vue i18n directive detection

Add detection for Vue's v-t directive and $t() method calls.
This enables completion and validation in Vue templates.
```

### バグ修正

```
fix(lsp): prevent duplicate diagnostic reports

Diagnostics were being reported twice due to both
textDocument/didChange and didSave handlers running
validation. Now only didChange triggers validation.
```

### パフォーマンス改善

```
perf(parser): implement incremental parsing

Use tree-sitter's incremental parsing API to only
reparse changed portions of the document. Reduces
parse time by 60% on average for typical edits.
```

### リファクタリング

```
refactor(analyzer): extract key extraction logic

Move key extraction from the main analyzer into a
dedicated module to improve testability and enable
reuse in other components.
```

## 避けるべき間違い

1. **変更の混在**

   ```
   # 悪い例
   fix: fix completion bug and add new parser feature

   # 良い例 - 2つのコミットに分割
   fix(completion): handle empty key scenarios
   feat(parser): add JSX support
   ```

2. **曖昧な説明**

   ```
   # 悪い例
   fix: fix bug
   update: update code

   # 良い例
   fix(validation): handle missing translation files gracefully
   refactor(config): consolidate configuration loading logic
   ```

3. **間違ったtypeの使用**

   ```
   # 悪い例 - 新機能なのに'fix'を使用
   fix: add hover support

   # 良い例
   feat: add hover support
   ```

## ツールと自動化

### Commitizen

対話形式でコミットメッセージを作成：

```bash
npm install -g commitizen
npm install -g cz-conventional-changelog
echo '{ "path": "cz-conventional-changelog" }' > ~/.czrc

# 使用方法：
git cz
```

### コミットメッセージテンプレート

Gitコミットテンプレートの設定：

```bash
git config commit.template .gitmessage
```

## 参考資料

- [Conventional Commits Specification](https://www.conventionalcommits.org/)
- [Angular Commit Message Guidelines](https://github.com/angular/angular/blob/main/CONTRIBUTING.md#-commit-message-format)
- [How to Write a Git Commit Message](https://chris.beams.io/posts/git-commit/)

