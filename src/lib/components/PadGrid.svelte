<script lang="ts">
  import { app, editAssignment, profile, redo, setPreview, undo } from '../store.svelte';
  import { t } from '../i18n.svelte';
  import { ALL_KEYS, cellKey, keyCell, keyFromRef, keyId, ledColor } from '../keys';
  import { halfKeys, paint, shapeKeys, type Shape } from '../paint';

  const p = $derived(profile());
  const combined = $derived(p.mode === 'combined');
  const switchKey = $derived(keyFromRef(p.switchKey));
  const active = $derived(app.snap!.active);
  const rows = Array.from({ length: 9 }, (_, i) => i);

  let shape: Shape = $state('key');
  let focus = $state(0); // roving tabindex
  let buttons: HTMLButtonElement[] = $state([]);
  /** Target painted by the current stroke; undefined when not painting. */
  let stroke: string | null | undefined;

  const ownerIndex = (k: number) => p.targets.findIndex((x) => x.id === p.assignment[keyId(k)]);

  function led(k: number): number {
    const i = combined ? ownerIndex(k) : active;
    return i < 0 ? 0 : (app.snap!.targets[i]?.leds[k] ?? 0);
  }

  function background(k: number): string {
    if (k === switchKey) return 'var(--pad)';
    if (!combined) return ledColor(led(k)) ?? 'var(--pad)';
    const i = ownerIndex(k);
    return i < 0 ? 'var(--pad)' : `color-mix(in srgb, ${p.targets[i].color} 80%, var(--pad))`;
  }

  function label(k: number): string {
    const [r, c] = keyCell(k);
    const pos =
      r === 0
        ? t('grid.top', { n: c + 1 })
        : c === 8
          ? t('grid.scene', { n: r })
          : t('grid.pad', { row: r, col: c + 1 });
    if (k === switchKey) return `${pos} — ${t('grid.switchKey')}`;
    if (combined) return `${pos} — ${p.targets[ownerIndex(k)]?.name ?? t('grid.none')}`;
    return `${pos} — ${ledColor(led(k)) ? t('grid.ledOn') : t('grid.ledOff')}`;
  }

  function apply(k: number, target: string | null, record: boolean) {
    editAssignment(paint(p.assignment, shapeKeys(k, shape), target, switchKey), record);
  }

  function fill(keys: number[], target: string | null) {
    editAssignment(paint(p.assignment, keys, target, switchKey));
  }

  function down(e: PointerEvent, k: number) {
    focus = k;
    if (!combined || (e.button !== 0 && e.button !== 2)) return;
    // Touch captures the pointer implicitly: release it so dragging paints other keys.
    (e.currentTarget as HTMLElement).releasePointerCapture?.(e.pointerId);
    stroke = e.button === 2 ? null : app.brush;
    apply(k, stroke, true);
  }

  function enter(k: number) {
    if (stroke !== undefined) apply(k, stroke, false);
  }

  const moves: Record<string, [number, number]> = {
    ArrowUp: [-1, 0],
    ArrowDown: [1, 0],
    ArrowLeft: [0, -1],
    ArrowRight: [0, 1],
  };

  function onKeydown(e: KeyboardEvent) {
    const m = moves[e.key];
    if (m) {
      const [r, c] = keyCell(focus);
      const [nr, nc] = [r + m[0], c + m[1]];
      const k = nr >= 0 && nr < 9 && nc >= 0 && nc < 9 ? cellKey(nr, nc) : null;
      if (k !== null) {
        focus = k;
        buttons[k]?.focus();
      }
    } else if (combined && (e.key === 'Enter' || e.key === ' ')) {
      apply(focus, app.brush, true);
    } else if (combined && (e.key === 'Delete' || e.key === 'Backspace')) {
      apply(focus, null, true);
    } else return;
    e.preventDefault();
  }
</script>

<svelte:window onpointerup={() => (stroke = undefined)} />

<!-- Square grid sized to fit the window height (header + toolbar + hints). -->
<div
  class="flex w-full flex-col gap-3"
  style="max-width: max(280px, min(480px, calc(100vh - {combined ? 370 : 215}px)))"
