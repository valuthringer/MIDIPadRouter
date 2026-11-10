<script lang="ts">
  import { onMount } from 'svelte';
  import { app, init, redo, undo } from './lib/store.svelte';
  import { i18n, t } from './lib/i18n.svelte';
  import ModeSwitch from './lib/components/ModeSwitch.svelte';
  import ProfileMenu from './lib/components/ProfileMenu.svelte';
  import PadGrid from './lib/components/PadGrid.svelte';
  import TargetList from './lib/components/TargetList.svelte';
  import MidiMonitor from './lib/components/MidiMonitor.svelte';
  import SetupWizard from './lib/components/SetupWizard.svelte';
  import Settings from './lib/components/Settings.svelte';

  onMount(() => void init());

  $effect(() => {
    const s = app.config?.settings;
    if (!s) return;
    i18n.lang = s.language;
    document.documentElement.lang = s.language;
    if (s.theme === 'auto') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = s.theme;
  });

  function onKey(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement).tagName;
    if (!e.ctrlKey || tag === 'INPUT' || tag === 'SELECT') return;
    const k = e.key.toLowerCase();
    if (k === 'z' && !e.shiftKey) undo();
    else if (k === 'y' || (k === 'z' && e.shiftKey)) redo();
    else return;
    e.preventDefault();
  }

  const lpDot = { ok: 'bg-ok', missing: 'bg-danger', busy: 'bg-danger', unsupported: 'bg-danger' };
</script>

<svelte:window onkeydown={onKey} />

{#if app.config && app.snap}
  <div class="flex h-screen flex-col">
    <header class="flex flex-wrap items-center gap-3 border-b border-line bg-panel px-4 py-2">
      <h1 class="text-base font-semibold">MIDIPadRouter</h1>
      <ModeSwitch />
      <ProfileMenu />
      <span class="ml-auto flex items-center gap-2 text-sm" role="status">
        <span class="h-2.5 w-2.5 rounded-full {lpDot[app.snap.launchpad]}" aria-hidden="true"
        ></span>
        {t('lp.' + app.snap.launchpad)}
      </span>
      <button
        class="btn"
        onclick={() => (app.settings = true)}
        aria-label={t('settings.open')}
        title={t('settings.open')}>⚙</button
      >
    </header>

    {#if app.error}
      <div class="flex items-start gap-3 bg-warn px-4 py-2 text-sm" role="alert">
        <p>
          <strong>{t('err.title')} :</strong>
          {t('err.' + app.error.code, { arg: app.error.arg })}
        </p>
      </div>
    {/if}
    {#if app.warning}
      <div class="flex items-start gap-3 bg-warn px-4 py-2 text-sm" role="alert">
        <p class="flex-1">
          {t('warn.config', { arg: t('err.' + app.warning.code, { arg: app.warning.arg }) })}
        </p>
        <button class="btn" onclick={() => (app.warning = null)} aria-label={t('common.dismiss')}
          >✕</button
        >
      </div>
    {/if}
    {#if app.shortcutErrors.length}
      <div class="bg-warn px-4 py-2 text-sm" role="alert">
        {t('warn.shortcuts', { arg: app.shortcutErrors.join(', ') })}
      </div>
    {/if}

    <main class="flex min-h-0 flex-1 gap-4 p-4">
      <section class="flex min-w-0 flex-1 flex-col items-center overflow-y-auto">
        <PadGrid />
      </section>
      <aside class="w-80 shrink-0 overflow-y-auto">
        <TargetList />
      </aside>
    </main>

    <MidiMonitor />
  </div>

  {#if app.wizard}<SetupWizard />{/if}
  {#if app.settings}<Settings />{/if}
{/if}
