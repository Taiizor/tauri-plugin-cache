import { invoke } from '@tauri-apps/api/core'

/**
 * Sets a value in the cache with an optional TTL (time-to-live in seconds)
 * @param key The key to store the value under
 * @param value The value to store
 * @param ttl Optional time-to-live in seconds
 */
export async function set(key: string, value: string, ttl?: number): Promise<void> {
  return await invoke<void>('plugin:cache|set', {
    request: {
      key,
      value,
      ttl
    }
  });
}

/**
 * Gets a value from the cache
 * @param key The key to retrieve the value for
 * @returns The value if it exists and has not expired, otherwise null
 */
export async function get(key: string): Promise<string | null> {
  return await invoke<{ value?: string, exists: boolean }>('plugin:cache|get', {
    request: { key }
  }).then((response) => response.value ?? null);
}

/**
 * Checks if a key exists in the cache and has not expired
 * @param key The key to check
 * @returns True if the key exists and has not expired, otherwise false
 */
export async function hasKey(key: string): Promise<boolean> {
  return await invoke<{ exists: boolean }>('plugin:cache|has-key', {
    request: { key }
  }).then((response) => response.exists);
}

/**
 * Removes a value from the cache
 * @param key The key to remove
 */
export async function remove(key: string): Promise<void> {
  return await invoke<void>('plugin:cache|remove', {
    request: { key }
  });
}

/**
 * Clears all values from the cache
 */
export async function clear(): Promise<void> {
  return await invoke<void>('plugin:cache|clear', {});
}

/**
 * Gets all keys in the cache that have not expired
 * @returns Array of keys
 */
export async function keys(): Promise<string[]> {
  return await invoke<{ keys: string[] }>('plugin:cache|keys', {}).then((response) => response.keys);
}

export default {
  set,
  get,
  hasKey,
  remove,
  clear,
  keys
};
