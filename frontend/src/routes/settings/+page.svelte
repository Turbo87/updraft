<script lang="ts">
  import { resolve } from '$app/paths';

  import { m } from '$lib/paraglide/messages.js';
</script>

<main>
  <h1>{m.settings_heading()}</h1>
  <nav aria-label={m.settings_heading()}>
    <ul>
      <li><a href={resolve('/settings/language')}>{m.language_label()}</a></li>
      <li><a href={resolve('/settings/units')}>{m.units_label()}</a></li>
      <li><a href={resolve('/settings/airspace')}>{m.airspace_label()}</a></li>
      <li><a href={resolve('/settings/devices')}>{m.external_devices_heading()}</a></li>
      <li><a href={resolve('/settings/about')}>{m.about_heading()}</a></li>
    </ul>
  </nav>
  <a class="back-link" href={resolve('/')}>{m.back_to_flight_view()}</a>
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  h1 {
    margin-block-start: 0;
  }

  ul {
    display: grid;
    max-width: 30rem;
    margin: 0;
    padding: 0;
    gap: 0.75rem;
    list-style: none;
  }

  nav a {
    display: block;
    padding: 1rem;
    border: 0.0625rem solid light-dark(var(--color-gray-300), var(--color-gray-700));
    border-radius: 0.5rem;
  }

  .back-link {
    display: inline-block;
    margin-block-start: 2rem;
  }
</style>
