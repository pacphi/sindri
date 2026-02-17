// Root ESLint flat config (ESLint 9+).
// Individual apps can extend this or override rules as needed.
import js from "@eslint/js";
import tseslint from "typescript-eslint";

const unusedVarsConfig = [
  "error",
  {
    argsIgnorePattern: "^_",
    varsIgnorePattern: "^_",
    caughtErrorsIgnorePattern: "^_",
    destructuredArrayIgnorePattern: "^_",
  },
];

/** @type {import("eslint").Linter.Config[]} */
export default [
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

  // Base recommended rules for all JS/TS files
  js.configs.recommended,

  // TypeScript parser and recommended rules
  ...tseslint.configs.recommended,

  {
    // Apply to all TypeScript and TSX source files
    files: [
      "apps/*/src/**/*.{ts,tsx}",
      "packages/*/src/**/*.{ts,tsx}",
      "apps/*/tests/**/*.{ts,tsx}",
    ],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
    },
    rules: {
      // Allow underscore-prefixed unused vars (common destructuring convention)
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": unusedVarsConfig,
      // Disable no-duplicate-imports — it doesn't understand TS type-only imports
      "no-duplicate-imports": "off",
      "no-console": ["warn", { allow: ["warn", "error"] }],
      "no-debugger": "error",
      "prefer-const": "error",
      "no-var": "error",
      "@typescript-eslint/no-explicit-any": "warn",
    },
  },
];
