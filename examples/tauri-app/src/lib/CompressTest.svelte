<script>
  import { set, get, stats, CompressionMethod } from 'tauri-plugin-cache-api';
  
  let testResults = "";
  let isRunning = false;
  
  async function runCompressionTest() {
    if (isRunning) return;
    
    isRunning = true;
    testResults = "Starting compression method comparison test...\n";
    
    try {
      // Create large data for testing
      const largeData = generateLargeData();
      const dataSize = new TextEncoder().encode(JSON.stringify(largeData)).length;
      testResults += `Test data size: ${formatSize(dataSize)}\n\n`;
      
      // First save without compression
      const uncompressedKey = "test-uncompressed";
      const startUncompressed = performance.now();
      await set(uncompressedKey, largeData, { compress: false });
      const endUncompressed = performance.now();
      testResults += `Uncompressed save time: ${(endUncompressed - startUncompressed).toFixed(2)} ms\n`;
      
      // Test with Zlib compression
      const zlibKey = "test-zlib";
      const startZlib = performance.now();
      await set(zlibKey, largeData, { 
        compress: true,
        compressionMethod: CompressionMethod.Zlib 
      });
      const endZlib = performance.now();
      testResults += `Zlib compression save time: ${(endZlib - startZlib).toFixed(2)} ms\n`;
      
      // Test with LZMA2 compression
      const lzma2Key = "test-lzma2";
      const startLzma2 = performance.now();
      await set(lzma2Key, largeData, { 
        compress: true,
        compressionMethod: CompressionMethod.Lzma2 
      });
      const endLzma2 = performance.now();
      testResults += `LZMA2 compression save time: ${(endLzma2 - startLzma2).toFixed(2)} ms\n\n`;
      
      // Get cache statistics
      const cacheStats = await stats();
      testResults += `Total items in cache: ${cacheStats.totalSize}\n\n`;
      
      // Read performance tests
      testResults += `--- Reading Performance ---\n`;
      
      // Read uncompressed
      const startReadUncompressed = performance.now();
      await get(uncompressedKey);
      const endReadUncompressed = performance.now();
      testResults += `Uncompressed read time: ${(endReadUncompressed - startReadUncompressed).toFixed(2)} ms\n`;
      
      // Read Zlib compressed
      const startReadZlib = performance.now();
      await get(zlibKey);
      const endReadZlib = performance.now();
      testResults += `Zlib compressed read time: ${(endReadZlib - startReadZlib).toFixed(2)} ms\n`;
      
      // Read LZMA2 compressed
      const startReadLzma2 = performance.now();
      await get(lzma2Key);
      const endReadLzma2 = performance.now();
      testResults += `LZMA2 compressed read time: ${(endReadLzma2 - startReadLzma2).toFixed(2)} ms\n\n`;
      
      testResults += `--- Compression Efficiency Comparison ---\n`;
      testResults += `Data Type: JSON with repetitive content\n`;
      testResults += `Zlib: Fast compression/decompression, good for general use\n`;
      testResults += `LZMA2: Better compression ratio, slower, best for text and base64 data\n`;
      
      testResults += "\nTest completed! For optimal performance, choose Zlib for frequently changing data and LZMA2 for maximum space savings of rarely changed data.";
      
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
  
  // Test a specific compression method
  async function testSpecificMethod(method) {
    if (isRunning) return;
    
    isRunning = true;
    testResults = `Starting ${method} compression test...\n`;
    
    try {
      // Create large data for testing
      const largeData = generateLargeData();
      const dataSize = new TextEncoder().encode(JSON.stringify(largeData)).length;
      testResults += `Test data size: ${formatSize(dataSize)}\n\n`;
      
      // Get correct compression method
      const compressionMethod = method === 'zlib' 
        ? CompressionMethod.Zlib 
        : CompressionMethod.Lzma2;
      
      // Save with specified compression
      const compressedKey = `test-${method}`;
      const startCompressed = performance.now();
      await set(compressedKey, largeData, { 
        compress: true,
        compressionMethod: compressionMethod
      });
      const endCompressed = performance.now();
      testResults += `${method.toUpperCase()} compression time: ${(endCompressed - startCompressed).toFixed(2)} ms\n`;
      
      // Read performance
      const startReadCompressed = performance.now();
      await get(compressedKey);
      const endReadCompressed = performance.now();
      testResults += `${method.toUpperCase()} read time: ${(endReadCompressed - startReadCompressed).toFixed(2)} ms\n`;
      
      testResults += `\nTest completed for ${method.toUpperCase()} compression.`;
      
    } catch (error) {
      testResults += `Error: ${error.message}`;
    }
    
    isRunning = false;
  }
</script>

<div class="compression-test">
  <h2>Compression Test</h2>
  <p>This test measures the effect of compression on performance and size.</p>
  
  <div class="test-buttons">
    <button on:click={runCompressionTest} disabled={isRunning} class="main-btn">
      {isRunning ? 'Test running...' : 'Run Comprehensive Test'}
    </button>
    
    <div class="specific-test-buttons">
      <button on:click={() => testSpecificMethod('zlib')} disabled={isRunning} class="method-btn zlib-btn">
        <span class="method-name">Zlib</span>
        <span class="method-desc">Test Fast Compression</span>
      </button>
      
      <button on:click={() => testSpecificMethod('lzma2')} disabled={isRunning} class="method-btn lzma-btn">
        <span class="method-name">LZMA2</span>
        <span class="method-desc">Test High Ratio Compression</span>
      </button>
    </div>
  </div>
  
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
    --zlib-color: #536dfe;
    --zlib-hover: #758ffe;
    --lzma-color: #6200ee;
    --lzma-hover: #7722ff;
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
      --zlib-color: #536dfe;
      --zlib-hover: #758ffe;
      --lzma-color: #bb86fc;
      --lzma-hover: #cda5fd;
    }
  }
  
  .compression-test {
    padding: 1.5rem;
    border: 1px solid var(--test-border-color);
    border-radius: 8px;
    margin-top: 2rem;
    margin-bottom: 2rem;
    background-color: var(--test-background);
    color: var(--text-color);
    text-align: center;
  }
  
  h2 {
    margin-top: 0;
    color: var(--text-color);
    margin-bottom: 0.5rem;
  }
  
  p {
    margin-bottom: 1.5rem;
    color: var(--text-light, #666);
  }
  
  .test-buttons {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }
  
  .main-btn {
    width: 100%;
    max-width: 500px;
    font-weight: bold;
    padding: 12px 24px;
    font-size: 1.1rem;
    background-color: var(--button-bg);
    color: var(--button-text-color);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.2);
  }
  
  .main-btn:hover:not(:disabled) {
    background-color: var(--button-hover-bg);
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
  }
  
  .specific-test-buttons {
    display: flex;
    gap: 1rem;
    width: 100%;
    max-width: 500px;
    justify-content: space-between;
  }
  
  .method-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 12px 8px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s ease;
    color: white;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.15);
  }
  
  .zlib-btn {
    background-color: var(--zlib-color);
  }
  
  .zlib-btn:hover:not(:disabled) {
    background-color: var(--zlib-hover);
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
  }
  
  .lzma-btn {
    background-color: var(--lzma-color);
  }
  
  .lzma-btn:hover:not(:disabled) {
    background-color: var(--lzma-hover);
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
  }
  
  .method-name {
    font-weight: bold;
    font-size: 1rem;
    margin-bottom: 4px;
  }
  
  .method-desc {
    font-size: 0.8rem;
    opacity: 0.9;
  }
  
  button:disabled {
    background-color: var(--button-disabled-bg);
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }
  
  .test-results {
    margin-top: 1.5rem;
    background-color: var(--results-bg);
    padding: 1rem;
    border-radius: 6px;
    color: var(--text-color);
    text-align: left;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.2);
  }
  
  pre {
    white-space: pre-wrap;
    word-break: break-word;
    font-family: monospace;
    font-size: 0.9rem;
    margin: 0;
    color: var(--text-color);
    max-height: 400px;
    overflow-y: auto;
  }
  
  /* Responsive layout */
  @media (max-width: 500px) {
    .specific-test-buttons {
      flex-direction: column;
    }
  }
</style> 