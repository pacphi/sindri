import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.js'],
    coverage: {
      reporter: ['text', 'json'],
      include: ['src/**/*.js'],
      exclude: ['src/**/*.test.js'],
    },
  },
});
