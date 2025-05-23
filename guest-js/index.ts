import { invoke } from '@tauri-apps/api/core'

/**
 * Supported compression methods
 */
export enum CompressionMethod {
  /**
   * Zlib compression (default, balanced speed/ratio)
   */
  Zlib = 'zlib',
  /**
   * LZMA2 compression (better compression ratio, slower)
   */
  Lzma2 = 'lzma2'
}

/**
 * Interface for cache statistics
 */
export interface CacheStats {
  /**
   * Total number of items in the cache
   */
  totalSize: number;
  /**
   * Number of active (non-expired) items in the cache
   */
  activeSize: number;
}

/**
 * Options for setting a cache item
 */
export interface SetItemOptions {
  /**
   * Time-to-live in seconds. If not provided, the item will never expire.
   */
  ttl?: number;
  /**
   * Whether to compress the data before storing. If not provided, uses the default compression setting.
   */
  compress?: boolean;
  /**
   * Compression method to use. If not provided, uses the default compression method.
   */
  compressionMethod?: CompressionMethod;
}

/**
 * Sets an item in the cache with optional TTL and compression
 * @param key The key to store the value under
 * @param value The value to store
 * @param options Options for setting the cache item
 * @returns A promise that resolves when the operation is complete
 * @example
 * ```typescript
 * // Set an item with no expiration
 * await cache.set('user', { name: 'John', age: 30 });
 * 
 * // Set an item that expires in 60 seconds
 * await cache.set('token', 'abc123', { ttl: 60 });
 * 
 * // Set an item with compression enabled
 * await cache.set('largeData', largeObject, { compress: true });
 * 
 * // Set an item with both TTL and compression
 * await cache.set('temporaryData', data, { ttl: 300, compress: true });
 * 
 * // Set an item with LZMA2 compression for better compression ratio
 * await cache.set('largeTextData', largeText, { compress: true, compressionMethod: CompressionMethod.Lzma2 });
 * 
 * // Set an item with Zlib compression for faster compression
 * await cache.set('mediumData', mediumObject, { compress: true, compressionMethod: CompressionMethod.Zlib });
 * ```
 */
export async function set(key: string, value: any, options?: SetItemOptions): Promise<void> {
  await invoke('plugin:cache|set', {
    key,
    value,
    options,
  });
}

/**
 * Gets an item from the cache
 * @param key The key to retrieve
 * @returns The stored value or null if not found or expired
 * @example
 * ```typescript
 * const user = await cache.get<User>('user');
 * if (user) {
 *   console.log(user.name); // 'John'
 * }
 * ```
 */
export async function get<T = any>(key: string): Promise<T | null> {
  const result = await invoke<T | null>('plugin:cache|get', {
    key,
  });
  return result === undefined ? null : result;
}

/**
 * Checks if an item exists in the cache and is not expired
 * @param key The key to check
 * @returns True if the item exists and is not expired
 * @example
 * ```typescript
 * if (await cache.has('user')) {
 *   // User exists in cache
 * }
 * ```
 */
export async function has(key: string): Promise<boolean> {
  const response = await invoke<{ value: boolean }>('plugin:cache|has', {
    key,
  });
  return response.value;
}

/**
 * Removes an item from the cache
 * @param key The key to remove
 * @returns A promise that resolves when the operation is complete
 * @example
 * ```typescript
 * await cache.remove('user');
 * ```
 */
export async function remove(key: string): Promise<void> {
  await invoke('plugin:cache|remove', {
    key,
  });
}

/**
 * Clears all items from the cache
 * @returns A promise that resolves when the operation is complete
 * @example
 * ```typescript
 * await cache.clear();
 * ```
 */
export async function clear(): Promise<void> {
  await invoke('plugin:cache|clear');
}

/**
 * Gets statistics about the cache
 * @returns Cache statistics including the number of active and total items
 * @example
 * ```typescript
 * const stats = await cache.stats();
 * console.log(`Cache has ${stats.totalSize} items (${stats.activeSize} active)`);
 * ```
 */
export async function stats(): Promise<CacheStats> {
  return await invoke<CacheStats>('plugin:cache|stats');
}