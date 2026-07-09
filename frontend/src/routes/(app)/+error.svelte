<script lang="ts">
  import { page } from '$app/state';
</script>

<!-- Rendered inside the dialog panel when a screen's load fails (e.g. an unknown
     feature in a shared "what's here?" link). The chrome's title falls back to a
     generic error label and Close returns to the map. -->
<p class="status">{page.status}</p>
<p class="message">{page.error?.message}</p>

<style>
  .status {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
  }
  .message {
    margin: 0.25rem 0 0;
    color: #595959;
  }
  @media (prefers-color-scheme: dark) {
    .message {
      color: #b0b0b0;
    }
  }
</style>
