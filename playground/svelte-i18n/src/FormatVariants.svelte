<!--
  svelte-i18n のフォーマットバリアント
  - $format('key') : 翻訳フォーマッター
  - $json('key') : JSON 生値の取得 (配列やオブジェクト)
  - $date(date, options) : 日付フォーマット (翻訳キーなし)
  - $number(num, options) : 数値フォーマット (翻訳キーなし)
  - $time(date, options) : 時刻フォーマット (翻訳キーなし)
-->
<script>
  import { format, json, date, time, number } from "svelte-i18n";

  const now = new Date();
  const price = 1234.56;
</script>

<div>
  <h3>$format</h3>
  <p>{$format("format.example")}</p>

  <h3>$json (raw JSON values)</h3>
  <p>Colors: {JSON.stringify($json("json_data.colors"))}</p>
  <p>Config: {JSON.stringify($json("json_data.config"))}</p>

  <h3>$date (date formatting)</h3>
  <p>{$date(now, { dateStyle: "full" })}</p>
  <p>{$date(now, { dateStyle: "short" })}</p>

  <h3>$time (time formatting)</h3>
  <p>{$time(now, { timeStyle: "medium" })}</p>
  <p>{$time(now, { timeStyle: "short" })}</p>

  <h3>$number (number formatting)</h3>
  <p>{$number(price, { style: "currency", currency: "USD" })}</p>
  <p>{$number(price, { style: "currency", currency: "JPY" })}</p>
  <p>{$number(0.75, { style: "percent" })}</p>
</div>
