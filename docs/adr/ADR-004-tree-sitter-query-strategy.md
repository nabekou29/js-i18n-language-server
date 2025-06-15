---
title: "ADR-004: tree-sitterクエリベースのi18n解析戦略"
type: "Architecture Decision Record"
adr_number: "004"
created_date: "2025-06-15"
author: "@nabekou29"
status: "proposed"
category: "architecture"
impact_level: "high"
---

# ADR-004: tree-sitterクエリベースのi18n解析戦略

## ステータス

**現在のステータス:** 提案中

**決定日:** 2025-06-15

**関連する決定:**

- 関連する決定: [ADR-001 技術スタック選定](/docs/adr/ADR-001-technology-stack-selection.md)
- この決定により置き換えられる: なし

---

## コンテキスト

### 背景・状況

i18n Language Serverでは、JavaScript/TypeScriptコード内の翻訳キー使用箇所を正確に抽出する必要があります。対象となるパターンは多様で、単純な文字列マッチングでは限界があります。

**解析対象パターン:**

- 関数呼び出し: `t('key')`, `i18n.t('key')`, `$t('key')`
- チェーン呼び出し: `useTranslation().t('key')`
- JSXコンポーネント: `<Trans i18nKey="key" />`
- 動的キー: `t(\`prefix.\${variable}\`)`
- TypeScript型情報: `t<'key'>('key')`

**解決すべき問題:**

- 文字列マッチングでは構文的に不正確
- 複雑なネスト構造や式の解析困難
- ライブラリ固有パターンへの対応
- TypeScript/JSXの言語固有構文への対応

### 制約条件

**技術的制約:**

- tree-sitter v0.20系の機能制限
- パフォーマンス要件（リアルタイム解析）
- メモリ効率性の要求

**言語対応制約:**

- JavaScript, TypeScript, JSX, TSXの全対応
- 各言語の構文差異への対応
- 将来的な言語バージョンアップへの追従

---

## 検討した選択肢

### 選択肢A: 正規表現ベースの解析

**概要:**
正規表現を使って文字列レベルでi18n関数呼び出しを検出。

**メリット:**

- 実装が簡単
- パフォーマンスが高い
- ライブラリ依存なし

**デメリット:**

- 構文的に不正確（コメント内も検出）
- 複雑なネスト構造に対応困難
- JSXの属性解析が困難

**実装コスト:** 低  
**技術的複雑さ:** 低  
**リスク:** 高（誤検出・見落とし）

### 選択肢B: 簡単なASTウォーク

**概要:**
tree-sitterでASTを生成し、ノードを順次走査してパターンマッチング。

**メリット:**

- 構文的に正確
- 実装が理解しやすい
- デバッグが容易

**デメリット:**

- 全ノードを走査するためパフォーマンス低下
- 複雑なパターンの記述が困難
- 言語別の差異対応が煩雑

**実装コスト:** 中  
**技術的複雑さ:** 中  
**リスク:** 中（パフォーマンス）

### 選択肢C: tree-sitterクエリ言語使用

**概要:**
tree-sitterのクエリ言語を使用して、宣言的にパターンを定義。

**メリット:**

- 高精度の構文解析
- 宣言的で保守しやすいパターン定義
- パフォーマンスが最適化されている
- 複雑なパターンも簡潔に表現

**デメリット:**

- クエリ言語の学習コスト
- デバッグが困難な場合がある
- クエリの最適化が必要

**実装コスト:** 中  
**技術的複雑さ:** 高  
**リスク:** 低

### 比較表

| 観点               | 選択肢A | 選択肢B | 選択肢C | 重要度 |
| ------------------ | ------- | ------- | ------- | ------ |
| **精度**           | 低      | 高      | 最高    | 高     |
| **パフォーマンス** | 最高    | 中      | 高      | 高     |
| **保守性**         | 低      | 中      | 高      | 高     |
| **拡張性**         | 低      | 中      | 高      | 中     |
| **実装コスト**     | 低      | 中      | 中      | 中     |

---

## 決定

### 採用する選択肢

**選択:** 選択肢C（tree-sitterクエリ言語使用）

**決定理由:**

1. **実績のある手法**: js-i18n.nvimで実証済みの高精度解析手法
2. **スコープ管理**: useTranslation()等のスコープを正確に追跡可能
3. **ライブラリ特化**: 各i18nライブラリの特性に最適化されたクエリ

### 決定の詳細

**実装方針:**

