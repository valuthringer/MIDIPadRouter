<script lang="ts">
  import { app, profile, setMonitor } from '../store.svelte';
  import { t } from '../i18n.svelte';
  import type { Frame } from '../types';

  // Closed by default: the backend records and sends nothing until opened.
  let open = $state(false);

  const hex = (f: Frame) =>
    f.bytes
      .slice(0, f.len)
      .map((b) => b.toString(16).toUpperCase().padStart(2, '0'))
      .join(' ');

  function describe(f: Frame) {
    const name = f.target === null ? null : (profile().targets[f.target]?.name ?? '?');
    if (f.fromLaunchpad) return `Launchpad → ${name ?? t('monitor.unassigned')}`;
    return `${name} → Launchpad`;
  }
</script>

<details class="border-t border-line bg-panel" bind:open ontoggle={() => setMonitor(open)}>
  <summary class="cursor-pointer px-4 py-2 text-sm text-muted">{t('monitor.title')}</summary>
  {#if open}
    <ol class="max-h-48 overflow-y-auto px-4 pb-3 font-mono text-xs" aria-live="off">
      {#each [...(app.snap?.monitor ?? [])].reverse() as f, i (i)}
        <li class="flex gap-3 {f.routed ? '' : 'text-muted'}">
          <span class="w-56 shrink-0 truncate">{describe(f)}</span>
          <span>{hex(f)}</span>
          {#if !f.routed}<span>({t('monitor.blocked')})</span>{/if}
        </li>
      {:else}
        <li class="text-muted">{t('monitor.empty')}</li>
      {/each}
    </ol>
  {/if}
</details>
