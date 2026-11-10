<script lang="ts">
  import { app, exportProfile, history, importProfile, saveNow } from '../store.svelte';
  import { t } from '../i18n.svelte';

  let editing: 'new' | 'rename' | null = $state(null);
  let name = $state('');
  let confirmDelete = $state(false);

  const current = $derived(app.config!.activeProfile);
  const names = $derived(Object.keys(app.config!.profiles));
  const taken = $derived(name.trim() !== current && names.includes(name.trim()));

  function switchTo(n: string) {
    app.config!.activeProfile = n;
    history.clear();
    void saveNow();
  }

  function start(kind: 'new' | 'rename') {
    editing = kind;
    name = kind === 'new' ? t('profile.copy', { name: current }) : current;
  }

  function commit() {
    const n = name.trim();
    if (!n || taken) return;
    const c = app.config!;
    const copy = $state.snapshot(c.profiles[current]);
    if (editing === 'rename') delete c.profiles[current];
    c.profiles[n] = copy;
    c.activeProfile = n;
    editing = null;
    history.clear();
    void saveNow();
  }

  function remove() {
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    confirmDelete = false;
    const c = app.config!;
    delete c.profiles[current];
    c.activeProfile = Object.keys(c.profiles)[0];
    history.clear();
    void saveNow();
  }
</script>

{#if editing}
  <form
    class="flex items-center gap-1"
    onsubmit={(e) => {
      e.preventDefault();
      commit();
    }}
  >
    <!-- svelte-ignore a11y_autofocus -->
    <input class="field w-44" bind:value={name} aria-label={t('profile.name')} autofocus />
    <button class="btn-primary" type="submit" disabled={!name.trim() || taken}>OK</button>
    <button class="btn" type="button" onclick={() => (editing = null)}>✕</button>
    {#if taken}<span class="text-xs text-danger" role="alert">{t('profile.exists')}</span>{/if}
  </form>
{:else}
  <div class="flex items-center gap-1">
    <label class="flex items-center gap-1.5 text-sm text-muted">
      {t('profile.label')}
      <select
        class="field w-40 text-fg"
        value={current}
        onchange={(e) => switchTo(e.currentTarget.value)}
      >
        {#each names as n (n)}<option value={n}>{n}</option>{/each}
      </select>
    </label>
    <button
      class="btn"
      onclick={() => start('new')}
      title={t('profile.new')}
      aria-label={t('profile.new')}>+</button
    >
    <button
      class="btn"
      onclick={() => start('rename')}
      title={t('profile.rename')}
      aria-label={t('profile.rename')}>✎</button
    >
    {#if names.length > 1}
      <button
        class="btn {confirmDelete ? 'border-danger text-danger' : ''}"
        onclick={remove}
        onblur={() => (confirmDelete = false)}
        title={confirmDelete ? t('profile.confirmDelete') : t('profile.delete')}
        aria-label={confirmDelete ? t('profile.confirmDelete') : t('profile.delete')}
        >{confirmDelete ? '✓' : '🗑'}</button
      >
    {/if}
    <button class="btn" onclick={importProfile}>{t('profile.import')}</button>
    <button class="btn" onclick={() => exportProfile(current)}>{t('profile.export')}</button>
  </div>
{/if}
