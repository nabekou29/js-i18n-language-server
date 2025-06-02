---
title: "[コンポーネント名] Component Design Document"
type: "Component Design Document"
hierarchy_level: "L3-Component"
parent_doc: "[L2 Service Doc へのリンク]"
version: "v1.0"
created_date: "YYYY-MM-DD"
last_updated: "YYYY-MM-DD"
author: "[実装者名]"
status: "draft" # draft | review | approved
---

## 1. コンポーネント概要

**コンポーネント名:** [Component Name]  
**責務:** [このコンポーネントの具体的な責務を1-2行で]  
**使用場所:** [どこで使われるか]

## 2. インターフェース設計

```typescript
// 公開インターフェース
export interface IUserValidator {
  validate(user: CreateUserDto): ValidationResult;
}

export interface ValidationResult {
  isValid: boolean;
  errors?: string[];
}
```

## 3. 実装詳細

### 3.1 主要メソッドの実装

```typescript
export class UserValidator implements IUserValidator {
  validate(user: CreateUserDto): ValidationResult {
    const errors: string[] = [];

    // メールアドレスの検証
    if (!this.isValidEmail(user.email)) {
      errors.push("Invalid email format");
    }

    // 名前の検証
    if (!user.name || user.name.trim().length === 0) {
      errors.push("Name is required");
    }

    return {
      isValid: errors.length === 0,
      errors: errors.length > 0 ? errors : undefined,
    };
  }

  private isValidEmail(email: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
  }
}
```

### 3.2 設定・依存関係

```typescript
// 必要な設定
interface ValidatorConfig {
  maxNameLength?: number; // デフォルト: 100
  allowedDomains?: string[]; // デフォルト: すべて許可
}
```

## 4. エラー処理

- **想定されるエラー:** [どんなエラーが発生しうるか]
- **エラー時の動作:** [エラー発生時の処理]

## 5. テスト方針

### 5.1 主要テストケース

```typescript
describe("UserValidator", () => {
  it("should validate valid user", () => {
    const user = { email: "test@example.com", name: "Test User" };
    const result = validator.validate(user);
    expect(result.isValid).toBe(true);
  });

  it("should reject invalid email", () => {
    const user = { email: "invalid-email", name: "Test User" };
    const result = validator.validate(user);
    expect(result.isValid).toBe(false);
    expect(result.errors).toContain("Invalid email format");
  });
});
```

### 5.2 テストカバレッジ目標

- 単体テスト: 80%以上
- 主要な境界値をカバー

## 6. 使用例

```typescript
// 使用例
const validator = new UserValidator();
const result = validator.validate({
  email: "user@example.com",
  name: "John Doe",
});

if (!result.isValid) {
  console.error("Validation errors:", result.errors);
}
```

## 7. 今後の改善案

- [将来的に追加したい機能]
- [パフォーマンス改善の余地]
- [より汎用的にする方法]
