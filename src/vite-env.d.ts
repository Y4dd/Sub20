/// <reference types="svelte" />
/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "web" for Sub20 Lite (`npm run build:web`); unset for the desktop app. */
  readonly VITE_TARGET?: "web";
}
