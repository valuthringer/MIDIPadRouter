<script lang="ts">
  import { app, openHelp, profile, refreshPorts, save, saveNow } from '../store.svelte';
  import { t } from '../i18n.svelte';
  import { defaultPorts } from '../targets';
  import type { Mode } from '../types';

  let step = $state(1);
  const windows = $derived(app.platform === 'windows');
  const p = $derived(profile());
  const lpText = {
    ok: 'wizard.lpOk',
    missing: 'wizard.lpMissing',
    busy: 'wizard.lpBusy',
    unsupported: 'wizard.lpUnsupported',
  };

  // Check the ports every 2 s while this step is on screen only.
  $effect(() => {
    if (step !== 2) return;
    void refreshPorts();
    const id = setInterval(refreshPorts, 2000);
    return () => clearInterval(id);
  });

  function useDefaultNames() {
    for (const target of p.targets) Object.assign(target, defaultPorts(target.id));
    save(0);
  }

  function createVirtual() {
    app.config!.settings.virtualPorts = true;
    useDefaultNames();
  }

  function pickMode(m: Mode) {
    p.mode = m;
    save(0);
  }

  function close() {
    app.config!.settings.setupDone = true;
    app.wizard = false;
    void saveNow();
  }
</script>

<div class="modal-backdrop" role="presentation">
  <div
    class="modal flex flex-col gap-4"
    role="dialog"
    aria-modal="true"
    aria-labelledby="wizard-title"
  >
    <div class="flex items-center justify-between">
      <h2 id="wizard-title" class="text-lg font-semibold">{t('wizard.title')}</h2>
      <button class="btn" onclick={close} aria-label={t('wizard.close')}>✕</button>
    </div>
    <p class="text-xs text-muted">{t('wizard.step', { n: step })}</p>

    {#if step === 1}
      <h3 class="font-semibold">{t('wizard.lpTitle')}</h3>
      <p role="status" class={app.snap?.launchpad === 'ok' ? 'text-ok' : ''}>
        {app.snap?.launchpad === 'ok' ? '🟢' : '🔴'}
        {t(lpText[app.snap?.launchpad ?? 'missing'])}
      </p>
    {:else if step === 2}
      <h3 class="font-semibold">{t('wizard.portsTitle')}</h3>
      <p class="text-sm">{t('wizard.pipes')}</p>
      {#if windows}
        <p class="text-sm">{t('wizard.portsWin')}</p>
        <div class="flex gap-2">
          <button class="btn" onclick={() => openHelp('midiservices')}
            >{t('wizard.midiservices')}</button
          >
          <button class="btn" onclick={() => openHelp('loopmidi')}>{t('wizard.loopmidi')}</button>
        </div>
      {:else if !app.config!.settings.virtualPorts}
        <p class="text-sm">{t('wizard.portsUnix')}</p>
        <button class="btn-primary self-start" onclick={createVirtual}
          >{t('wizard.createVirtual')}</button
        >
      {:else}
        <p class="text-sm text-ok">{t('wizard.virtualDone')}</p>
      {/if}

      <ul class="flex flex-col gap-2">
        {#each p.targets as target, i (target.id)}
          {@const snap = app.snap?.targets[i]}
          {@const toOk = app.config!.settings.virtualPorts
            ? !!snap?.toApp
            : app.ports.outputs.includes(target.portToApp)}
          {@const fromOk = app.config!.settings.virtualPorts
            ? !!snap?.fromApp
            : app.ports.inputs.includes(target.portFromApp)}
          <li class="card text-sm" style="border-left: 4px solid {target.color}">
            <strong>{target.name}</strong>
            <div class="font-mono text-xs">
              {toOk ? '🟢' : '🔴'} « {target.portToApp} » — {t(
                toOk ? 'wizard.found' : 'wizard.missing',
              )}
            </div>
            <div class="font-mono text-xs">
              {fromOk ? '🟢' : '🔴'} « {target.portFromApp} » — {t(
                fromOk ? 'wizard.found' : 'wizard.missing',
              )}
            </div>
            <p class="mt-1 text-xs text-muted">
              {t('wizard.inApp', {
                name: target.name,
                to: target.portToApp,
                from: target.portFromApp,
              })}
            </p>
          </li>
        {/each}
      </ul>
      {#if windows}
        <button class="btn self-start" onclick={useDefaultNames}>{t('wizard.useDefaults')}</button>
      {/if}
    {:else}
      <h3 class="font-semibold">{t('wizard.modeTitle')}</h3>
      {#each ['switch', 'combined'] as const as m (m)}
        <button
          class="card text-left {p.mode === m ? 'border-2 border-accent' : ''}"
          aria-pressed={p.mode === m}
          onclick={() => pickMode(m)}
        >
          <strong>{t(`mode.${m}`)}</strong>
          <p class="text-sm text-muted">{t(`wizard.${m}Desc`)}</p>
        </button>
      {/each}
    {/if}

    <div class="flex justify-between pt-2">
      <button class="btn" disabled={step === 1} onclick={() => step--}>{t('wizard.back')}</button>
      {#if step < 3}
        <button class="btn-primary" onclick={() => step++}>{t('wizard.next')}</button>
      {:else}
        <button class="btn-primary" onclick={close}>{t('wizard.finish')}</button>
      {/if}
    </div>
  </div>
</div>
