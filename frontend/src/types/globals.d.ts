/**
 * Global type declarations for Vite-defined constants and environment
 * These are defined in vite.config.ts using the define option
 */

declare const __DEV__: boolean;
declare const __PROD__: boolean;
declare const __APP_VERSION__: string;
declare const __BUILD_DATE__: string;

/**
 * Vite import.meta.env type declarations
 */
interface ImportMetaEnv {
  readonly MODE: string;
  readonly BASE_URL: string;
  readonly PROD: boolean;
  readonly DEV: boolean;
  readonly SSR: boolean;
  // Add any other environment variables you use
  [key: string]: string | boolean | undefined;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
