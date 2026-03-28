<!--
  svelte-i18n の補間と ICU MessageFormat
  - {name} : 変数埋め込み
  - {count, plural, ...} : 複数形
  - {gender, select, ...} : 条件分岐
-->
<script>
  import { _ } from "svelte-i18n";

  let count = 0;
  let gender = "other";
  let role = "admin";
</script>

<div>
  <h3>Simple interpolation</h3>
  <p>{$_("common.welcome", { values: { name: "Charlie" } })}</p>

  <h3>Plural (ICU MessageFormat)</h3>
  <div>
    <label>
      Count:
      <input type="number" bind:value={count} min="0" max="99" />
    </label>
    <p>{$_("items.count", { values: { count } })}</p>
    <p>{$_("items.selected", { values: { count } })}</p>
  </div>

  <h3>Select (ICU MessageFormat)</h3>
  <div>
    <label>
      Gender:
      <select bind:value={gender}>
        <option value="male">Male</option>
        <option value="female">Female</option>
        <option value="other">Other</option>
      </select>
    </label>
    <p>{$_("user.greeting", { values: { gender, name: "Dana" } })}</p>
  </div>

  <div>
    <label>
      Role:
      <select bind:value={role}>
        <option value="admin">Admin</option>
        <option value="editor">Editor</option>
        <option value="viewer">Viewer</option>
      </select>
    </label>
    <p>{$_("user.role", { values: { role } })}</p>
  </div>
</div>

<style>
  input[type="number"] {
    width: 60px;
    padding: 0.25rem;
  }

  select {
    padding: 0.25rem;
  }

  label {
    display: inline-flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.5rem;
  }
</style>
