<script>
  export let name = "home";
  export let size = 18;
  const paths = {
    home: '<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1Z"/>',
    "layout-grid":
      '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
    "sliders-horizontal":
      '<path d="M3 6h6m4 0h8M3 12h12m4 0h2M3 18h2m4 0h12M9 3v6m6 0v6M5 15v6"/>',
    shirt: '<path d="m8 3-5 3 3 5 2-1v11h8V10l2 1 3-5-5-3c0 4-8 4-8 0Z"/>',
    settings:
      '<path d="M3 6h6m4 0h8M3 12h12m4 0h2M3 18h2m4 0h12M9 3v6m6 0v6M5 15v6"/>',
    users:
      '<circle cx="9" cy="7" r="4"/><path d="M2 21v-3a7 7 0 0 1 14 0v3m1-18a4 4 0 0 1 0 8m2 4a7 7 0 0 1 3 6"/>',
    download: '<path d="M12 3v12m-5-5 5 5 5-5M3 16v5h18v-5"/>',
    play: '<path d="m7 4 14 8-14 8Z"/>',
    search: '<circle cx="10.5" cy="10.5" r="7.5"/><path d="m16 16 5 5"/>',
    "arrow-up-right": '<path d="M7 17 17 7M7 7h10v10"/>',
    chevron: '<path d="m9 5 7 7-7 7"/>',
    close: '<path d="m6 6 12 12M6 18 18 6"/>',
    check: '<path d="m5 12 4 4L19 6"/>',
    copy: '<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V3H3v13h5"/>',
    folder: '<path d="M3 7V4h6l3 3h9v14H3Z"/>',
    edit: '<path d="m15 4 5 5-11 11H4v-5Zm-2 2 5 5"/>',
    cube: '<path d="m12 3 9 5v9l-9 5-9-5V8Zm-9 5 9 5 9-5m-9 5v9"/>',
    alert: '<path d="m12 3 10 18H2Zm0 6v5m0 3v1"/>',
    refresh: '<path d="M21 8V3l-4 4A9 9 0 1 0 4 18m17-10h-6"/>',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true">{@html paths[name] || paths["layout-grid"]}</svg
>
