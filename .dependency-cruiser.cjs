const tauri = "(^|/)@tauri-apps/";

module.exports = {
  forbidden: [
    { name: "no-circular", severity: "error", from: {}, to: { circular: true } },
    { name: "resolved-imports", severity: "error", from: {}, to: { couldNotResolve: true } },
    {
      name: "domain-is-independent",
      severity: "error",
      from: { path: "^src/shared/domain[.]ts$" },
      to: {},
    },
    {
      name: "features-use-shared-code-and-other-features",
      severity: "error",
      from: { path: "^src/features/" },
      to: { path: "^src/(?!features/|shared/)" },
    },
    {
      name: "shared-code-does-not-depend-on-features",
      severity: "error",
      from: { path: "^src/shared/" },
      to: { path: "^src/(?!shared/)" },
    },
    {
      name: "backend-does-not-depend-on-app-or-views",
      severity: "error",
      from: { path: "^src/desktop/tauri[.]ts$" },
      to: { path: "^src/(?!(?:shared/domain|desktop/api)[.]ts$)" },
    },
    {
      name: "desktop-api-only-uses-domain-types",
      severity: "error",
      from: { path: "^src/desktop/api[.]ts$" },
      to: { path: "^src/(?!shared/domain[.]ts$)" },
    },
    {
      name: "application-depends-on-the-desktop-api-contract",
      severity: "error",
      from: { path: "^src/app/" },
      to: { path: "^src/desktop/tauri[.]ts$" },
    },
    {
      name: "tauri-only-through-backend",
      severity: "error",
      from: { path: "^src/(?!desktop/tauri[.]ts$)" },
      to: { path: tauri },
    },
    {
      name: "entrypoint-is-not-a-library",
      severity: "error",
      from: { path: "^src/" },
      to: { path: "^src/main\\.ts$" },
    },
  ],
  options: {
    doNotFollow: { path: ["^node_modules/", "^src-tauri/icons/"] },
    tsConfig: { fileName: "tsconfig.json" },
    tsPreCompilationDeps: true,
  },
};
