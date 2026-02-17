// Root ESLint flat config (ESLint 9+).
// Individual apps can extend this or override rules as needed.
import js from "@eslint/js";

/** @type {import("eslint").Linter.Config[]} */
export default [
  // Base recommended rules for all JS/TS files
  js.configs.recommended,

  {
    // Apply to all TypeScript and TSX source files
    files: ["apps/*/src/**/*.{ts,tsx}", "packages/*/src/**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
    },
    rules: {
      // Catch common bugs
      "no-unused-vars": "off", // TypeScript handles this better
      "no-console": ["warn", { allow: ["warn", "error"] }],
      "no-debugger": "error",
      "no-duplicate-imports": "error",
      "prefer-const": "error",
      "no-var": "error",
    },
  },

  {
    // Ignore generated, build, and dependency directories
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/.turbo/**",
      "**/coverage/**",
      "apps/web/src/routeTree.gen.ts",
      "apps/api/src/generated/**",
    ],
  },
];
