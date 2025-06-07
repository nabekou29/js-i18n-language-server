# ADR-003: LSP技術検証と学習アプローチ

- **Status**: Accepted
- **Date**: 2025-06-08
- **Author**: @nabekou29
- **Deciders**: @nabekou29

## Context and Problem Statement

JS i18n Language Serverの開発に先立ち、LSPプロトコルの理解と技術検証が必要である。tower-lspフレームワークとRustでのLSP実装パターンを習得するため、チュートリアル的な実装を通じた学習アプローチを検討する。

## Decision Drivers

- **技術理解**: LSPプロトコルとtower-lspフレームワークの習得
- **実装パターンの確立**: Rustでの効果的なLSP実装方法の検証
- **リスク軽減**: 本格実装前に技術的な課題を発見・解決
- **知識共有**: チームメンバーや将来の開発者への学習資料

## Considered Options

### Option 1: 完全機能の一括実装
全てのi18n機能を最初から実装する。

**Pros:**
- 最終的な設計を最初から考慮できる
- 手戻りが少ない

**Cons:**
- 実装が複雑になり、デバッグが困難
- 動作確認までに時間がかかる
- LSPの基礎理解が不十分なまま進む可能性

### Option 2: 技術検証ファースト
技術検証用のチュートリアル実装を作成し、得られた知見を基に本格実装を開始する。

**Pros:**
- LSPプロトコルとtower-lspの深い理解を獲得
- 技術的な課題を早期に発見
- 実装パターンとベストプラクティスの確立
- 学習資料として将来も活用可能

**Cons:**
- 本格実装の開始が遅れる
- チュートリアル実装と本格実装で一部重複作業が発生

## Decision Outcome

**選択: Option 2 - 技術検証ファースト**

### 学習・検証フェーズ

#### 技術検証実装（完了）
**目的**: LSPプロトコルとtower-lspの理解

**実装内容**:
- 基本的なLSPサーバー構造
- 診断機能（TODO/FIXME検出）
- ホバー機能（固定メッセージ）
- 補完機能（固定候補）
- テキスト同期
- 統合テスト環境

**成果**:
- LSPプロトコルの基本的な理解
- tower-lspフレームワークの使用方法習得
- Rustでの非同期処理パターン確立
- テスト戦略の検証

### 本格実装フェーズ（今後）

技術検証で得られた知見を基に、JS i18n Language Serverの本格実装を開始する。技術検証実装は学習資料として保持し、本格実装は新規プロジェクトとして開始する。

### 技術検証から得られた知見

- tower-lspを使用したLSPサーバーの基本構造
- 効果的なテスト戦略（統合テスト中心）
- Rustでの非同期処理とエラーハンドリング
- LSPクライアントとの通信パターン

## Consequences

### Positive

- **技術理解**: LSPプロトコルとtower-lspの深い理解を獲得
- **リスク軽減**: 本格実装前に技術的な課題を発見・解決
- **学習資料**: 将来の開発者向けの実践的な学習資料
- **実装パターン**: Rustでの効果的なLSP実装パターンを確立

### Negative

- **開発期間**: 技術検証に時間を要する
- **重複作業**: 一部の基本実装が重複する可能性

### Risks and Mitigation

- **リスク**: 技術検証に時間をかけすぎる
  - **対策**: 明確な検証項目と期限を設定

- **リスク**: 技術検証と本格実装の乖離
  - **対策**: 検証項目を本格実装に必要な要素に絞る

## References

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [tower-lsp Documentation](https://github.com/ebkalderon/tower-lsp)
- 既存の設計文書: `docs/design/l1-system/architecture.md`