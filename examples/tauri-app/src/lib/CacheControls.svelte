<script>
  import { set, get, has, remove, clear, stats } from 'tauri-plugin-cache-api'

	let response = ''
	let key = 'test-key'
	let value = 'test-value'
	let ttl = '10'
	let compress = false

	function updateResponse(returnValue) {
		response = `[${new Date().toLocaleTimeString()}] ` + (typeof returnValue === 'string' ? returnValue : JSON.stringify(returnValue)) + '<br>' + response
	}

	function setItem() {
		const ttlValue = ttl !== '' ? parseInt(ttl) : undefined;
		
		set(key, value, { 
			ttl: ttlValue,
			compress: compress
		})
			.then(() => updateResponse(`Successfully set "${key}" with value: ${value}${compress ? ' (compressed)' : ''}`))
			.catch(err => updateResponse(`Error: ${err.toString()}`))
	}

	// Function to generate large data - to demonstrate the effect of compression
	function generateLargeData() {
		// Create an array with 1000 elements
		const largeArray = Array.from({ length: 1000 }, (_, i) => ({
			id: i,
			name: `Item ${i}`,
			description: `This is a test description for item ${i} that has some repetitive text to demonstrate compression efficiency. Compression works best with repetitive content.`
		}));
		
		value = JSON.stringify(largeArray);
		updateResponse(`Generated large data (${(value.length / 1024).toFixed(2)} KB)`);
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

<div class="cache-controls">
  <h2>Cache Controls</h2>
  
  <div class="input-group">
    <label for="key">Key:</label>
    <input id="key" bind:value={key} />
  </div>
  
  <div class="input-group">
    <label for="value">Value:</label>
    <textarea id="value" bind:value={value} rows="4"></textarea>
    <button class="generate-btn" on:click={generateLargeData}>Generate Test Data</button>
  </div>
  
  <div class="input-group">
    <label for="ttl">TTL (seconds):</label>
    <input id="ttl" bind:value={ttl} type="number" min="0" />
  </div>
  
  <div class="input-group toggle-group">
    <div class="toggle-label-container">
      <label for="compress">Compress data:</label>
      <div class="toggle-wrapper">
        <input id="compress" type="checkbox" class="toggle-input" bind:checked={compress} />
        <label for="compress" class="toggle-label"></label>
      </div>
    </div>
    <div class="tooltip">Compression is recommended for large text data to save disk space</div>
  </div>
  
  <div class="button-group">
    <button on:click={setItem}>Set</button>
    <button on:click={getItem}>Get</button>
    <button on:click={hasItem}>Has</button>
    <button on:click={removeItem}>Remove</button>
    <button on:click={clearCache}>Clear All</button>
    <button on:click={getStats}>Stats</button>
  </div>
  
  {#if response}
    <div class="response">
        <h3>Response:</h3>
        <div class="response-content">{@html response}</div>
    </div>
  {/if}
</div>

<style>
  /* Theme variables for light/dark mode support */
  :root {
    --border-color: #ddd;
    --background: white;
    --background-alt: #f5f5f5;
    --primary-color: #2c2c54;
    --primary-hover: #474787;
    --disabled-color: #666;
    --button-text: white;
    --text: #333;
    --text-light: #666;
    --surface-alt: #f5f5f5;
    --console-bg: #0e0b0b;
    --console-text: #f1f1f1;
    --toggle-off: #ccc;
    --toggle-on: #536dfe;
    --toggle-on-shadow: rgba(83, 109, 254, 0.5);
    --toggle-off-shadow: rgba(0, 0, 0, 0.1);
  }

  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    :root {
      --border-color: #444;
      --background: #1a1a1a;
      --background-alt: #2d2d2d;
      --primary-color: #536dfe;
      --primary-hover: #7986cb;
      --disabled-color: #454545;
      --button-text: white;
      --text: #eee;
      --text-light: #aaa;
      --surface-alt: #2d2d2d;
      --console-bg: #10192f;
      --console-text: #f1f1f1;
      --toggle-off: #555;
      --toggle-on: #536dfe;
      --toggle-on-shadow: rgba(83, 109, 254, 0.5);
      --toggle-off-shadow: rgba(0, 0, 0, 0.3);
    }
  }

  /* Apply theme variables to styles */
  :global(body) {
    background-color: var(--background);
    color: var(--text);
  }

  .cache-controls {
    border: 1px solid var(--border-color);
    padding: 20px;
    border-radius: 8px;
    margin-top: 20px;
    background-color: var(--background-alt);
  }

  .input-group {
    margin-bottom: 10px;
    display: flex;
    align-items: center;
  }

  .input-group > label {
    width: 120px;
    font-weight: bold;
    color: var(--text);
  }

  .input-group input, .input-group textarea {
    flex: 1;
    padding: 8px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background-color: var(--background);
    color: var(--text);
  }
  
  /* Modern toggle switch */
  .toggle-group {
    flex-direction: column;
    align-items: flex-start;
  }
  
  .toggle-label-container {
    display: flex;
    align-items: center;
    margin-bottom: 5px;
    width: 100%;
  }
  
  .toggle-label-container > label {
    width: 120px;
    font-weight: bold;
    color: var(--text);
  }
  
  .toggle-wrapper {
    position: relative;
    display: inline-block;
    width: 50px;
    height: 24px;
  }
  
  .toggle-input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  
  .toggle-label {
    display: block;
    position: relative;
    width: 50px;
    height: 24px;
    background-color: var(--toggle-off);
    border-radius: 24px;
    cursor: pointer;
    transition: background-color 0.3s, box-shadow 0.3s;
    box-shadow: 0 1px 3px var(--toggle-off-shadow);
  }
  
  .toggle-label:before {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background-color: white;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: transform 0.3s, background-color 0.3s;
  }
  
  .toggle-input:checked + .toggle-label {
    background-color: var(--toggle-on);
    box-shadow: 0 1px 5px var(--toggle-on-shadow);
  }
  
  .toggle-input:checked + .toggle-label:before {
    transform: translateX(26px);
  }
  
  .toggle-input:focus + .toggle-label {
    box-shadow: 0 0 0 2px var(--toggle-on-shadow);
  }
  
  .input-group textarea {
    font-family: monospace;
    font-size: 12px;
    resize: vertical;
  }
  
  .tooltip {
    font-size: 0.8em;
    color: var(--text-light);
    margin-left: 120px;
    font-style: italic;
  }
  
  .generate-btn {
    margin-left: 10px;
    padding: 4px 8px;
    font-size: 0.8em;
  }

  .button-group {
    display: flex;
    gap: 10px;
    margin-top: 15px;
    flex-wrap: wrap;
  }

  button {
    padding: 8px 16px;
    background-color: var(--primary-color);
    color: var(--button-text);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  button:hover {
    background-color: var(--primary-hover);
  }

  .response {
    margin-top: 20px;
  }

  .response-content {
    background-color: var(--console-bg);
    padding: 10px;
    border-radius: 4px;
    max-height: 200px;
    overflow-y: auto;
    white-space: pre-wrap;
    font-family: monospace;
    color: var(--console-text);
  }

  h2, h3 {
    color: var(--text);
  }
</style> 