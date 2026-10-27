<script lang="ts">
  import { onMount } from "svelte";
  import { api, stateLabel, type Friction, type Stats } from "../lib/api";

  let st = $state<Stats | null>(null);
  let error = $state("");
  let carded = $state<Record<string, string>>({});

  onMount(async () => {
    try {
      st = await api.readingStats();
    } catch (e) {
      error = String(e);
    }
  });

  const fmt = (n: number) => (n >= 10000 ? `${(n / 1000).toFixed(n >= 100000 ? 0 : 1)}K` : n.toLocaleString());
  const dayLabel = (d: string) => new Date(`${d}T12:00:00`).toLocaleDateString(undefined, { month: "short", day: "numeric" });

  // --- bar chart: characters per day ---
  const W = 640, H = 150, PAD_T = 18, PAD_B = 22, GAP = 2;
  const max = $derived(st ? Math.max(1, ...st.days.map((d) => d.chars)) : 1);
  const bw = $derived(st ? W / st.days.length : 0);
  let hover = $state<number | null>(null);

  /** A bar with a 4px rounded top, anchored to the baseline. */
  function bar(i: number, chars: number): string {
    const x = i * bw + GAP / 2, w = bw - GAP;
    const h = chars ? Math.max(3, ((H - PAD_T - PAD_B) * chars) / max) : 0;
    const y = H - PAD_B - h, r = Math.min(4, w / 2, h);
    return `M${x},${H - PAD_B} V${y + r} Q${x},${y} ${x + r},${y} H${x + w - r} Q${x + w},${y} ${x + w},${y + r} V${H - PAD_B} Z`;
  }

  async function card(f: Friction) {
    const id = `${f.key.lemma}\u0001${f.key.reading}`;
    carded = { ...carded, [id]: "…" };
    try {
      await api.cardCreate(f.key, f.display, null, null);
      carded = { ...carded, [id]: "✓ in Cards" };
    } catch (e) {
      carded = { ...carded, [id]: String(e) };
    }
  }
  const cardState = (f: Friction) => carded[`${f.key.lemma}\u0001${f.key.reading}`];
</script>

