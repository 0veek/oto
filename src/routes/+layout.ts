// Tauri doesn't have a Node.js server to do proper SSR.
// Prerender routes so dual windows (`/` and `/settings`) have static HTML
// that the Tauri asset protocol can load directly.
// See: https://v2.tauri.app/start/frontend/sveltekit/
export const ssr = false;
export const prerender = true;
