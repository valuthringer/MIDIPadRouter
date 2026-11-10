<script lang="ts">
  import { app, history, profile, save } from '../store.svelte';
  import { t } from '../i18n.svelte';
  import { keyCell, keyFromRef, keyId, keyRef } from '../keys';

  const s = $derived(app.config!.settings);
  // Top buttons (72..79) and scene buttons (64..71) can serve as switch key.
  const candidates = [
    ...Array.from({ length: 8 }, (_, i) => 72 + i),
    ...Array.from({ length: 8 }, (_, i) => 64 + i),
  ];
  const keyName = (k: number) => {
    const [r, c] = keyCell(k);
    return r === 0 ? t('grid.top', { n: c + 1 }) : t('grid.scene', { n: r });
  };

  function setSwitchKey(k: number) {
    const p = profile();
    p.switchKey = k < 0 ? null : keyRef(k);
    if (k >= 0) delete p.assignment[keyId(k)]; // the switch key is reserved
    history.clear();
    save(0);
  }
</script>

<div
  class="modal-backdrop"
  role="presentation"
  onclick={(e) => e.target === e.currentTarget && (app.settings = false)}
>
  <div
    class="modal flex flex-col gap-4"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
  >
    <h2 id="settings-title" class="text-lg font-semibold">{t('settings.title')}</h2>

    <label class="flex flex-col gap-1 text-sm">
      {t('settings.language')}
      <select class="field" bind:value={s.language} onchange={() => save(0)}>
        <option value="fr">Français</option>
        <option value="en">English</option>
      </select>
    </label>

    <label class="flex flex-col gap-1 text-sm">
      {t('settings.theme')}
      <select class="field" bind:value={s.theme} onchange={() => save(0)}>
        <option value="auto">{t('settings.themeAuto')}</option>
        <option value="light">{t('settings.themeLight')}</option>
        <option value="dark">{t('settings.themeDark')}</option>
      </select>
    </label>

    <label class="flex flex-col gap-1 text-sm">
      {t('settings.switchKey')}
      <select
        class="field"
        value={String(keyFromRef(profile().switchKey))}
        onchange={(e) => setSwitchKey(Number(e.currentTarget.value))}
      >
        <option value="-1">{t('settings.noSwitchKey')}</option>
        {#each candidates as k (k)}<option value={String(k)}>{keyName(k)}</option>{/each}
      </select>
    </label>

    <label class="flex items-center gap-2 text-sm">
      <input type="checkbox" bind:checked={s.autostart} onchange={() => save(0)} />
      {t('settings.autostart')}
    </label>
    <label class="flex items-center gap-2 text-sm">
      <input type="checkbox" bind:checked={s.startMinimized} onchange={() => save(0)} />
      {t('settings.startMinimized')}
    </label>

    <div class="flex justify-between gap-2 pt-2">
      <button
        class="btn"
        onclick={() => {
          app.settings = false;
          app.wizard = true;
        }}>{t('settings.wizard')}</button
      >
      <button class="btn-primary" onclick={() => (app.settings = false)}
        >{t('settings.close')}</button
      >
    </div>
  </div>
</div>