```rust
// js-i18n.nvimベースのライブラリ別クエリ管理
struct TranslationQueries {
    // i18next/react-i18next
    i18next_trans_f_call: Query,     // trans_f_call: 文言取得関数の呼び出し
    i18next_trans_f: Query,          // trans_f: 文言取得関数の定義
    react_i18next_trans_component: Query, // Transコンポーネント
    
    // next-intl
    next_intl_use_translations: Query,
    next_intl_trans_f_call: Query,
    
    // 共通
    import_statements: Query,
}

// 実際のクエリ例（js-i18n.nvimから移植）
const I18NEXT_TRANS_F_CALL_QUERY: &str = r#"
; trans_f_call（文言取得関数呼び出し）の検出
(call_expression
  function: [
    (identifier)
    (member_expression)
  ] @i18n.t_func_name
    arguments: (arguments
      (string
        (string_fragment) @i18n.key
      ) @i18n.key_arg
    )
) @i18n.call_t
"#;

const I18NEXT_TRANS_F_QUERY: &str = r#"
; trans_f（文言取得関数定義）useTranslation()フックの検出
(call_expression
  function: (identifier) @i18n.hook_name (#eq? @i18n.hook_name "useTranslation")
  arguments: (arguments
    (string
      (string_fragment) @i18n.namespace
    ) @i18n.namespace_arg
    (object
      (pair
        key: (property_identifier) @i18n.keyPrefix_key (#eq? @i18n.keyPrefix_key "keyPrefix")
        value: (string
          (string_fragment) @i18n.key_prefix
        ) @i18n.key_prefix_arg
      )
    ) @i18n.options
  )
) @i18n.get_t
"#;
```

**設計原則:**

- js-i18n.nvimで実証済みのクエリパターンを基盤とする
- ライブラリ別の特化クエリによる高精度解析
- スコープ管理による正確なキープレフィックス適用
- 統一されたキャプチャ名（@i18n.key, @i18n.t_func_name等）

---

## 結果・影響

### ポジティブな影響

**短期的効果:**

- 解析精度の大幅向上（90%以上の精度目標）
- 複雑なJavaScript/TypeScript構文への対応
- JSXコンポーネントの正確な解析

**長期的効果:**

- 新しいi18nライブラリパターンへの容易な対応
- TypeScript型情報との統合可能性
- 他のコード解析機能への応用

### ネガティブな影響・トレードオフ

**受け入れるコスト:**

- tree-sitterクエリ言語の学習コスト
- クエリのデバッグ・最適化に必要な時間
- 初期実装の複雑度増加

**リスク・課題:**

- クエリのパフォーマンスチューニングが必要
- 軽減策：ベンチマークテストによる継続的な最適化

---

## 実装計画

### フェーズ1: 基本クエリ実装

**実施内容:**

- [ ] JavaScript基本パターンのクエリ実装
- [ ] TypeScriptクエリの実装
- [ ] JSX/TSXクエリの実装
- [ ] クエリ実行エンジンの実装

**成功基準:**

- 基本的なi18n関数呼び出しの90%以上検出
- パフォーマンステストクリア

### フェーズ2: 高度なパターン対応

**実施内容:**

- [ ] 複雑なネスト構造への対応
- [ ] 動的キー生成の部分対応
- [ ] ライブラリ固有パターンの追加
- [ ] エラーハンドリングの強化

**成功基準:**

- 実プロジェクトでの検証テスト成功
- 誤検出率5%以下

---

## 監視・測定

### 成功指標

| 指標           | 現在値 | 目標値   | 測定方法             |
| -------------- | ------ | -------- | -------------------- |
| 検出精度       | -      | >90%     | テストケース評価     |
| 誤検出率       | -      | <5%      | マニュアル検証       |
| 解析速度       | -      | <50ms/file | ベンチマーク測定     |

### 代表的なテストケース（js-i18n.nvimベース）

```javascript
// useTranslation()によるスコープ定義
const { t } = useTranslation('common', { keyPrefix: 'prefix-1' })

// 基本的なt()呼び出し
t('no-prefix-key-1')           // → 'no-prefix-key-1'
t('prefix-1-key-1')            // → 'prefix-1.prefix-1-key-1'

// JSXでのスコープ変更
<Translation ns="user" keyPrefix="profile">
  {(t) => (
    <div>
      {t('name')}                // → 'user:profile.name'
      {t('email')}               // → 'user:profile.email'
    </div>
  )}
</Translation>

// Transコンポーネント
<Trans i18nKey="welcome.message" />
<Trans 
  i18nKey="user.greeting" 
  t={t}                         // スコープのt関数を使用
/>

// ネストされたスコープ
function Component() {
  const { t: tl } = useTranslation('local', { keyPrefix: 'tsl-prefix-1' })
  return <div>{tl('tsl-prefix-1-key-1')}</div>  // → 'local:tsl-prefix-1.tsl-prefix-1-key-1'
}
```

**期待される解析結果:**
- スコープ情報が正確に適用される
- キープレフィックスとネームスペースが組み合わされる
- ネストされたスコープが正しく管理される

---

## 参考資料

### 技術資料

- [tree-sitter Query Language Documentation](https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax)
- [tree-sitter JavaScript Grammar](https://github.com/tree-sitter/tree-sitter-javascript)
- [tree-sitter TypeScript Grammar](https://github.com/tree-sitter/tree-sitter-typescript)
- [js-i18n.nvim](https://github.com/nabekou29/js-i18n.nvim) - 実証済みのi18n解析実装

### 関連ADR

- [ADR-001: 技術スタック選定](/docs/adr/ADR-001-technology-stack-selection.md)
- [ADR-003: ファイルID管理戦略](/docs/adr/ADR-003-file-id-management-strategy.md)