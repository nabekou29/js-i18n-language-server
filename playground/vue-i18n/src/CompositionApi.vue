<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t, te, tm, rt } = useI18n();
</script>

<template>
  <div>
    <h2>{{ $t("page.title") }}</h2>
    <p>{{ $t("page.description") }}</p>
    <p>{{ t("message.welcome", { name: "World" }) }}</p>

    <!-- Attribute binding -->
    <input :placeholder="$t('form.placeholder')" />

    <!-- Conditional rendering with te -->
    <span v-if="te('optional')">{{ t("optional") }}</span>
    <span v-show="$te('visible')">{{ $t("visible") }}</span>

    <!-- tm + rt: iterate raw message object -->
    <ul>
      <li v-for="(msg, key) in tm('common')" :key="key">
        {{ key }}: {{ rt(msg) }}
      </li>
    </ul>

    <!-- Event handler -->
    <button @click="alert($t('message.hello'))">
      {{ $t("form.submit") }}
    </button>
  </div>
</template>
