# Legere frontend

The SvelteKit (Svelte 5 runes, TypeScript, static-adapter) UI served inside
Legere's Tauri webview. See the [root README](../README.md) for development
commands and [ARCHITECTURE.md](../ARCHITECTURE.md) for how the frontend
talks to the Rust backend (Tauri commands, backend events, the
`legere-content://` token resolution).

Run frontend checks from the repo root:

```sh
npx --prefix frontend svelte-check --tsconfig ./frontend/tsconfig.json
```
