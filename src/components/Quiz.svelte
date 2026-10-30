<script lang="ts">
  // Questions after a text (ADR 0019): one at a time, the answer and its explanation right away.
  import { onMount } from "svelte";
  import { api, type QuizItem } from "../lib/api";

  let { textId, ondone = () => {} }: { textId: string; ondone?: (r: { right: number; total: number; missed_words: string[] }) => void } =
    $props();

  let items = $state<QuizItem[]>([]);
  let at = $state(0);
  let chosen = $state<number | null>(null);
  let answers: [string, number][] = [];
  let result = $state<{ right: number; total: number; missed_words: string[] } | null>(null);
  let error = $state("");
  let loading = $state(true);

  onMount(async () => {
    try {
      items = await api.quizQuestions(textId);
      if (!items.length) error = "The AI's questions didn't pass the check this time; try again later.";
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  const item = $derived(items[at]);

  function choose(i: number) {
    if (chosen !== null || !item) return;
    chosen = i;
    answers.push([item.id, i]);
  }

  async function next() {
    if (at + 1 < items.length) {
      at += 1;
      chosen = null;
      return;
    }
    try {
      result = await api.quizSubmit(textId, answers);
      ondone(result);
    } catch (e) {
      error = String(e);
    }
  }

  function onkey(e: KeyboardEvent) {
    if (!item || result) return;
    if (chosen === null && /^[1-4]$/.test(e.key) && Number(e.key) <= item.options.length) choose(Number(e.key) - 1);
    else if (chosen !== null && e.key === "Enter") next();
  }
</script>

<svelte:window onkeydown={onkey} />

<section class="card quiz">
  {#if loading}
    <p class="muted">Writing a few questions about the text…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if result}
    <p class="score"><strong>{result.right} of {result.total}</strong> right.</p>
    {#if result.missed_words.length}
      <p class="muted">{result.missed_words.join("、")} will come back in your next practice words.</p>
    {:else}
      <p class="muted">Nicely done.</p>
    {/if}
  {:else if item}
    <p class="muted small">Question {at + 1} of {items.length}{item.kind === "cloze" ? " · fill the gap" : ""}</p>
    <p class="question">{item.question}</p>
    <div class="options">
      {#each item.options as o, i}
        <button
          class:right={chosen !== null && i === item.answer}
          class:wrong={chosen === i && i !== item.answer}
          disabled={chosen !== null && i !== item.answer && chosen !== i}
          onclick={() => choose(i)}><kbd>{i + 1}</kbd> {o}</button
        >
      {/each}
    </div>
    {#if chosen !== null}
      <p class="expl">{chosen === item.answer ? "Right. " : "Not quite. "}{item.explanation}</p>
      <button class="primary" onclick={next}>{at + 1 < items.length ? "Next" : "See the result"}</button>
    {/if}
  {/if}
</section>

<style>
  .quiz {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .small {
    font-size: 0.85rem;
    margin: 0;
  }
  .question {
    font-family: var(--font-read);
    font-size: 1.25rem;
    margin: 0;
  }
  .options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .options button {
    text-align: left;
    font-family: var(--font-read);
    font-size: 1.05rem;
  }
  .options button.right {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .options button.wrong {
    border-color: var(--danger);
    color: var(--danger);
  }
  .expl {
    background: var(--surface-2);
    border-radius: 6px;
    padding: 0.5rem 0.8rem;
    margin: 0;
  }
  .score {
    font-size: 1.3rem;
    margin: 0;
  }
  .quiz > button {
    align-self: flex-start;
  }
  kbd {
    font-family: var(--font-ui);
    font-size: 0.75rem;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 0 0.35em;
    margin-right: 0.3rem;
  }
</style>
