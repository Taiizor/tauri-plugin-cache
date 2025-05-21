<script>
  import Greet from './lib/Greet.svelte'
  import CompressTest from './lib/CompressTest.svelte'
  import CacheControls from './lib/CacheControls.svelte'
</script>

<main class="container">
  <h1>Tauri Cache Plugin Demo</h1>

  <div class="row">
    <a href="https://vite.dev" target="_blank">
      <img src="/vite.svg" class="logo vite" alt="Vite Logo" />
    </a>
    <a href="https://tauri.app" target="_blank">
      <img src="/tauri.svg" class="logo tauri" alt="Tauri Logo" />
    </a>
    <a href="https://svelte.dev" target="_blank">
      <img src="/svelte.svg" class="logo svelte" alt="Svelte Logo" />
    </a>
  </div>

  <div class="row">
    <Greet />
  </div>

<!-- -->
  <CacheControls />

  <!-- Add the compression test component -->
  <CompressTest />

</main>

<style>
  .logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
  }

  .logo.svelte:hover {
    filter: drop-shadow(0 0 2em #ff3e00);
  }
</style>