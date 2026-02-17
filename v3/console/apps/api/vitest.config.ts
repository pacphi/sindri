import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: false,
    environment: "node",
    // Unit and mock-based integration tests that run without a live server.
    // Tests that require a running server (agent-registration, auth-middleware,
    // database-operations, heartbeat-metrics, terminal-session) are tagged as
    // "integration" and run separately against a real deployment using:
    //   VITEST_MODE=integration npm test
    include:
      process.env.VITEST_MODE === "integration"
        ? [
            "tests/agent-registration.test.ts",
            "tests/auth-middleware.test.ts",
            "tests/database-operations.test.ts",
            "tests/heartbeat-metrics.test.ts",
            "tests/terminal-session.test.ts",
          ]
        : [
            "tests/instances.test.ts",
            "tests/websocket-channels.test.ts",
            "tests/deployment-wizard.test.ts",
            "tests/instance-lifecycle.test.ts",
            "tests/command-execution.test.ts",
            "tests/scheduled-tasks.test.ts",
            "tests/command-palette.test.ts",
            "tests/multi-terminal.test.ts",
            "tests/templates.test.ts",
            // Phase 3: Observability
            "tests/metrics-pipeline.test.ts",
            "tests/fleet-dashboard.test.ts",
            "tests/instance-dashboard.test.ts",
            "tests/log-aggregation.test.ts",
            "tests/alerting.test.ts",
            // Phase 4: Administration & Security
            "tests/rbac-teams.test.ts",
            "tests/extension-admin.test.ts",
            "tests/config-drift.test.ts",
            "tests/cost-tracking.test.ts",
            "tests/security-dashboard.test.ts",
          ],
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "lcov"],
      include: ["src/**/*.ts"],
      exclude: ["src/index.ts"],
    },
  },
});
