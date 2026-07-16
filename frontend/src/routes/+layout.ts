// No Node server exists in the Tauri webview, so SSR must be disabled;
// every route fetches its data directly via invoke() in onMount instead.
export const ssr = false;
