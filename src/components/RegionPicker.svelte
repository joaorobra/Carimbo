<script lang="ts">
  import { api } from "../lib/api";
  import { i18n, t, type Lang, type Region } from "../lib/i18n";
  import Icon from "./Icon.svelte";

  // First-run gate: pick a region before anything else. US is the default and
  // pre-selected (the app is US-first); Brazil switches the UI to Portuguese and
  // dates to day-first. Choosing seeds region-appropriate example snippets and
  // records `region.chosen` so this never shows again.
  interface Props {
    /** Called once the region is chosen, examples are seeded, and settings are
        persisted — the parent then re-checks the (now non-empty) library. */
    onDone: () => void;
  }
  let { onDone }: Props = $props();

  // The language each region implies. Region drives date order; language is a
  // sensible default the user can still change later in Settings.
  const REGION_LANG: Record<Region, Lang> = { us: "en", br: "pt-BR" };

  let selected = $state<Region>("us");
  let saving = $state(false);

  const OPTIONS: { region: Region; titleKey: Parameters<typeof t>[0]; descKey: Parameters<typeof t>[0]; flag: string }[] = [
    { region: "us", titleKey: "region.us", descKey: "region.usDesc", flag: "🇺🇸" },
    { region: "br", titleKey: "region.br", descKey: "region.brDesc", flag: "🇧🇷" },
  ];

  async function choose() {
    if (saving) return;
    saving = true;
    try {
      // Set language first so the rest of the UI is already in the right
      // language once the picker dismisses, then region + the seed.
      await i18n.setLang(REGION_LANG[selected]);
      await i18n.setRegion(selected);
      await api.seedDefaultSnippets(selected);
      await api.setSetting("region.chosen", true);
      onDone();
    } catch {
      // Even if seeding fails, don't trap the user on the picker — record the
      // choice and move on; they can add snippets manually.
      await api.setSetting("region.chosen", true);
      onDone();
    }
  }
</script>

<div class="overlay">
  <div class="card" role="dialog" aria-label={t("region.title")} aria-modal="true">
    <div class="welcome">
      <span class="brand-mark"><Icon name="stamp" size={28} /></span>
      <h1>{t("region.title")}</h1>
      <p class="lead">{t("region.lead")}</p>
    </div>

    <div class="options" role="radiogroup" aria-label={t("settings.region")}>
      {#each OPTIONS as opt (opt.region)}
        <button
          class="option"
          class:selected={selected === opt.region}
          role="radio"
          aria-checked={selected === opt.region}
          onclick={() => (selected = opt.region)}
        >
          <span class="flag" aria-hidden="true">{opt.flag}</span>
          <span class="text">
            <span class="title">{t(opt.titleKey)}</span>
            <span class="desc">{t(opt.descKey)}</span>
          </span>
          <span class="check" aria-hidden="true">
            {#if selected === opt.region}<Icon name="check" size={18} />{/if}
          </span>
        </button>
      {/each}
    </div>

    <div class="actions">
      <button class="primary" disabled={saving} onclick={choose}>
        <span>{t("region.continue")}</span>
        <Icon name="check" size={16} />
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--scrim);
    z-index: 2100;
    padding: var(--space-4);
  }
  .card {
    width: 100%;
    max-width: 460px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
    padding: var(--space-6);
    max-height: 90vh;
    overflow-y: auto;
  }
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }
  .brand-mark {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
  }
  h1 {
    margin: 0;
    font-size: var(--fs-xl);
  }
  .lead {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
  .options {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .option {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition:
      border-color var(--transition-fast),
      background var(--transition-fast);
  }
  .option:hover {
    border-color: var(--accent);
  }
  .option.selected {
    border-color: var(--accent);
    background: var(--accent-weak);
  }
  .flag {
    font-size: 1.6rem;
    line-height: 1;
    flex-shrink: 0;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .title {
    font-weight: 600;
  }
  .desc {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .check {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    flex-shrink: 0;
    color: var(--accent);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-5);
  }
  .primary {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit-target);
    padding: 0 var(--space-5);
    background: var(--accent);
    color: var(--accent-text);
    border: none;
    border-radius: var(--radius-md);
    font-weight: 600;
    transition: filter var(--transition-fast);
  }
  .primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
