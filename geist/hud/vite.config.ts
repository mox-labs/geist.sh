import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    lib: {
      entry: 'src/index.ts',
      formats: ['es'],
      fileName: 'index',
    },
    rollupOptions: {
      external: [/^lit/, /^@lit/],
    },
    target: 'es2022',
    minify: false,
    sourcemap: true,
  },
});
