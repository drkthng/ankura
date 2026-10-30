<script module lang="ts">
  import { marked } from "marked";

  // Heading ids like GitHub's, so #anchors in the guides work.
  marked.use({
    renderer: {
      heading({ tokens, depth }) {
        const text = this.parser.parseInline(tokens);
        const id = text
          .replace(/<[^>]+>/g, "")
          .toLowerCase()
          .trim()
          .replace(/[^\p{L}\p{N}\s-]/gu, "")
          .replace(/\s+/g, "-");
        return `<h${depth} id="${id}">${text}</h${depth}>`;
      },
    },
  });
</script>

<script lang="ts">
  // The user guides from docs/guide, built into the app: the same files as in the repo.
  import { openUrl } from "@tauri-apps/plugin-opener";

  const files = import.meta.glob(["../../docs/guide/*.md", "../../docs/research/learning-evidence.md", "../../docs/research/feature-ideas.md"], {
    query: "?raw",
    import: "default",
    eager: true,
  }) as Record<string, string>;
  const images = import.meta.glob("../../docs/img/*.png", { query: "?url", import: "default", eager: true }) as Record<string, string>;

  /** Guides in reading order; the learning guide first. */
  const ORDER = [
    "learning",
    "today",
    "reading",
    "write-for-me",
    "cards",
    "kanji",
    "level-check",
    "practice-test",
    "stats",
    "anki",
    "ai",
    "learning-evidence",
    "feature-ideas",
  ];
  const guides = Object.entries(files)
    .map(([path, text]) => {
      const name = path.split("/").pop()!.replace(/\.md$/, "");
      const title = /^#\s+(.+)$/m.exec(text)?.[1] ?? name;
      return { name, title, text, research: path.includes("/research/") };
    })
    .sort((a, b) => (ORDER.indexOf(a.name) + 1 || 99) - (ORDER.indexOf(b.name) + 1 || 99));

  let { start = "learning" }: { start?: string } = $props();
  let current = $state(guides.find((g) => g.name === start)?.name ?? guides[0]?.name);
  let article: HTMLElement | undefined = $state();

  const html = $derived.by(() => {
    const g = guides.find((x) => x.name === current);
    if (!g) return "";
    // The "← back" link at the top is for reading on GitHub; the list on the left does that here.
    const text = g.text.replace(/^\[← [^\]]*\]\([^)]*\)\s*$/m, "");
    return marked.parse(text, { async: false }) as string;
  });

  // Images point at ../img/x.png in the repo; use the bundled files.
  $effect(() => {
    void html;
    article?.querySelectorAll("img").forEach((img) => {
      const file = img.getAttribute("src")?.split("/").pop();
      const url = Object.entries(images).find(([p]) => p.endsWith(`/${file}`))?.[1];
      if (url) img.src = url;
    });
    article?.scrollTo?.(0, 0);
  });

  /** Guide links switch the page, anchors scroll, web links open in the browser. */
  function onclick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    const href = a.getAttribute("href") ?? "";
    e.preventDefault();
    if (/^https?:/.test(href)) {
      openUrl(href).catch(() => {});
      return;
    }
    const [file, hash] = href.split("#");
    const name = file.split("/").pop()?.replace(/\.md$/, "");
    if (name && guides.some((g) => g.name === name)) current = name;
    if (hash) setTimeout(() => article?.querySelector(`[id="${hash}"]`)?.scrollIntoView({ behavior: "smooth" }), 50);
  }

</script>

<div class="guide">
  <nav class="toc" aria-label="Guides">
    <p class="muted small">Guides</p>
    {#each guides.filter((g) => !g.research) as g}
      <button class:on={g.name === current} onclick={() => (current = g.name)}>{g.title}</button>
    {/each}
    <p class="muted small research">Research</p>
    {#each guides.filter((g) => g.research) as g}
      <button class:on={g.name === current} onclick={() => (current = g.name)}>{g.title.split(":")[0]}</button>
    {/each}
  </nav>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <article class="md" bind:this={article} {onclick}>
    {@html html}
  </article>
</div>

<style>
  .guide {
    display: grid;
    grid-template-columns: 13rem minmax(0, 1fr);
    height: 100%;
  }
  .toc {
    border-right: 1px solid var(--line);
    padding: 1.5rem 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    overflow-y: auto;
  }
  .toc button {
    text-align: left;
    border: none;
    background: none;
    padding: 0.35rem 0.6rem;
    font-size: 0.9rem;
  }
  .toc button.on {
    background: var(--surface-2);
    font-weight: 600;
  }
  .small {
    font-size: 0.8rem;
    margin: 0 0 0.4rem 0.6rem;
  }
  .small.research {
    margin-top: 1.2rem;
  }
  .md {
    overflow-y: auto;
    padding: 2rem 2.5rem 4rem;
    max-width: 52rem;
    line-height: 1.6;
  }
  .md :global(h1) {
    margin-top: 0;
  }
  .md :global(h2) {
    margin-top: 2rem;
    border-bottom: 1px solid var(--line);
    padding-bottom: 0.2rem;
  }
  .md :global(img) {
    max-width: 100%;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .md :global(table) {
    border-collapse: collapse;
    margin: 1rem 0;
    font-size: 0.92rem;
  }
  .md :global(th),
  .md :global(td) {
    border: 1px solid var(--line);
    padding: 0.35rem 0.6rem;
    text-align: left;
    vertical-align: top;
  }
  .md :global(th) {
    background: var(--surface-2);
  }
  .md :global(code) {
    background: var(--surface-2);
    border-radius: 4px;
    padding: 0 0.3em;
    font-size: 0.9em;
  }
  .md :global(blockquote) {
    margin: 1rem 0;
    padding: 0.4rem 1rem;
    border-left: 4px solid var(--accent);
    background: var(--surface-2);
    border-radius: 0 6px 6px 0;
  }
  .md :global(a) {
    color: var(--accent);
  }
</style>
