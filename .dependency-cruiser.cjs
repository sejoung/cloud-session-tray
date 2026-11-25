const tauri = "(^|/)@tauri-apps/";

module.exports = {
  forbidden: [
    { name: "no-circular", severity: "error", from: {}, to: { circular: true } },
    { name: "resolved-imports", severity: "error", from: {}, to: { couldNotResolve: true } },
    {
      name: "domain-is-independent",
      severity: "error",
      from: { path: "^src/domain\\.ts$" },
      to: {},
    },
    {
      name: "views-use-domain-and-other-views",
      severity: "error",
      from: { path: "^src/ui/" },
      to: { path: "^src/(?!ui/|domain[.]ts$)" },
    },
    {
      name: "backend-does-not-depend-on-app-or-views",
      severity: "error",
      from: { path: "^src/backend\\.ts$" },
      to: { path: "^src/(?!(?:domain|desktop-api)[.]ts$)" },
    },
    {
      name: "desktop-api-only-uses-domain-types",
      severity: "error",
      from: { path: "^src/desktop-api[.]ts$" },
      to: { path: "^src/(?!domain[.]ts$)" },
    },
    {
      name: "application-depends-on-the-desktop-api-contract",
      severity: "error",
      from: { path: "^src/app[.]ts$" },
      to: { path: "^src/backend[.]ts$" },
    },
    {
      name: "tauri-only-through-backend",
      severity: "error",
      from: { path: "^src/(?!backend\\.ts$)" },
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
