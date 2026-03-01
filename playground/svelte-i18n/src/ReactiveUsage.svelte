<!--
  svelte-i18n のリアクティブ使用パターン
  - $: msg = $_('key') : リアクティブ宣言
  - {#if ...}{$_('key')}{/if} : 条件分岐
  - {#each ...}{$_('key')}{/each} : ループ
-->
<script>
  import { _ } from "svelte-i18n";

  let count = $state(0);
  let isLoggedIn = $state(true);
  let isAdmin = $state(false);
  let userName = $state("Eve");

  const fruits = ["apple", "banana", "cherry"];

  // $derived で翻訳結果を計算
  const counterMessage = $derived(
    $_("reactive.counter", { values: { count } }),
  );
  const statusMessage = $derived(
    $_("reactive.status", { values: { online: String(isLoggedIn) } }),
  );
</script>

<div>
  <h3>Reactive declarations ($derived)</h3>
  <div>
    <button onclick={() => count++}>+</button>
    <button onclick={() => count--}>-</button>
    <p>{counterMessage}</p>
  </div>

  <h3>Conditional rendering</h3>
  <div>
    <label>
      <input type="checkbox" bind:checked={isLoggedIn} />
      Logged in
    </label>

    {#if isLoggedIn}
      <p>{$_("conditions.logged_in", { values: { name: userName } })}</p>
    {:else}
      <p>{$_("conditions.logged_out")}</p>
    {/if}
  </div>

  <div>
    <label>
      <input type="checkbox" bind:checked={isAdmin} />
      Admin
    </label>

    {#if isAdmin}
      <p>{$_("conditions.admin")}</p>
    {:else}
      <p>{$_("conditions.user")}</p>
    {/if}
  </div>

  <h3>Loop rendering</h3>
  <ul>
    {#each fruits as fruit}
      <li>{$_("loop.fruit", { values: { name: fruit } })}</li>
    {/each}
  </ul>

  <h3>Reactive status</h3>
  <p>{statusMessage}</p>
</div>

<style>
  button {
    padding: 0.25rem 0.75rem;
    margin-right: 0.5rem;
    font-size: 1rem;
  }

  label {
    display: inline-flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.5rem;
  }
</style>
