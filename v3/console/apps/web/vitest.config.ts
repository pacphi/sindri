import { defineConfig } from "vitest/config";
import { resolve } from "path";

export default defineConfig({
  test: {
    globals: false,
    environment: "node",
    // Unit tests (no browser/DOM required):
    //   - utils.test.ts: formatBytes, formatUptime, formatRelativeTime, cn
    //
    // Integration tests that require a running API server (VITEST_MODE=integration):
    //   - instance-realtime.test.ts: WebSocket broadcast verification
    include:
      process.env.VITEST_MODE === "integration"
        ? ["tests/instance-realtime.test.ts"]
        : ["tests/utils.test.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "lcov"],
      include: ["src/**/*.ts", "src/**/*.tsx"],
      exclude: ["src/main.tsx", "src/routeTree.gen.ts"],
    },
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
    },
  },
});
