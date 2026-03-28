<!--
  svelte-i18n の基本的な使い方
  - $_('key') : メインの翻訳関数 (underscore ストア)
  - $t('key') : $_ のエイリアス
  - $format('key') : 明示的なフォーマッター名
  - テンプレートと script 両方で使用可能
-->
<script>
  import { _, t, format } from "svelte-i18n";

  // script ブロック内での使用
  const hello = $_("common.hello");
  const goodbye = $t("common.goodbye");
</script>

<div>
  <h3>$_ (underscore store)</h3>
  <p>{$_("common.hello")}</p>
  <p>{$_("common.goodbye")}</p>

  <h3>$t (alias)</h3>
  <p>{$t("common.hello")}</p>
  <p>{$t("home.title")}</p>

  <h3>$format (explicit formatter)</h3>
  <p>{$format("format.example")}</p>

  <h3>script block usage</h3>
  <p>{hello}</p>
  <p>{goodbye}</p>

  <h3>With values (interpolation)</h3>
  <p>{$_("common.welcome", { values: { name: "World" } })}</p>
</div>
