# Tauri Plugin Cache Example App

This is an example Tauri application demonstrating the capabilities of `tauri-plugin-cache`, built with Svelte 5 and Vite.

## Features Demonstrated

- **Basic Cache Operations**: Set, Get, Has, Remove, and Clear operations.
- **TTL (Time-To-Live)**: Cache expiration management.
- **Compression Benchmarks & Comparison**: Side-by-side performance comparison for Uncompressed, Zlib, LZMA2, and Zstd compression methods.
- **Item Details & Compression Metrics**: Using `details(key)` to inspect original size, compressed size, compression method, and space savings ratio.
- **Cache Statistics**: View total and active cache items via `stats()`.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/)
- [Bun](https://bun.sh/) (recommended)

### Installation

Navigate to the example app directory and install dependencies:

```bash
# Using bun (recommended)
bun install

# Using npm
npm install

# Using pnpm
pnpm install

# Using yarn
yarn install
```

### Running the Example

Run the desktop application in development mode:

```bash
# Using bun (recommended)
bun run tauri dev

# Using npm
npm run tauri dev

# Using pnpm
pnpm tauri dev

# Using yarn
yarn tauri dev
```

### Building for Production

```bash
# Using bun (recommended)
bun run tauri build

# Using npm
npm run tauri build

# Using pnpm
pnpm tauri build

# Using yarn
yarn tauri build
```

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
