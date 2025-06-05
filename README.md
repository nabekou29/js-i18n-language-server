# Rust LSP チュートリアル

このプロジェクトは、Rust + tower-lsp を使用した基本的なLSPサーバーの実装例です。

## 機能

- **初期化とシャットダウン**: LSPプロトコルの基本的なライフサイクル
- **ホバー**: カーソル位置にホバーメッセージを表示
- **補完**: `hello` と `world` の簡単な補完候補
- **診断**: `TODO` と `FIXME` コメントを検出して診断メッセージを表示

## ビルド

```bash
cargo build
```

## 実行方法

### VSCodeでの動作確認

1. VSCodeの拡張機能「Generic LSP Client」をインストール
2. `.vscode/settings.json` を作成：

```json
{
  "genericLanguageServer.servers": {
    "rust-lsp-tutorial": {
      "command": ["target/debug/rust-lsp-tutorial"],
      "rootPatterns": ["Cargo.toml"],
      "filePattern": "**/*.txt"
    }
  }
}
```

3. VSCodeを再起動して `test.txt` を開く

### Neovimでの動作確認

1. Neovimの設定ファイルに以下を追加：

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "text",
  callback = function()
    vim.lsp.start({
      name = "rust-lsp-tutorial",
      cmd = { "path/to/target/debug/rust-lsp-tutorial" },
      root_dir = vim.fn.getcwd(),
    })
  end,
})
```

2. Neovimで `test.txt` を開く

## 動作確認

- `test.txt` を開いて、TODOやFIXMEが診断として表示されることを確認
- テキストにカーソルを合わせるとホバーメッセージが表示される
- 入力時に補完候補（hello, world）が表示される

## 次のステップ

このチュートリアルで学んだことを活かして、実際のi18n LSPサーバーの実装に進むことができます。主な拡張ポイント：

- tree-sitterを使用した構文解析
- 翻訳ファイルの読み込みとキャッシュ
- 実際の翻訳キーに基づく補完機能
- より高度な診断機能