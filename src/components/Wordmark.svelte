<script lang="ts">
  // Brand logotype: a small stamp glyph (the "carimbo" — Portuguese for
  // rubber stamp) pressed into an ink circle, paired with the wordmark.
  // Inline SVG only — the CSP has no font-src for webfonts, so the type
  // itself is rendered in the surrounding system font, not baked into the
  // SVG as paths.
  interface Props {
    // Compact mode drops the text and shows only the mark, for tight spaces.
    compact?: boolean;
    size?: number;
  }
  let { compact = false, size = 20 }: Props = $props();
</script>

<span class="wordmark" class:compact>
  <svg
    class="mark"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <!-- Ink pad: a soft circular field the stamp presses into. -->
    <circle cx="12" cy="12" r="11" fill="var(--accent)" opacity="0.14" />
    <!-- Stamp body: an angled tag shape suggesting a pressed impression. -->
    <path
      d="M8 15.5 L12 7 L16 15.5 Z"
      fill="none"
      stroke="var(--accent)"
      stroke-width="1.6"
      stroke-linejoin="round"
      stroke-linecap="round"
    />
    <rect
      x="6.5"
      y="15.5"
      width="11"
      height="2.6"
      rx="1.3"
      fill="var(--accent)"
    />
  </svg>
  {#if !compact}
    <span class="word">Carimbo</span>
  {/if}
</span>

<style>
  .wordmark {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    line-height: 1;
    user-select: none;
  }
  .mark {
    flex-shrink: 0;
  }
  .word {
    font-size: var(--fs-lg);
    font-weight: var(--weight-heading);
    letter-spacing: var(--tracking-tight);
    color: var(--text);
    white-space: nowrap;
  }
</style>
