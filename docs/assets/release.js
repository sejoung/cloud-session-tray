const repository = "github.com/sejoung/cloud-session-tray";
const latestRelease = "https://api.github.com/repos/sejoung/cloud-session-tray/releases/latest";
const installers = document.querySelectorAll("[data-installer]");

fetch(latestRelease, { signal: AbortSignal.timeout(5000) })
  .then(async (response) => {
    if (!response.ok) return;
    const release = await response.json();
    if (!Array.isArray(release.assets)) return;

    for (const link of installers) {
      const asset = release.assets.find(({ name, browser_download_url: url }) =>
        typeof name === "string" && name.endsWith(link.dataset.installer) &&
        typeof url === "string" && url.startsWith(`https://${repository}/releases/download/`),
      );
      if (asset) link.href = asset.browser_download_url;
    }
  })
  .catch(() => {});
