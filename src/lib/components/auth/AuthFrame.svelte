<script lang="ts">
  interface Props {
    tilt?: number; // rotation in deg, default 0
    children: import('svelte').Snippet;
  }
  let { tilt = 0, children }: Props = $props();
</script>

<div class="page">
  <div class="frame" style:transform="rotate({tilt}deg)">
    {@render children()}
  </div>
</div>

<style>
  .page {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #f8f1e7;
    padding: 2rem 1rem;
  }

  .frame {
    width: 100%;
    max-width: 480px;
    background: #fdf8f2;
    border: 1px solid #d8c6b1;
    outline: 3px solid #f8f1e7;
    outline-offset: -6px;
    padding: 2.5rem 2rem;
    font-family: Georgia, serif;
    color: #34251c;
  }

  /* ── Shared auth styles exported as :global so host pages can use them ── */
  :global(.auth-title) {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.6rem;
    font-weight: 400;
    letter-spacing: 0.04em;
    margin: 0 0 0.5rem;
    color: #34251c;
  }
  :global(.auth-hint) {
    font-size: 0.78rem;
    color: #9a7c5c;
    margin: 0 0 1.2rem;
    font-family: Inter, sans-serif;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  :global(.auth-choose) {
    font-size: 0.85rem;
    color: #6f3b24;
    margin: 0 0 1rem;
    font-style: italic;
  }
  :global(.auth-progress) {
    display: flex;
    gap: 6px;
    margin-bottom: 1.5rem;
  }
  :global(.auth-pip) {
    height: 3px;
    flex: 1;
    background: #d8c6b1;
    border-radius: 2px;
    transition: background 0.3s;
  }
  :global(.auth-pip.active) { background: #c65f3c; }
  :global(.auth-pip.done)   { background: #6f3b24; }
  :global(.auth-fields) {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  :global(.auth-field) {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  :global(.auth-field span) {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #6f3b24;
    font-family: Inter, sans-serif;
  }
  :global(.auth-field input) {
    background: transparent;
    border: none;
    border-bottom: 1px solid #d8c6b1;
    padding: 0.4rem 0;
    font-family: Georgia, serif;
    font-size: 1rem;
    color: #34251c;
    outline: none;
    transition: border-color 0.2s;
    width: 100%;
  }
  :global(.auth-field input:focus) { border-bottom-color: #c65f3c; }
  :global(.auth-grid) {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }
  :global(.auth-icon-btn) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid #d8c6b1;
    padding: 0.75rem 0.4rem;
    cursor: pointer;
    color: #6f3b24;
    transition: all 0.2s;
    border-radius: 2px;
  }
  :global(.auth-icon-btn:hover) {
    background: #f0e6d6;
    border-color: #c65f3c;
  }
  :global(.auth-icon-btn.selected) {
    background: #6f3b24;
    border-color: #6f3b24;
    color: #f8f1e7;
  }
  :global(.auth-icon-btn svg) { width: 32px; height: 32px; }
  :global(.auth-icon-label) {
    font-size: 0.6rem;
    font-family: Inter, sans-serif;
    letter-spacing: 0.04em;
    text-align: center;
    line-height: 1.2;
  }
  :global(.auth-nav) {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
    margin-top: 1rem;
  }
  :global(.auth-btn-primary) {
    background: #34251c;
    color: #f8f1e7;
    border: none;
    padding: 0.6rem 1.5rem;
    font-family: Georgia, serif;
    font-size: 0.9rem;
    cursor: pointer;
    letter-spacing: 0.06em;
    transition: background 0.2s;
  }
  :global(.auth-btn-primary:hover:not(:disabled)) { background: #6f3b24; }
  :global(.auth-btn-primary:disabled) { opacity: 0.5; cursor: not-allowed; }
  :global(.auth-btn-ghost) {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #6f3b24;
    padding: 0.6rem 1rem;
    font-family: Georgia, serif;
    font-size: 0.9rem;
    cursor: pointer;
    transition: border-color 0.2s;
  }
  :global(.auth-btn-ghost:hover) { border-color: #6f3b24; }
  :global(.auth-error) {
    font-size: 0.82rem;
    color: #c65f3c;
    margin: 0.5rem 0 0;
    font-family: Inter, sans-serif;
  }
  :global(.auth-switch) {
    margin-top: 1.5rem;
    font-size: 0.8rem;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
    text-align: center;
  }
  :global(.auth-link) {
    background: none;
    border: none;
    color: #c65f3c;
    cursor: pointer;
    font-size: inherit;
    padding: 0;
    text-decoration: underline;
    font-family: inherit;
  }
</style>
