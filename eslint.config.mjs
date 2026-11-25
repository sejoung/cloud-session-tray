import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import globals from "globals";
import tseslint from "typescript-eslint";

export default defineConfig(
  { ignores: ["dist/**", "src-tauri/**", "node_modules/**"] },
  {
    files: ["**/*.{cjs,mjs,ts}"],
    extends: [js.configs.recommended, tseslint.configs.recommended],
    languageOptions: { globals: globals.node },
    linterOptions: { reportUnusedDisableDirectives: "error" },
    rules: {
      "max-lines": ["error", { max: 400 }],
      "complexity": ["error", { max: 16, variant: "modified" }],
      "eqeqeq": "error",
      "no-duplicate-imports": "error",
    },
  },
  {
    files: ["src/**/*.ts", "tests/**/*.ts"],
    languageOptions: { globals: globals.browser },
  },
);
