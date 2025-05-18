# Tauri Plugin Cache

This plugin provides a simple caching system for Tauri applications. It allows you to store and retrieve string values with optional expiration times (TTL).

## Installation

There are two packages needed for complete functionality:

1. The Rust plugin:
```sh
cargo add tauri-plugin-cache
```

2. The JavaScript/TypeScript API package:
```bash
npm install @taiizor/plugin-cache
# or
yarn add @taiizor/plugin-cache
# or
pnpm add @taiizor/plugin-cache
```

## Configuration

In your Tauri application, you need to initialize the plugin:

```rust
// src-tauri/src/main.rs

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_cache::init())
        // ... other plugins and configuration
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Usage

### JavaScript/TypeScript API

```typescript
import { get, set, hasKey, remove, clear, keys } from '@taiizor/plugin-cache';

// Store a value in the cache
await set('user_data', JSON.stringify({ name: 'John', age: 30 }));

// Store a value with a time-to-live (TTL) of 60 seconds
await set('session_token', 'abc123', 60);

// Retrieve a value from the cache
const userData = await get('user_data');
if (userData) {
  const user = JSON.parse(userData);
  console.log(`Hello ${user.name}!`);
}

// Check if a key exists in the cache
const hasToken = await hasKey('session_token');
console.log(`Has token: ${hasToken}`);

// Remove a specific key
await remove('session_token');

// Get all keys in the cache
const allKeys = await keys();
console.log('Cached keys:', allKeys);

// Clear the entire cache
await clear();
```

## API

### JavaScript/TypeScript API

#### `set(key: string, value: string, ttl?: number): Promise<void>`
- Stores a value in the cache
- `key`: The key to store the value under
- `value`: The string value to store
- `ttl`: Optional time-to-live in seconds. If omitted, the value will not expire.

#### `get(key: string): Promise<string | null>`
- Retrieves a value from the cache
- `key`: The key to retrieve
- Returns the stored value if it exists and hasn't expired, otherwise null

#### `hasKey(key: string): Promise<boolean>`
- Checks if a key exists in the cache and hasn't expired
- `key`: The key to check
- Returns true if the key exists and hasn't expired, otherwise false

#### `remove(key: string): Promise<void>`
- Removes a value from the cache
- `key`: The key to remove

#### `clear(): Promise<void>`
- Clears all values from the cache

#### `keys(): Promise<string[]>`
- Gets all keys in the cache that haven't expired
- Returns an array of keys

## Permissions

This plugin requires the following permissions:

```json
{
  "permissions": [
    "cache:default"
  ]
}
```

For more fine-grained control:

```json
{
  "permissions": [
    "cache:allow-set",
    "cache:allow-get",
    "cache:allow-haskey",
    "cache:allow-remove",
    "cache:allow-clear",
    "cache:allow-keys"
  ]
}
```

## Storage

By default, the cache is stored in a JSON file in the application's config directory:
- Windows: `%APPDATA%\{app-name}\cache\cache.json`
- macOS: `~/Library/Application Support/{app-name}/cache/cache.json`
- Linux: `~/.config/{app-name}/cache/cache.json`

## License

MIT or Apache-2.0 at your option.