<div class="page">
  <h1>Stats</h1>
  {#if error}<p class="error">{error}</p>{/if}

  {#if st}
    <div class="tiles">
      <div class="tile"><span class="label">Read today</span><span class="value">{fmt(st.chars_today)}</span><span class="sub">characters</span></div>
      <div class="tile">
        <span class="label">Reading streak</span><span class="value">{st.streak} {st.streak === 1 ? "day" : "days"}</span>
        <span class="sub">best {st.best_streak}</span>
      </div>
      <div class="tile"><span class="label">Known words</span><span class="value">{fmt(st.known_words)}</span><span class="sub">Anki and your marks</span></div>
      <div class="tile"><span class="label">Read in total</span><span class="value">{fmt(st.chars_total)}</span><span class="sub">characters · {st.texts_read} {st.texts_read === 1 ? "text" : "texts"}</span></div>
      <div class="tile"><span class="label">Words met</span><span class="value">{fmt(st.words_seen_30d)}</span><span class="sub">last 30 days · {st.words_looked_up_30d} looked up</span></div>
    </div>

    <section class="card">
      <h2>Characters read per day <span class="muted small">· last 30 days</span></h2>
      <div class="chart">
        <svg viewBox="0 0 {W} {H}" role="img" aria-label="Characters read per day, last 30 days" onmouseleave={() => (hover = null)}>
          <line class="axis" x1="0" x2={W} y1={H - PAD_B} y2={H - PAD_B} />
          <text class="tick" x="0" y="10">{fmt(max)}</text>
          {#each st.days as d, i}
            <path class="bar" class:dim={hover !== null && hover !== i} d={bar(i, d.chars)} />
            <!-- Hit target: the whole column, wider than the bar. -->
            <rect class="hit" x={i * bw} y="0" width={bw} height={H - PAD_B} role="presentation" onmouseenter={() => (hover = i)} />
          {/each}
          {#each [0, Math.floor(st.days.length / 2), st.days.length - 1] as i}
            <text class="tick" x={i * bw + bw / 2} y={H - 6} text-anchor={i === 0 ? "start" : i === st.days.length - 1 ? "end" : "middle"}>
              {i === st.days.length - 1 ? "Today" : dayLabel(st.days[i].day)}
            </text>
          {/each}
        </svg>
        {#if hover !== null}
          <div
            class="tip"
            style:left="{((hover + 0.5) / st.days.length) * 100}%"
            style:transform="translateX({hover < 4 ? "-10%" : hover > st.days.length - 5 ? "-90%" : "-50%"})"
          >
            <strong>{st.days[hover].chars.toLocaleString()}</strong> characters<br /><span class="muted">{dayLabel(st.days[hover].day)}</span>
          </div>
        {/if}
      </div>
      <details>
        <summary class="muted small">Show as a table</summary>
        <table>
          <thead><tr><th>Day</th><th>Characters</th></tr></thead>
          <tbody>{#each [...st.days].reverse() as d}<tr><td>{d.day}</td><td>{d.chars.toLocaleString()}</td></tr>{/each}</tbody>
        </table>
      </details>
    </section>

    <section class="card">
      <h2>Stealth leeches</h2>
      <p class="muted small">
        Words Anki says you know, but you looked up again while reading (twice or more in 60 days). Your cards for them
        may not be working; a new card with a fresh sentence can help.
      </p>
      {#if st.stealth_leeches.length}
        <ul class="words">
          {#each st.stealth_leeches as f}
            <li>
              <span class="w">{f.display}</span><span class="muted">{f.key.reading}</span>
              <span class="muted small">looked up {f.lookups}× · {stateLabel[f.state]}{f.strength ? `, ${Math.round(f.strength)} days` : ""}</span>
              {#if cardState(f)}<span class="small">{cardState(f)}</span>{:else}<button class="ghost" onclick={() => card(f)}>+ card</button>{/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">None so far. Good sign.</p>
      {/if}
    </section>

    <section class="card">
      <h2>Looked up most</h2>
      {#if st.most_looked_up.length}
        <ul class="words">
          {#each st.most_looked_up as f}
            <li>
              <span class="w">{f.display}</span><span class="muted">{f.key.reading}</span>
              <span class="muted small">looked up {f.lookups}× · {stateLabel[f.state]}</span>
              {#if cardState(f)}<span class="small">{cardState(f)}</span>{:else}<button class="ghost" onclick={() => card(f)}>+ card</button>{/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">Words you look up more than once show up here.</p>
      {/if}
    </section>
  {:else if !error}
    <p class="muted">Counting…</p>
  {/if}
</div>

<style>
  .page {
    max-width: 880px;
    margin: 0 auto;
    padding: 2rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .small {
    font-size: 0.85rem;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.8rem;
  }
  .tile {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 0.8rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .label {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .value {
    font-size: 1.6rem;
    font-weight: 600;
  }
  .sub {
    color: var(--muted);
    font-size: 0.78rem;
  }
  .chart {
    position: relative;
  }
  svg {
    width: 100%;
    height: auto;
    display: block;
  }
  .bar {
    fill: var(--accent);
    transition: opacity 0.1s;
  }
  .bar.dim {
    opacity: 0.45;
  }
  .hit {
    fill: transparent;
  }
  .axis {
    stroke: var(--line);
    stroke-width: 1;
  }
  .tick {
    fill: var(--muted);
    font-size: 10px;
  }
  .tip {
    position: absolute;
    top: 1.6rem;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.3rem 0.6rem;
    font-size: 0.82rem;
    white-space: nowrap;
    pointer-events: none;
    box-shadow: var(--shadow);
  }
  table {
    margin-top: 0.5rem;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  td,
  th {
    padding: 0.15rem 1rem 0.15rem 0;
    text-align: left;
  }
  .words {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.35rem;
  }
  .words li {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .w {
    font-family: var(--font-read);
    font-size: 1.15rem;
  }
  .words button {
    font-size: 0.8rem;
    padding: 0.1em 0.6em;
    margin-left: auto;
  }
</style>