>
  {#if combined}
    <h2 class="text-sm font-semibold">{t('grid.editorTitle')}</h2>
    <div class="flex flex-wrap items-center gap-2" role="radiogroup" aria-label={t('paint.brush')}>
      <span class="text-xs text-muted">{t('paint.brush')} :</span>
      {#each p.targets as target (target.id)}
        <button
          role="radio"
          aria-checked={app.brush === target.id}
          class="flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs {app.brush ===
          target.id
            ? 'border-fg font-semibold'
            : 'border-line'}"
          onclick={() => (app.brush = target.id)}
        >
          <span class="h-3 w-3 rounded-full" style="background:{target.color}"></span>
          {target.name}
        </button>
      {/each}
      <button
        role="radio"
        aria-checked={app.brush === null}
        class="rounded-full border px-2.5 py-1 text-xs {app.brush === null
          ? 'border-fg font-semibold'
          : 'border-line'}"
        onclick={() => (app.brush = null)}>{t('paint.none')}</button
      >
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <label class="flex items-center gap-1 text-xs text-muted">
        {t('paint.shape')}
        <select class="field w-auto" bind:value={shape}>
          <option value="key">{t('paint.key')}</option>
          <option value="row">{t('paint.row')}</option>
          <option value="column">{t('paint.column')}</option>
        </select>
      </label>
      <button class="btn" onclick={() => fill(ALL_KEYS, app.brush)}>{t('paint.all')}</button>
      <button class="btn" onclick={() => fill(halfKeys('left'), app.brush)}
        >{t('paint.left')}</button
      >
      <button class="btn" onclick={() => fill(halfKeys('right'), app.brush)}
        >{t('paint.right')}</button
      >
      <button class="btn" onclick={() => editAssignment({})}>{t('paint.clear')}</button>
      <button class="btn" onclick={undo} aria-label={t('paint.undo')} title={t('paint.undo')}
        >↶</button
      >
      <button class="btn" onclick={redo} aria-label={t('paint.redo')} title={t('paint.redo')}
        >↷</button
      >
    </div>
  {:else}
    <h2 class="text-sm font-semibold">
      {t('grid.switchTitle', { name: p.targets[active]?.name ?? '' })}
    </h2>
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="grid grid-cols-9 gap-1.5 rounded-2xl border border-line bg-panel p-3 {combined
      ? 'touch-none'
      : ''}"
    role="group"
    aria-label={t('grid.label')}
    onkeydown={onKeydown}
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#each rows as row (row)}
      {#each rows as col (col)}
        {@const k = cellKey(row, col)}
        {#if k === null}
          <div></div>
        {:else}
          {@const round = row === 0 || col === 8}
          {@const dot = combined ? ledColor(led(k)) : null}
          <button
            bind:this={buttons[k]}
            tabindex={k === focus ? 0 : -1}
            class="relative flex aspect-square items-center justify-center border border-[var(--pad-line)] text-sm font-bold {round
              ? 'scale-[0.8] rounded-full'
              : 'rounded-md'}"
            style="background:{background(k)}{k === switchKey && !combined
              ? `;border:3px solid ${p.targets[active]?.color}`
              : ''}"
            aria-label={label(k)}
            title={label(k)}
            onpointerdown={(e) => down(e, k)}
            onpointerenter={() => enter(k)}
            onfocus={() => (focus = k)}
          >
            {#if k === switchKey}⇄{/if}
            {#if dot}
              <span class="absolute inset-[32%] rounded-full shadow" style="background:{dot}"
              ></span>
            {/if}
          </button>
        {/if}
      {/each}
    {/each}
  </div>

  {#if combined}
    <p class="text-xs text-muted">{t('paint.hint')}</p>
    <label class="flex items-center gap-2 text-sm">
      <input
        type="checkbox"
        checked={app.snap!.preview}
        onchange={(e) => setPreview(e.currentTarget.checked)}
      />
      {t('paint.mirror')}
    </label>
  {:else}
    <p class="text-xs text-muted">{t('grid.switchHint')}</p>
  {/if}
</div>
