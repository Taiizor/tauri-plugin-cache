<script>
  import Greet from './lib/Greet.svelte'
  import { set, get, hasKey, remove, clear, keys } from 'tauri-plugin-cache-api'

	let response = ''
	let key = 'test-key'
	let value = 'test-value'
	let ttlInput = '60' // string input from the form

	function updateResponse(returnValue) {
		response += `[${new Date().toLocaleTimeString()}] ` + (typeof returnValue === 'string' ? returnValue : JSON.stringify(returnValue)) + '<br>'
	}

	async function handleSetCache() {
		try {
			// Convert ttlInput to number or undefined if empty
			const ttl = ttlInput ? Number(ttlInput) : undefined
			await set(key, value, ttl)
			updateResponse(`Value "${value}" stored with key "${key}"${ttl ? ` (expires in ${ttl} seconds)` : ''}`)
		} catch (error) {
			updateResponse(`Error: ${error.toString()}`)
		}
	}

	async function handleGetCache() {
		try {
			const result = await get(key)
			updateResponse(`Get "${key}": ${result !== null ? result : 'Not found or expired'}`)
		} catch (error) {
			updateResponse(`Error: ${error.toString()}`)
		}
	}

	async function handleHasKey() {
		try {
			const result = await hasKey(key)
			updateResponse(`Has key "${key}": ${result}`)
		} catch (error) {
			updateResponse(`Error: ${error.toString()}`)
		}
	}

	async function handleRemove() {
		try {
			await remove(key)
			updateResponse(`Removed key "${key}"`)
		} catch (error) {
			updateResponse(`Error: ${error.toString()}`)
		}
	}

	async function handleClear() {
		try {
			await clear()
			updateResponse('Cache cleared')
		} catch (error) {
			updateResponse(`Error: ${error.toString()}`)
		}
	}

	async function handleListKeys() {
		try {
			const allKeys = await keys()
			updateResponse(`Cache keys: ${allKeys.length > 0 ? allKeys.join(', ') : '(empty)'}`)
		} catch (error) {
			updateResponse(`Error: ${error.toString()}`)
		}
	}
</script>

<main class="container">
  <h1>Welcome to Tauri!</h1>

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

  <p>
    Click on the Tauri, Vite, and Svelte logos to learn more.
  </p>

  <div class="row">
    <Greet />
  </div>

  <div class="cache-container">
    <h2>Cache API Test</h2>
    
    <div class="cache-inputs">
      <div class="input-group">
        <label for="cache-key">Key:</label>
        <input id="cache-key" bind:value={key} placeholder="Cache key..." />
      </div>
      
      <div class="input-group">
        <label for="cache-value">Value:</label>
        <input id="cache-value" bind:value={value} placeholder="Cache value..." />
      </div>
      
      <div class="input-group">
        <label for="cache-ttl">TTL (seconds):</label>
        <input id="cache-ttl" bind:value={ttlInput} placeholder="Time to live..." />
      </div>
    </div>
    
    <div class="cache-buttons">
      <button on:click={handleSetCache}>Set</button>
      <button on:click={handleGetCache}>Get</button>
      <button on:click={handleHasKey}>Has Key</button>
      <button on:click={handleRemove}>Remove</button>
      <button on:click={handleListKeys}>List Keys</button>
      <button on:click={handleClear}>Clear All</button>
    </div>
    
    <div class="response-container">
      <h3>Results:</h3>
      <div class="response">{@html response}</div>
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

  .cache-container {
    margin-top: 2rem;
    padding: 1rem;
    border: 1px solid #ddd;
    border-radius: 8px;
  }

  .cache-inputs {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    min-width: 150px;
  }

  .input-group label {
    margin-bottom: 0.25rem;
    font-size: 0.9rem;
  }

  .cache-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .response-container {
    margin-top: 1rem;
    padding: 1rem;
    background-color: #f5f5f5;
    border-radius: 4px;
    max-height: 300px;
    overflow-y: auto;
  }

  .response {
    font-family: monospace;
    white-space: pre-wrap;
  }
</style>
