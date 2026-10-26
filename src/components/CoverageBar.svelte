<script lang="ts">
  import type { Coverage } from "../lib/api";

  let { coverage }: { coverage: Coverage } = $props();

  const pct = (n: number) => (coverage.words ? (100 * n) / coverage.words : 0);
  const parts = $derived([
    { cls: "known", n: coverage.known, label: "known" },
    { cls: "learning", n: coverage.learning, label: "learning" },
    { cls: "new", n: coverage.new, label: "new in Anki" },
    { cls: "unknown", n: coverage.unknown, label: "not in Anki" },
  ]);
</script>

<div class="coverage" title="Share of the words in this text that you know">
  <div class="headline">
    <strong>{Math.round(coverage.ratio * 100)}%</strong>
    <span class="muted">of {coverage.words.toLocaleString()} words known</span>
  </div>
  <div class="bar" role="img" aria-label="{Math.round(coverage.ratio * 100)}% known">
    {#each parts as p}
      {#if p.n > 0}<span class={p.cls} style:width="{pct(p.n)}%"></span>{/if}
    {/each}
  </div>
  <div class="legend">
    {#each parts as p}
      <span><i class={p.cls}></i>{p.n} {p.label}</span>
    {/each}
  </div>
</div>

<style>
  .coverage {
    display: grid;
    gap: 0.35rem;
    min-width: 260px;
  }
  .headline strong {
    font-size: 1.35rem;
    margin-right: 0.35rem;
  }
  .bar {
    display: flex;
    height: 8px;
    border-radius: 999px;
    overflow: hidden;
    background: var(--surface-2);
  }
  .legend {
    display: flex;
    gap: 0.9rem;
    font-size: 0.78rem;
    color: var(--muted);
    flex-wrap: wrap;
  }
  .legend i {
    display: inline-block;
    width: 0.65rem;
    height: 0.65rem;
    border-radius: 3px;
    margin-right: 0.3rem;
    vertical-align: -0.05rem;
  }
  .known {
    background: var(--accent);
  }
  .learning {
    background: var(--st-learning-line);
  }
  .new {
    background: var(--st-new-line);
  }
  .unknown {
    background: var(--st-unknown-line);
  }
</style>
