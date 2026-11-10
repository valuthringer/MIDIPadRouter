<script lang="ts">
  import { profile, save, setPreview, app } from '../store.svelte';
  import { t } from '../i18n.svelte';
  import type { Mode } from '../types';

  const modes: Mode[] = ['switch', 'combined'];

  function pick(m: Mode) {
    if (profile().mode === m) return;
    profile().mode = m;
    if (app.snap?.preview) void setPreview(false);
    save(0);
  }
</script>

<div
  class="flex rounded-xl border border-line bg-bg p-1"
  role="radiogroup"
  aria-label={t('mode.label')}
>
  {#each modes as m (m)}
    {@const on = profile().mode === m}
    <button
      role="radio"
      aria-checked={on}
      title={t(`mode.${m}Help`)}
      class="rounded-lg px-5 py-1.5 text-sm font-medium transition-colors {on
        ? 'bg-accent text-accent-fg'
        : 'text-muted hover:text-fg'}"
      onclick={() => pick(m)}>{t(`mode.${m}`)}</button
    >
  {/each}
</div>
