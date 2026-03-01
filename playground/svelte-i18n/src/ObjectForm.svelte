<!--
  svelte-i18n のオブジェクト形式
  - $_({ id: 'key' }) : オブジェクトで翻訳キーを指定
  - $_({ id: 'key', values: {...} }) : 値付き
  - $_({ id: 'key', locale: 'en' }) : ロケール明示指定
  - $_({ id: 'key', default: 'fallback' }) : デフォルト値
-->
<script>
  import { _ } from "svelte-i18n";
</script>

<div>
  <h3>Basic object form</h3>
  <p>{$_({ id: "common.hello" })}</p>
  <p>{$_({ id: "home.title" })}</p>

  <h3>With values</h3>
  <p>{$_({ id: "common.welcome", values: { name: "Alice" } })}</p>

  <h3>With locale override</h3>
  <p>{$_({ id: "common.hello", locale: "en" })}</p>
  <p>{$_({ id: "common.hello", locale: "ja" })}</p>

  <h3>With default fallback</h3>
  <p>{$_({ id: "nonexistent.key", default: "Fallback text" })}</p>

  <h3>Combined options</h3>
  <p>
    {$_({
      id: "common.welcome",
      values: { name: "Bob" },
      locale: "en",
    })}
  </p>
</div>
