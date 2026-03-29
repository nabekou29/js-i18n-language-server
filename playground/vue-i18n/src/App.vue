<script setup>
import { useI18n } from "vue-i18n";
import CompositionApi from "./CompositionApi.vue";
import OptionsApi from "./OptionsApi.vue";
import TemplatePatterns from "./TemplatePatterns.vue";
import ObjectAccess from "./ObjectAccess.vue";

const { t, locale, availableLocales } = useI18n();
</script>

<template>
  <main>
    <h1>{{ $t("app.title") }}</h1>

    <div class="locale-switcher">
      <label>
        {{ $t("app.language") }}:
        <select v-model="locale">
          <option v-for="loc in availableLocales" :key="loc" :value="loc">
            {{ loc }}
          </option>
        </select>
      </label>
    </div>

    <section>
      <h2>Composition API</h2>
      <CompositionApi />
    </section>

    <section>
      <h2>Options API</h2>
      <OptionsApi />
    </section>

    <section>
      <h2>Template Patterns</h2>
      <TemplatePatterns />
    </section>

    <section>
      <h2>Object Access</h2>
      <ObjectAccess />
    </section>
  </main>
</template>

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
