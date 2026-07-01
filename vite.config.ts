import { sveltekit } from '@sveltejs/kit/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(({ mode }) => ({
  // svelteTesting adds the 'browser' resolve condition for the jsdom test
  // env; scope it to test mode only so it never alters the app build.
  plugins: [sveltekit(), ...(mode === 'test' ? [svelteTesting()] : [])],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 5183,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.{test,spec}.{js,ts,svelte}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      // Measure only first-party source. Without an explicit include, v8 also
      // counts Vite/SvelteKit cache artifacts (e.g. hashed `out/*api-script.js`)
      // which are not our code and falsely depress the ratio.
      include: ['src/**/*.{ts,svelte}'],
      exclude: [
        'src/**/*.{test,spec}.{js,ts,svelte}',
        'src/**/*.d.ts',
        // Barrel re-exports — no runtime logic to cover.
        'src/**/index.ts',
        // Type-only module: interfaces compile away, nothing to execute.
        'src/lib/types.ts',
        // SvelteKit framework shell (load fn + reduced-motion bootstrap); the
        // app logic lives in +page.svelte, which is covered.
        'src/routes/+layout.*',
      ],
      thresholds: {
        lines: 70,
        functions: 70,
        branches: 70,
        statements: 70,
      },
    },
  },
}));
