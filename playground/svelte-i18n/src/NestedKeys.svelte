<!--
  svelte-i18n のネストキー
  - ドット記法でネストされた翻訳キーにアクセス
  - 'nested.deep.key' → { nested: { deep: { key: "..." } } }
-->
<script>
  import { _ } from "svelte-i18n";
</script>

<div>
  <h3>Two-level nesting</h3>
  <p>{$_("common.hello")}</p>
  <p>{$_("common.goodbye")}</p>
  <p>{$_("home.title")}</p>
  <p>{$_("home.description")}</p>

  <h3>Three-level nesting</h3>
  <p>{$_("common.buttons.save")}</p>
  <p>{$_("common.buttons.cancel")}</p>
  <p>{$_("common.buttons.delete")}</p>
  <p>{$_("home.hero.heading")}</p>
  <p>{$_("home.hero.subheading")}</p>

  <h3>Deep nesting</h3>
  <p>{$_("nested.deep.key")}</p>
  <p>{$_("nested.deep.another")}</p>
  <p>{$_("nested.level1.level2.level3")}</p>
</div>
