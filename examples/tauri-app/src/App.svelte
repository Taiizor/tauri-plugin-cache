<script>
  import Greet from './lib/Greet.svelte'
  import { set, get, has, remove, clear, stats } from 'tauri-plugin-cache-api'

	let response = ''
	let key = 'test-key'
	let value = 'test-value'
	let ttl = 10

	function updateResponse(returnValue) {
		response = `[${new Date().toLocaleTimeString()}] ` + (typeof returnValue === 'string' ? returnValue : JSON.stringify(returnValue)) + '<br>' + response
	}

	function setItem() {
		set(key, value, { ttl: parseInt(ttl) || undefined })
			.then(() => updateResponse(`Successfully set "${key}" with value: ${value}`))
			.catch(err => updateResponse(`Error: ${err.toString()}`))
	}

	function getItem() {
		get(key)
			.then(result => updateResponse(`Get "${key}": ${JSON.stringify(result)}`))
			.catch(err => updateResponse(`Error: ${err.toString()}`))
	}

	function hasItem() {
		has(key)
			.then(exists => updateResponse(`Has "${key}": ${exists}`))
			.catch(err => updateResponse(`Error: ${err.toString()}`))
	}

	function removeItem() {
		remove(key)
			.then(() => updateResponse(`Removed "${key}"`))
			.catch(err => updateResponse(`Error: ${err.toString()}`))
	}

	function clearCache() {
		clear()
			.then(() => updateResponse('Cache cleared'))
			.catch(err => updateResponse(`Error: ${err.toString()}`))
	}
	
	function getStats() {
		stats()
			.then(result => updateResponse(`Cache stats: Total items: ${result.totalSize}, Active items: ${result.activeSize}`))
			.catch(err => updateResponse(`Error: ${err.toString()}`))
	}
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

  <div class="cache-controls">
    <h2>Cache Controls</h2>
    
    <div class="input-group">
      <label for="key">Key:</label>
      <input id="key" bind:value={key} />
    </div>
    
    <div class="input-group">
      <label for="value">Value:</label>
      <input id="value" bind:value={value} />
    </div>
    
    <div class="input-group">
      <label for="ttl">TTL (seconds):</label>
      <input id="ttl" bind:value={ttl} type="number" min="0" />
    </div>
    
    <div class="button-group">
      <button on:click={setItem}>Set</button>
      <button on:click={getItem}>Get</button>
      <button on:click={hasItem}>Has</button>
      <button on:click={removeItem}>Remove</button>
      <button on:click={clearCache}>Clear All</button>
      <button on:click={getStats}>Stats</button>
    </div>
    
    <div class="response">
      <h3>Response:</h3>
      <div class="response-content">{@html response}</div>
    </div>
  </div>

</main>

<style>
  .logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
  }

  .logo.svelte:hover {
    filter: drop-shadow(0 0 2em #ff3e00);
  }

  .cache-controls {
    border: 1px solid #ddd;
    padding: 20px;
    border-radius: 8px;
    margin-top: 20px;
  }

  .input-group {
    margin-bottom: 10px;
    display: flex;
    align-items: center;
  }

  .input-group label {
    width: 120px;
    font-weight: bold;
  }

  .input-group input {
    flex: 1;
    padding: 8px;
    border: 1px solid #ddd;
    border-radius: 4px;
  }

  .button-group {
    display: flex;
    gap: 10px;
    margin-top: 15px;
    flex-wrap: wrap;
  }

  button {
    padding: 8px 16px;
    background-color: #2c2c54;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  button:hover {
    background-color: #474787;
  }

  .response {
    margin-top: 20px;
  }

  .response-content {
    background-color: #0e0b0b;
    padding: 10px;
    border-radius: 4px;
    max-height: 200px;
    overflow-y: auto;
    white-space: pre-wrap;
    font-family: monospace;
  }
</style>