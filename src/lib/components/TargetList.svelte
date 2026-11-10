<script lang="ts">
  import { app, history, profile, refreshPorts, save, setActive } from '../store.svelte';
  import { t } from '../i18n.svelte';
  import { newTarget, MAX_TARGETS } from '../targets';

  const p = $derived(profile());
  const virt = $derived(app.config!.settings.virtualPorts && app.platform !== 'windows');
  let confirmDelete: string | null = $state(null);

  // The Launchpad itself is never a valid target port.
  const usable = (list: string[]) => list.filter((n) => !n.toLowerCase().includes('launchpad'));

  function status(enabled: boolean, port: string, connected: boolean, list: string[]) {
    if (!enabled) return { ok: false, text: t('status.disabled') };
    if (!port) return { ok: false, text: t('status.noPort') };
    if (connected) return { ok: true, text: t('status.ok') };
    if (!virt && !list.includes(port))
      return { ok: false, text: t('status.missingPort', { port }) };
    return { ok: false, text: t('status.closed', { port }) };
  }

  function add() {
    const target = newTarget(p.targets, t);
    p.targets.push(target);
    p.shortcuts[target.id] = `Ctrl+Alt+${'abcde'.indexOf(target.id) + 1}`;
    save(0);
  }

  function remove(id: string) {
    if (confirmDelete !== id) {
      confirmDelete = id;
      return;
    }
    confirmDelete = null;
    p.targets = p.targets.filter((x) => x.id !== id);
    delete p.shortcuts[id];
    p.assignment = Object.fromEntries(Object.entries(p.assignment).filter(([, v]) => v !== id));
    history.clear(); // undo could bring back keys of the deleted application
    if (app.brush === id) app.brush = p.targets[0]?.id ?? null;
    save(0);
  }
</script>

<div class="flex flex-col gap-3">
  <div class="flex items-center justify-between">
    <h2 class="text-sm font-semibold">{t('targets.title')}</h2>
    <button
      class="btn"
      onclick={refreshPorts}
      aria-label={t('targets.refresh')}
      title={t('targets.refresh')}>↻</button
    >
  </div>

  {#each p.targets as target, i (target.id)}
    {@const snap = app.snap?.targets[i]}
    {@const isActive = app.snap?.active === i}
    {@const out = status(target.enabled, target.portToApp, !!snap?.toApp, app.ports.outputs)}
    {@const inn = status(target.enabled, target.portFromApp, !!snap?.fromApp, app.ports.inputs)}
    <div
      class="card flex flex-col gap-2 border-l-4"
      style="border-left-color:{target.color}"
      aria-label={target.name}
      role="group"
    >
      <div class="flex items-center gap-2">
        <input
          type="color"
          class="h-7 w-7 shrink-0 cursor-pointer rounded border-0 bg-transparent p-0"
          bind:value={target.color}
          onchange={() => save()}
          aria-label={t('targets.color')}
        />
        <input
          class="field font-medium"
          bind:value={target.name}
          oninput={() => save(600)}
          aria-label={t('targets.name')}
        />
        {#key app.snap}
          <span
            class="h-2.5 w-2.5 shrink-0 rounded-full {snap?.activity
              ? 'blink bg-accent'
              : 'bg-line'}"
            title={t('status.activity')}
            aria-hidden="true"
          ></span>
        {/key}
      </div>

      {#if virt}
        <p class="text-xs text-muted">
          {t('targets.virtual', { to: target.portToApp, from: target.portFromApp })}
        </p>
      {:else}
        <label class="text-xs text-muted">
          {t('targets.toApp')}
          <select class="field mt-0.5" bind:value={target.portToApp} onchange={() => save(0)}>
            <option value="">{t('targets.choose')}</option>
            {#if target.portToApp && !app.ports.outputs.includes(target.portToApp)}
              <option value={target.portToApp}
                >{t('targets.notFound', { port: target.portToApp })}</option
              >
            {/if}
            {#each usable(app.ports.outputs) as port (port)}
              <option value={port}>{port}</option>
            {/each}
          </select>
        </label>
        <label class="text-xs text-muted">
          {t('targets.fromApp')}
          <select class="field mt-0.5" bind:value={target.portFromApp} onchange={() => save(0)}>
            <option value="">{t('targets.choose')}</option>
            {#if target.portFromApp && !app.ports.inputs.includes(target.portFromApp)}
              <option value={target.portFromApp}
                >{t('targets.notFound', { port: target.portFromApp })}</option
              >
            {/if}
            {#each usable(app.ports.inputs) as port (port)}
              <option value={port}>{port}</option>
            {/each}
          </select>
        </label>
      {/if}

      <ul class="text-xs">
        {#each [out, inn] as s, j (j)}
          <li class="flex items-center gap-1.5 {s.ok ? 'text-ok' : 'text-danger'}">
            <span aria-hidden="true">{s.ok ? '🟢' : '🔴'}</span>
            {j === 0 ? '→' : '←'}
            {s.text}
          </li>
        {/each}
      </ul>

      {#if p.mode === 'switch'}
        <label class="text-xs text-muted">
          {t('targets.shortcut')}
          <input
            class="field mt-0.5"
            placeholder="Ctrl+Alt+1"
            value={p.shortcuts[target.id] ?? ''}
            onchange={(e) => {
              p.shortcuts[target.id] = e.currentTarget.value.trim();
              save(0);
            }}
          />
        </label>
      {/if}

      <div class="flex items-center gap-2">
        <label class="flex items-center gap-1.5 text-sm">
          <input type="checkbox" bind:checked={target.enabled} onchange={() => save(0)} />
          {t('targets.enabled')}
        </label>
        <span class="flex-1"></span>
        {#if p.targets.length > 1}
          <button
            class="btn {confirmDelete === target.id ? 'border-danger text-danger' : ''}"
            onclick={() => remove(target.id)}
            onblur={() => (confirmDelete = null)}
            >{confirmDelete === target.id
              ? t('targets.confirmDelete')
              : t('targets.delete')}</button
          >
        {/if}
        {#if p.mode === 'switch'}
          {#if isActive}
            <span class="rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-accent-fg"
              >{t('targets.active')}</span
            >
          {:else}
            <button class="btn" disabled={!target.enabled} onclick={() => setActive(i)}
              >{t('targets.activate')}</button
            >
          {/if}
        {/if}
      </div>
    </div>
  {/each}

  {#if p.targets.length < MAX_TARGETS}
    <button class="btn border-dashed py-2" onclick={add}>{t('targets.add')}</button>
  {/if}
</div>
