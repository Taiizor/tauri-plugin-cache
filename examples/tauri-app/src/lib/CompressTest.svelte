<script>
  import { set, get, stats } from 'tauri-plugin-cache-api';
  
  let testResults = "";
  let isRunning = false;
  
  async function runCompressionTest() {
    if (isRunning) return;
    
    isRunning = true;
    testResults = "Starting...\n";
    
    try {
      // Create large data for testing
      const largeData = generateLargeData();
      const dataSize = new TextEncoder().encode(JSON.stringify(largeData)).length;
      testResults += `Test data size: ${formatSize(dataSize)}\n`;
      
      // First save without compression
      const uncompressedKey = "test-uncompressed";
      const startUncompressed = performance.now();
      await set(uncompressedKey, largeData, { compress: false });
      const endUncompressed = performance.now();
      testResults += `Uncompressed save time: ${(endUncompressed - startUncompressed).toFixed(2)} ms\n`;
      
      // Then save with compression
      const compressedKey = "test-compressed";
      const startCompressed = performance.now();
      await set(compressedKey, largeData, { compress: true });
      const endCompressed = performance.now();
      testResults += `Compressed save time: ${(endCompressed - startCompressed).toFixed(2)} ms\n`;
      
      // Get cache statistics
      const cacheStats = await stats();
      testResults += `Total items in cache: ${cacheStats.totalSize}\n`;
      
      // Read both records and measure time
      const startReadUncompressed = performance.now();
      await get(uncompressedKey);
      const endReadUncompressed = performance.now();
      testResults += `Uncompressed read time: ${(endReadUncompressed - startReadUncompressed).toFixed(2)} ms\n`;
      
      const startReadCompressed = performance.now();
      await get(compressedKey);
      const endReadCompressed = performance.now();
      testResults += `Compressed read time: ${(endReadCompressed - startReadCompressed).toFixed(2)} ms\n`;
      
      testResults += "\nTest completed! Compression can be effective in reducing data size, but may increase processing time.";
      
    } catch (error) {
      testResults += `Error: ${error.message}`;
    }
    
    isRunning = false;
  }
  
  // Create test data with 1000 elements
  function generateLargeData() {
    return Array.from({ length: 1000 }, (_, i) => ({
      id: i,
      name: `Item ${i}`,
      description: `This is a test description for item ${i} to demonstrate compression efficiency. Compression works best with repetitive content. This text is somewhat longer and filled with the same expressions. Compressible data, compressible data, compressible data, compressible data...`,
      metadata: {
        timestamp: Date.now(),
        tags: ["test", "compression", "example", "tauri", "cache", "plugin"],
        isActive: i % 2 === 0,
        priority: Math.floor(Math.random() * 100),
        nested: {
          level1: {
            level2: {
              level3: "Deeply nested content"
            }
          }
        }
      }
    }));
  }
  
  function formatSize(bytes) {
    if (bytes < 1024) return `${bytes} bytes`;
    else if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    else return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
</script>

<div class="compression-test">
  <h2>Compression Test</h2>
  <p>This test measures the effect of compression on performance and size.</p>
  
  <button on:click={runCompressionTest} disabled={isRunning}>
    {isRunning ? 'Test running...' : 'Start Compression Test'}
  </button>
  
  {#if testResults}
    <div class="test-results">
      <h3>Test Results:</h3>
      <pre>{testResults}</pre>
    </div>
  {/if}
</div>

<style>
  /* Theme variables use CSS custom properties for theme support */
  :root {
    --test-border-color: var(--border-color, #ddd);
    --test-background: var(--background, white);
    --button-bg: var(--primary-color, #2c2c54);
    --button-hover-bg: var(--primary-hover, #474787);
    --button-disabled-bg: var(--disabled-color, #666);
    --button-text-color: var(--button-text, white);
    --results-bg: var(--surface-alt, #f5f5f5);
    --text-color: var(--text, #333);
  }

  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    :root {
      --test-border-color: #444;
      --test-background: #222;
      --button-bg: #536dfe;
      --button-hover-bg: #7986cb;
      --button-disabled-bg: #454545;
      --button-text-color: white;
      --results-bg: #2d2d2d;
      --text-color: #eee;
    }
  }
  
  .compression-test {
    padding: 1rem;
    border: 1px solid var(--test-border-color);
    border-radius: 8px;
    margin-top: 2rem;
    margin-bottom: 2rem;
    background-color: var(--test-background);
    color: var(--text-color);
  }
  
  h2 {
    margin-top: 0;
    color: var(--text-color);
  }
  
  button {
    padding: 8px 16px;
    background-color: var(--button-bg);
    color: var(--button-text-color);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s;
  }
  
  button:disabled {
    background-color: var(--button-disabled-bg);
    cursor: not-allowed;
  }
  
  button:hover:not(:disabled) {
    background-color: var(--button-hover-bg);
  }
  
  .test-results {
    margin-top: 1rem;
    background-color: var(--results-bg);
    padding: 1rem;
    border-radius: 4px;
    color: var(--text-color);
  }
  
  pre {
    white-space: pre-wrap;
    word-break: break-word;
    font-family: monospace;
    font-size: 0.9rem;
    margin: 0;
    color: var(--text-color);
  }
</style> 