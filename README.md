# Forge

Forge is a developer tool for note-taking and code management, built with Tauri and React.

## Project Structure

- `src-ui`: Frontend application (React + Vite + TypeScript)
- `src-tauri`: Backend application (Rust + Tauri)

## Getting Started

### Prerequisites

- Node.js
- Rust & Cargo
- Tauri CLI

### Development

1. Install dependencies:
   ```bash
   cd src-ui
   npm install
   ```

2. Run development server:
   ```bash
   # From project root
   cargo tauri dev
   ```

## Build

```bash
# From project root
cargo tauri build
```
