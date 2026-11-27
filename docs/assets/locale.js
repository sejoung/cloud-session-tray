const entry = new URL(window.location.href);

if (entry.searchParams.get("lang") !== "en" && /^ko(?:-|$)/i.test(navigator.language)) {
  const korean = new URL("ko/", entry);
  korean.search = entry.search;
  korean.hash = entry.hash;
  window.location.replace(korean.href);
}
