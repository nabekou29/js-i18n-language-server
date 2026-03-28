<script>
  import { _, locale, locales, isLoading } from "svelte-i18n";
  import BasicUsage from "./BasicUsage.svelte";
  import FormatVariants from "./FormatVariants.svelte";
  import ObjectForm from "./ObjectForm.svelte";
  import Interpolation from "./Interpolation.svelte";
  import NestedKeys from "./NestedKeys.svelte";
  import ReactiveUsage from "./ReactiveUsage.svelte";
</script>

{#if $isLoading}
  <p>Loading...</p>
{:else}
  <main>
    <h1>{$_("app.title")}</h1>

    <div class="locale-switcher">
      <label>
        {$_("app.language")}:
        <select bind:value={$locale}>
          {#each $locales as loc}
            <option value={loc}>{loc}</option>
          {/each}
        </select>
      </label>
    </div>

    <section>
      <h2>{$_("app.section.basic")}</h2>
      <BasicUsage />
    </section>

    <section>
      <h2>{$_("app.section.format")}</h2>
      <FormatVariants />
    </section>

    <section>
      <h2>{$_("app.section.object")}</h2>
      <ObjectForm />
    </section>

    <section>
      <h2>{$_("app.section.interpolation")}</h2>
      <Interpolation />
    </section>

    <section>
      <h2>{$_("app.section.nested")}</h2>
      <NestedKeys />
    </section>

    <section>
      <h2>{$_("app.section.reactive")}</h2>
      <ReactiveUsage />
    </section>
  </main>
{/if}

<style>
  main {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
    font-family: system-ui, -apple-system, sans-serif;
  }

  h1 {
    border-bottom: 2px solid #333;
    padding-bottom: 0.5rem;
  }

  section {
    margin: 2rem 0;
    padding: 1rem;
    border: 1px solid #ddd;
    border-radius: 8px;
  }

  h2 {
    margin-top: 0;
    color: #555;
  }

  .locale-switcher {
    margin: 1rem 0;
    padding: 0.5rem;
    background: #f5f5f5;
    border-radius: 4px;
  }

  select {
    padding: 0.25rem 0.5rem;
    font-size: 1rem;
  }
</style>
