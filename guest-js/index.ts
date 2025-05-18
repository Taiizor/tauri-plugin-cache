import { invoke } from '@tauri-apps/api/core'

/**
 * Interface for cache statistics
 */
export interface CacheStats {
  /**
   * Number of items in the cache
   */
  size: number;
}

/**
 * Options for setting a cache item
 */
export interface SetItemOptions {
  /**
   * Time-to-live in seconds. If not provided, the item will never expire.
   */
  ttl?: number;
}

/**
 * Sets an item in the cache with optional TTL
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
 * @returns Cache statistics including the number of items
 * @example
 * ```typescript
 * const stats = await cache.stats();
 * console.log(`Cache has ${stats.size} items`);
 * ```
 */
export async function stats(): Promise<CacheStats> {
  return await invoke<CacheStats>('plugin:cache|stats');
}
