# Halvor Web App

Svelte + TypeScript web application for Halvor, using WASM bindings from Rust.

## Development

### Local Development (Native)

```bash
# Install dependencies
npm install

# Build WASM module
make web-wasm-build

# Start dev server
npm run dev
```

### Docker Development (with hot reload)

```bash
# Start development container with watch mode
make web-dev

# Or directly:
cd halvor-web
docker-compose up halvor-web-dev
```

The dev container will:
- Watch Rust code and rebuild WASM automatically
- Watch Svelte code and hot reload
- Expose ports 5173 (Vite) and 8080 (WASM server)

## Production

### Build

```bash
# Build WASM
make web-wasm-build

# Build Svelte app
make web-build
```

### Run Production Container

```bash
make web-prod

# Or:
cd halvor-web
docker-compose up halvor-web-prod
```

## Architecture

- **Frontend**: Svelte 5 + TypeScript
- **Rust FFI**: WASM bindings via `wasm-bindgen`
- **Build**: Vite
- **Docker**: Multi-stage builds for production, dev mode with hot reload

## WASM Integration

The WASM module is built from `halvor-swift/halvor-ffi-wasm` and automatically loaded at runtime. Functions marked with `#[wasm_export]` or `#[multi_platform_export]` are available in TypeScript.

