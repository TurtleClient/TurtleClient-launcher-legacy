<script>
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "./Icon.svelte";
  import PlayerPreview from "./PlayerPreview.svelte";
  import { words } from "./native-i18n.js";

  function stored(key, fallback) {
    try {
      return JSON.parse(localStorage.getItem(key)) ?? fallback;
    } catch {
      return fallback;
    }
  }
  let preferences = stored("turtle-launcher-preferences", {
    locale: localStorage.getItem("turtle-language") || "de",
    accent: "#39d878",
    minimize: false,
    restore: true,
    reducedMotion: false,
  });
  let tab = "overview",
    appearanceTab = "capes",
    moduleTab = "emotes";
  let state = null,
    error = "",
    pending = false,
    polling = false,
    search = "",
    moduleSearch = "";
  let minecraftPath = "",
    dialog = null,
    profileName = "",
    signOutId = "",
    profileInput;
  let toast = "",
    toastTimer,
    cape = "",
    capeAccount = null,
    capeInput,
    previewBack = false,
    emote = "wave";
  let playerSkin = null,
    skinLoading = false,
    skinError = false,
    skinRequest = 0;
  let previousAction = null,
    minimizedForGame = false,
    dismissedError = "";
  const emotes = ["wave", "dance", "point", "clap", "cheer", "laugh"];
  const keys = "abcdefghijklmnopqrstuvwxyz0123456789".split("");
  const nav = [
    ["overview", "home"],
    ["profiles", "layout-grid"],
    ["modules", "sliders-horizontal"],
    ["appearance", "shirt"],
  ];
  const accents = ["#39d878", "#6ca8ff", "#b5b8c4"];
  $: t = words[preferences.locale] || words.de;
  $: profiles = state?.profiles || [];
  $: selected = profiles.find((p) => p.id === state?.selected_profile);
  $: accounts = state?.accounts?.accounts || [];
  $: account = accounts.find((a) => a.selected);
  $: player = account?.name || t.noAccount;
  $: busy = pending || !!state?.action;
  $: playing = state?.action === "play";
  $: ready = !!(
    state?.installed &&
    state?.prepared &&
    state?.accounts?.owner &&
    selected
  );
  $: version = state?.minecraft || "26.3";
  $: filteredProfiles = [...profiles]
    .filter((p) =>
      p.name.toLocaleLowerCase().includes(search.toLocaleLowerCase()),
    )
    .sort((a, b) => (b.last_played_ms || 0) - (a.last_played_ms || 0));
  $: code = [...(state?.lines || [])]
    .reverse()
    .map((l) => l.match(/(?:code\s+|code:\s*)([A-Z0-9][A-Z0-9-]{3,})/i)?.[1])
    .find(Boolean);
  $: notice = error || (state?.error !== dismissedError ? state?.error : "");
  $: status = state?.action
    ? t.working[state.action] || t.pleaseWait
    : !state
      ? t.loading
      : !state.installed
        ? t.clientMissing
        : !state.accounts?.owner
          ? t.noAccount
          : !state.prepared
            ? t.resourcesMissing
            : t.ready;
  $: primaryLabel = busy
    ? playing
      ? t.playing
      : t.pleaseWait
    : !state?.installed
      ? t.install
      : !state?.accounts?.owner
        ? t.signIn
        : !state?.prepared
          ? t.needsSetup
          : t.play;
  $: modules = state?.modules;
  $: accent = accents.includes(preferences.accent)
    ? preferences.accent
    : accents[0];

  function persist() {
    localStorage.setItem(
      "turtle-launcher-preferences",
      JSON.stringify(preferences),
    );
    localStorage.setItem("turtle-language", preferences.locale);
  }
  function notify(message) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 3000);
  }
  function navigate(next) {
    tab = next;
    search = "";
  }
  async function refresh() {
    if (polling) return;
    polling = true;
    try {
      const next = await invoke("get_state");
      state = next;
      if (
        next.action === "play" &&
        previousAction !== "play" &&
        preferences.minimize
      ) {
        try {
          await getCurrentWindow().minimize();
          minimizedForGame = true;
        } catch (e) {
          error = String(e);
        }
      }
      if (previousAction === "play" && !next.action && minimizedForGame) {
        if (preferences.restore) {
          try {
            const window = getCurrentWindow();
            await window.unminimize();
            await window.show();
            await window.setFocus();
          } catch (e) {
            error = String(e);
          }
        }
        minimizedForGame = false;
      }
      previousAction = next.action;
      const uuid = next.accounts?.accounts?.find((a) => a.selected)?.uuid || "";
      if (uuid !== capeAccount) {
        capeAccount = uuid;
        await loadCape();
        playerSkin = null;
        loadPlayerSkin();
      }
    } catch (e) {
      error = String(e);
    } finally {
      polling = false;
    }
  }
  async function run(action, value = null, wait = false) {
    pending = true;
    error = "";
    dismissedError = "";
    try {
      await invoke("start_action", { action, value });
      await refresh();
      if (wait) {
        for (let i = 0; i < 150 && state?.action; i++) {
          await new Promise((r) => setTimeout(r, 200));
          await refresh();
        }
        if (state?.action) throw new Error(t.operationBusy);
        if (state?.error) throw new Error(state.error);
      }
      return true;
    } catch (e) {
      error = String(e);
      return false;
    } finally {
      pending = false;
    }
  }
  async function play(profile = selected) {
    if (busy || !state) return;
    if (
      profile &&
      profile.id !== selected?.id &&
      !(await run("select-profile", profile.id, true))
    )
      return;
    if (!state.installed) {
      navigate("setup");
      await run("install");
    } else if (!state.accounts?.owner) navigate("account");
    else if (!state.prepared) navigate("setup");
    else await run("play");
  }
  async function open(location) {
    try {
      await invoke("open_location", { location });
    } catch (e) {
      error = String(e);
    }
  }
  async function cancelOperation() {
    try {
      await invoke("cancel_action");
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }
  async function openMicrosoft() {
    try {
      await invoke("open_sign_in");
    } catch (e) {
      error = String(e);
    }
  }
  async function copy(text) {
    try {
      await navigator.clipboard.writeText(text);
      notify(t.copied);
    } catch (e) {
      error = String(e);
    }
  }
  async function profileDialog(kind) {
    profileName = kind === "rename" ? selected?.name || "" : "";
    dialog = kind;
    await tick();
    profileInput?.focus();
  }
  async function saveProfile() {
    if (!profileName.trim() || busy) return;
    if (
      await run(
        dialog === "rename" ? "rename-profile" : "create-profile",
        profileName.trim(),
        true,
      )
    ) {
      dialog = null;
      navigate("profiles");
    }
  }
  async function setModule(patch) {
    if (!modules || busy) return;
    pending = true;
    error = "";
    try {
      const result = await invoke("save_modules", {
        settings: { ...modules, ...patch },
      });
      state = { ...state, modules: result };
      notify(t.saveSuccess);
    } catch (e) {
      error = String(e);
    } finally {
      pending = false;
    }
  }
  async function loadPlayerSkin(refresh = false) {
    const request = ++skinRequest;
    const uuid = state?.accounts?.accounts?.find((a) => a.selected)?.uuid;
    playerSkin = refresh ? playerSkin : null;
    skinError = false;
    if (!uuid) {
      skinLoading = false;
      return;
    }
    skinLoading = true;
    try {
      const appearance = await invoke("read_player_skin", { refresh });
      if (request === skinRequest) playerSkin = appearance;
    } catch {
      if (request === skinRequest) skinError = true;
    } finally {
      if (request === skinRequest) skinLoading = false;
    }
  }
  async function loadCape() {
    try {
      const data = await invoke("read_cape");
      cape = data
        ? `data:image/png;base64,${btoa(Array.from(data, (byte) => String.fromCharCode(byte)).join(""))}`
        : "";
    } catch (e) {
      error = String(e);
    }
  }
  async function upload(event) {
    const file = event.target.files?.[0];
    if (!file) return;
    event.target.value = "";
    if (file.type !== "image/png" || file.size > 2 * 1024 * 1024) {
      error = t.capeInvalid;
      return;
    }
    pending = true;
    error = "";
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (
        bytes.length < 24 ||
        new DataView(bytes.buffer).getUint32(16) !== 64 ||
        new DataView(bytes.buffer).getUint32(20) !== 32
      )
        throw new Error(t.capeInvalid);
      await invoke("upload_cape", { data: Array.from(bytes) });
      await loadCape();
      notify(t.capeUploaded);
    } catch (e) {
      error = String(e);
    } finally {
      pending = false;
    }
  }
  function lastPlayed(profile) {
    if (!profile.last_played_ms) return t.neverPlayed;
    return new Intl.DateTimeFormat(preferences.locale, {
      dateStyle: "short",
      timeStyle: "short",
    }).format(profile.last_played_ms);
  }
  function shortcut(event) {
    if (dialog) {
      if (event.key === "Tab") {
        const controls = [
          ...document
            .querySelector(".modal")
            .querySelectorAll("button:not(:disabled),input:not(:disabled)"),
        ];
        const first = controls[0],
          last = controls.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    }

    if (event.key === "Escape") dialog = null;
    if (
      event.key === "Enter" &&
      !dialog &&
      !["INPUT", "SELECT", "TEXTAREA", "BUTTON"].includes(
        event.target.tagName,
      ) &&
      !busy
    )
      play();
  }
  onMount(() => {
    let active = true;
    (async () => {
      await refresh();
      if (active && state && !state.installed) await run("install");
    })();
    const timer = setInterval(refresh, 1800);
    return () => {
      active = false;
      clearInterval(timer);
      clearTimeout(toastTimer);
    };
  });
</script>

<svelte:window on:keydown={shortcut} />
<div
  class="launcher"
  class:reduce-motion={preferences.reducedMotion}
  style:--accent={accent}
>
  <aside class="navigation">
    <button
      class="brand-button"
      title="Turtle Client"
      aria-label="Turtle Client"
      on:click={() => navigate("overview")}
      ><img src="/img/logo.png" alt="" /></button
    >
    <nav aria-label="Turtle Client">
      {#each nav as [page, icon]}<button
          class:active={tab === page}
          title={t[page]}
          aria-label={t[page]}
          aria-current={tab === page ? "page" : undefined}
          on:click={() => navigate(page)}><Icon name={icon} size={21} /></button
        >{/each}
    </nav>
    <nav class="bottom-nav" aria-label={t.settings}>
      <button
        class:active={tab === "settings"}
        title={t.settings}
        aria-label={t.settings}
        on:click={() => navigate("settings")}
        ><Icon name="settings" size={21} /></button
      >
      <button
        class:active={tab === "activity"}
        title={t.activity}
        aria-label={t.activity}
        on:click={() => navigate("activity")}
        ><Icon name="download" size={21} /></button
      >
    </nav>
    <button
      class="nav-account"
      title={player}
      aria-label={t.account}
      on:click={() => navigate("account")}
      ><span class="avatar"
        >{account?.name?.slice(0, 1).toUpperCase() || "?"}</span
      ></button
    >
  </aside>
  <div class="workspace">
    <header class="topbar">
      <div class="breadcrumb">
        <span>Turtle Client</span><span>/</span><strong>{t[tab]}</strong>
      </div>
      <div class="header-actions">
        <span class="version">Native {version}</span><button
          class="account-button"
          on:click={() => navigate("account")}
          ><span class="avatar"
            >{account?.name?.slice(0, 1).toUpperCase() || "?"}</span
          >{player}<Icon name="chevron" size={12} /></button
        >
      </div>
    </header>
    <main id="main-content">
      {#if notice}<div class="error-notice" role="alert">
          <Icon name="alert" size={20} />
          <div class="grow">
            <strong>{t.error}</strong>
            <details>
              <summary>{t.details}</summary>
              <p>{notice}</p>
            </details>
            <button class="text-button" on:click={() => navigate("activity")}
              >{t.viewLog}<Icon name="chevron" size={13} /></button
            >
          </div>
          <button
            class="icon-button"
            aria-label={t.close}
            on:click={() => {
              error = "";
              dismissedError = state?.error || "";
            }}><Icon name="close" /></button
          >
        </div>{/if}
      {#if !state}
        <section class="empty-state">
          <img src="/img/logo.png" alt="Turtle" />
          <h1>{error ? t.offline : t.loading}</h1>
          <p>{error ? t.backendHint : t.pleaseWait}</p>
          <button
            on:click={() => {
              error = "";
              refresh();
            }}><Icon name="refresh" />{t.refresh}</button
          >
        </section>
      {:else if tab === "overview"}
        <div class="page-heading">
          <div>
            <h1>{account ? `${t.hello}, ${player}.` : t.welcome}</h1>
            <p>{t.homeHint}</p>
          </div>
          <button on:click={() => navigate("profiles")}
            >{t.manageProfiles}<Icon name="chevron" size={15} /></button
          >
        </div>
        <div class="home-grid">
          <section>
            <div class="active-profile">
              <div class="active-profile-copy">
                <span class="caption">{t.selectedProfile}</span>
                <div class="profile-identity">
                  <div class="game-icon">
                    <img src="/img/logo.png" alt="" />
                  </div>
                  <div>
                    <h2>{selected?.name || "Turtle Client"}</h2>
                    <div class="badges">
                      <span class="badge">Minecraft {version}</span><span
                        class="badge">Native</span
                      >
                    </div>
                  </div>
                </div>
                <p>{selected ? lastPlayed(selected) : t.profileRequired}</p>
                <div class="button-row">
                  <button on:click={() => navigate("modules")}
                    >{t.modules}</button
                  ><button on:click={() => navigate("appearance")}
                    >{t.changeAppearance}</button
                  ><button
                    class="icon-button"
                    disabled={!selected || busy}
                    aria-label={t.editProfile}
                    title={t.editProfile}
                    on:click={() => profileDialog("rename")}
                    ><Icon name="sliders-horizontal" /></button
                  >
                </div>
              </div>
              <div class="profile-player">
                <PlayerPreview
                  skin={playerSkin}
                  {cape}
                  reducedMotion={preferences.reducedMotion}
                  label={t.skinControls}
                  unavailable={t.skinRendererUnavailable}
                />
              </div>
            </div>
            <div class="section-heading">
              <h3>{t.recent}</h3>
              <span>Minecraft {version}</span>
            </div>
            <div class="profile-list">
              {#each [...profiles]
                .sort((a, b) => (b.last_played_ms || 0) - (a.last_played_ms || 0))
                .slice(0, 3) as profile}<div
                  class="profile-row"
                  class:selected={profile.id === selected?.id}
                >
                  <div class="game-icon small">
                    <img src="/img/logo.png" alt="" />
                  </div>
                  <div class="grow">
                    <strong>{profile.name}</strong>
                    <p>Native · {lastPlayed(profile)}</p>
                  </div>
                  {#if profile.id === selected?.id}<span class="selected-label"
                      >{t.selected}</span
                    >{/if}<button
                    class="mini-play"
                    disabled={busy}
                    on:click={() => play(profile)}
                    ><Icon name="play" size={14} />{t.play}</button
                  ><button
                    class="icon-button"
                    aria-label={t.select}
                    title={t.select}
                    disabled={busy || profile.id === selected?.id}
                    on:click={() => run("select-profile", profile.id)}
                    ><Icon name="chevron" size={16} /></button
                  >
                </div>{:else}<div class="list-empty">
                  <p>{t.noProfiles}</p>
                  <button disabled={busy} on:click={() => run("install")}
                    >{t.install}</button
                  >
                </div>{/each}
            </div>
            <div class="button-row home-buttons">
              <button
                class="grow"
                disabled={busy || !state.installed}
                on:click={() => profileDialog("create")}
                >+ {t.newProfile}</button
              ><button class="grow" on:click={() => navigate("modules")}
                >{t.modules}</button
              >
            </div>
          </section>
          <aside class="side-stack">
            <section class="panel">
              <h3>{t.quickAccess}</h3>
              {#each [["account", t.minecraftAccount, player], ["setup", t.setup, `Minecraft ${version}`], ["activity", t.activity, t.logHint]] as [page, label, detail]}<button
                  class="quick-row"
                  on:click={() => navigate(page)}
                  ><span><strong>{label}</strong><small>{detail}</small></span
                  ><Icon name="chevron" size={16} /></button
                >{/each}
            </section>
            <section class="panel release-panel">
              <div class="row between">
                <h3>Turtle 0.1.0</h3>
                <span class="badge">Alpha</span>
              </div>
              <p>{t.inThisVersion}</p>
              {#each [["B", "emotes", "emoteShort"], ["C", "zoom", "zoomShort"], ["", "capes", "capeShort"]] as [key, label, description]}<div
                  class="release-row"
                >
                  <span class="key"
                    >{#if key}{key}{:else}<Icon
                        name="shirt"
                        size={16}
                      />{/if}</span
                  >
                  <div>
                    <strong>{t[label]}</strong><small>{t[description]}</small>
                  </div>
                </div>{/each}<button
                class="full"
                on:click={() => navigate("activity")}>{t.changes}</button
              >
            </section>
          </aside>
        </div>
      {:else if tab === "profiles"}
        <div class="page-heading">
          <div>
            <h1>{t.profiles}</h1>
            <p>{t.profileHint}</p>
          </div>
          <button
            class="primary"
            disabled={busy || !state.installed}
            on:click={() => profileDialog("create")}>+ {t.newProfile}</button
          >
        </div>
        <div class="toolbar">
          <span
            >{t.allProfiles} <span class="muted">{profiles.length}</span></span
          ><label class="search"
            ><Icon name="search" size={17} /><input
              bind:value={search}
              placeholder={t.profileSearch}
              aria-label={t.profileSearch}
            /></label
          >
        </div>
        <div class="profiles-layout">
          <div class="profiles-grid">
            {#each filteredProfiles as profile}<article
                class="profile-card"
                class:selected={profile.id === selected?.id}
              >
                <button
                  class="profile-cover"
                  disabled={busy}
                  aria-label={`${t.select}: ${profile.name}`}
                  on:click={() => run("select-profile", profile.id)}
                  ><img
                    src="/img/logo.png"
                    alt=""
                  />{#if profile.id === selected?.id}<span class="chip success"
                      >{t.selected}</span
                    >{/if}</button
                >
                <div class="profile-card-body">
                  <h3>{profile.name}</h3>
                  <p>{lastPlayed(profile)}</p>
                  <div class="row between">
                    <span class="badge">{version} · Native</span><button
                      class="mini-play"
                      disabled={busy}
                      on:click={() => play(profile)}
                      ><Icon name="play" size={13} />{t.play}</button
                    >
                  </div>
                </div>
              </article>{:else}<div class="list-empty">
                <p>{profiles.length ? t.noResults : t.noProfiles}</p>
              </div>{/each}
          </div>
          <aside class="panel profile-details">
            <span class="caption">{t.selectedProfile}</span>
            <h2>{selected?.name || t.noSelection}</h2>
            <dl>
              <div>
                <dt>{t.version}</dt>
                <dd>{version}</dd>
              </div>
              <div>
                <dt>Client</dt>
                <dd>Native · Windows x64</dd>
              </div>
              <div>
                <dt>{t.modules}</dt>
                <dd>
                  {modules
                    ? [
                        modules.zoom_enabled,
                        modules.emotes_enabled,
                        modules.capes,
                      ].filter(Boolean).length
                    : 0} / 3
                </dd>
              </div>
            </dl>
            <button
              class="full"
              disabled={!selected || busy}
              on:click={() => profileDialog("rename")}
              ><Icon name="edit" size={15} />{t.renameProfile}</button
            ><button class="full" on:click={() => navigate("modules")}
              >{t.modules}</button
            ><button class="full" on:click={() => navigate("appearance")}
              >{t.appearance}</button
            >
            <div class="divider"></div>
            <span class="caption">{t.dataFolder}</span>
            <p class="path">{selected?.home || state.data_home}</p>
          </aside>
        </div>
      {:else if tab === "modules"}
        <div class="page-heading">
          <div>
            <h1>{t.modules}</h1>
            <p>{selected?.name || t.noSelection} · {t.autoSave}</p>
          </div>
          <label class="search"
            ><Icon name="search" size={17} /><input
              bind:value={moduleSearch}
              placeholder={t.moduleSearch}
              aria-label={t.moduleSearch}
            /></label
          >
        </div>
        {#if !selected}<div class="panel list-empty">
            <p>{t.profileRequired}</p>
            <button on:click={() => navigate("setup")}>{t.goSetup}</button>
          </div>{:else}<div class="modules-layout">
            <section>
              <div class="section-heading">
                <h3>{t.installedModules} <span class="muted">3</span></h3>
                <span>Native {version}</span>
              </div>
              <div class="module-list">
                {#each [["emotes", "B", "emoteDescription", "emotes_enabled"], ["zoom", "C", "zoomDescription", "zoom_enabled"], ["capes", "", "capeDescription", "capes"]] as [name, key, description, property]}{#if t[name]
                    .toLocaleLowerCase()
                    .includes(moduleSearch.toLocaleLowerCase())}<div
                      class="module-row"
                    >
                      <span class="module-glyph"
                        >{#if key}{key}{:else}<Icon
                            name="shirt"
                            size={21}
                          />{/if}</span
                      >
                      <div class="grow">
                        <h3>{t[name]}</h3>
                        <p>{t[description]}</p>
                      </div>
                      <button
                        class="module-config"
                        class:chosen={moduleTab === name}
                        on:click={() => (moduleTab = name)}
                        >{t.configure}</button
                      ><button
                        class="toggle"
                        class:on={modules[property]}
                        role="switch"
                        aria-checked={modules[property]}
                        aria-label={t[name]}
                        disabled={busy}
                        on:click={() =>
                          setModule({ [property]: !modules[property] })}
                      ></button>
                    </div>{/if}{/each}
              </div>
              <p class="footnote">{t.nativeNotice}</p>
              {#if playing}<p class="footnote">{t.changesAfterRestart}</p>{/if}
            </section>
            <aside class="panel module-details">
              <div class="row between">
                <h3>{t[moduleTab]}</h3>
                <span class="chip">{t.configure}</span>
              </div>
              {#if moduleTab === "emotes"}<div class="mini-player">
                  <PlayerPreview
                    skin={playerSkin}
                    {cape}
                    reducedMotion={preferences.reducedMotion}
                    label={t.skinControls}
                    unavailable={t.skinRendererUnavailable}
                  />
                </div>
                <label for="emote-key">{t.key}</label><select
                  id="emote-key"
                  disabled={busy}
                  value={modules.emote_key}
                  on:change={(e) =>
                    setModule({ emote_key: e.currentTarget.value })}
                  >{#if !keys.includes(modules.emote_key)}<option
                      value={modules.emote_key}>{modules.emote_key}</option
                    >{/if}{#each keys as key}<option value={key}
                      >{key.toUpperCase()}</option
                    >{/each}</select
                >
                <p>{t.emoteNote}</p>
                <button
                  class="full"
                  on:click={() => {
                    appearanceTab = "emotes";
                    navigate("appearance");
                  }}>{t.preview}</button
                >{:else if moduleTab === "zoom"}<div class="setting-block">
                  <label for="zoom-key">{t.key}</label><select
                    id="zoom-key"
                    disabled={busy}
                    value={modules.zoom_key}
                    on:change={(e) =>
                      setModule({ zoom_key: e.currentTarget.value })}
                    >{#if !keys.includes(modules.zoom_key)}<option
                        value={modules.zoom_key}>{modules.zoom_key}</option
                      >{/if}{#each keys as key}<option value={key}
                        >{key.toUpperCase()}</option
                      >{/each}</select
                  >
                </div>
                <div class="setting-block">
                  <label for="zoom-factor"
                    >{t.zoomFactor}
                    <strong>{modules.zoom_factor.toFixed(1)}×</strong></label
                  ><input
                    id="zoom-factor"
                    type="range"
                    min="2"
                    max="10"
                    step=".5"
                    value={modules.zoom_factor}
                    disabled={busy}
                    on:change={(e) =>
                      setModule({ zoom_factor: Number(e.currentTarget.value) })}
                  />
                </div>
                <div class="setting-row">
                  <span>{t.smoothZoom}</span><button
                    class="toggle"
                    class:on={modules.smooth_zoom}
                    role="switch"
                    aria-checked={modules.smooth_zoom}
                    aria-label={t.smoothZoom}
                    disabled={busy}
                    on:click={() =>
                      setModule({ smooth_zoom: !modules.smooth_zoom })}
                  ></button>
                </div>{:else}<div class="cape-empty-icon">
                  <Icon name="shirt" size={48} />
                </div>
                <p>{t.capeNote}</p>
                <button
                  class="full"
                  on:click={() => {
                    appearanceTab = "capes";
                    navigate("appearance");
                  }}>{t.appearance}</button
                >{/if}
            </aside>
          </div>{/if}
      {:else if tab === "appearance"}
        <div class="page-heading">
          <div>
            <h1>{t.appearance}</h1>
            <p>{t.appearanceHint}</p>
          </div>
          <span class="muted small">{t.autoSave}</span>
        </div>
        <div class="appearance-layout">
          <section class="panel character-panel">
            <div class="row between">
              <span class="badge">{t.preview}</span><span class="muted small"
                >{player}</span
              >
            </div>
            <div class="character-stage">
              <PlayerPreview
                skin={playerSkin}
                reducedMotion={preferences.reducedMotion}
                label={t.skinControls}
                unavailable={t.skinRendererUnavailable}
                back={previewBack}
                {cape}
                emote={appearanceTab === "emotes" ? emote : ""}
              />
            </div>
            <div class="row center">
              <button on:click={() => (previewBack = !previewBack)}
                ><Icon name="refresh" size={14} />{t.rotate}</button
              ><button
                on:click={() =>
                  (appearanceTab =
                    appearanceTab === "emotes" ? "capes" : "emotes")}
                >{t[appearanceTab === "emotes" ? "capes" : "emotes"]}</button
              >
            </div>
            <p class="small muted center-text" role="status">
              {skinLoading
                ? t.skinLoading
                : playerSkin?.skin
                  ? playerSkin.cached
                    ? t.skinCached
                    : t.ownSkin
                  : account
                    ? skinError
                      ? t.skinOffline
                      : t.skinDefaultMissing
                    : t.skinSignIn}
            </p>
            <p class="small muted center-text">{t.skinControls}</p>
            {#if account}<button
                class="full"
                disabled={skinLoading}
                on:click={() => loadPlayerSkin(true)}
                ><Icon name="refresh" size={14} />{t.reloadSkin}</button
              >{/if}
          </section>
          <section>
            <div class="tabs" role="tablist" aria-label={t.appearance}>
              <button
                role="tab"
                aria-selected={appearanceTab === "capes"}
                class:active={appearanceTab === "capes"}
                on:click={() => (appearanceTab = "capes")}>{t.capes}</button
              ><button
                role="tab"
                aria-selected={appearanceTab === "emotes"}
                class:active={appearanceTab === "emotes"}
                on:click={() => (appearanceTab = "emotes")}>{t.emotes}</button
              >
            </div>
            {#if appearanceTab === "capes"}<div class="section-heading">
                <h3>{t.yourCape}</h3>
                <button
                  disabled={busy || !account}
                  on:click={() => capeInput?.click()}>+ {t.uploadCape}</button
                ><input
                  class="visually-hidden"
                  type="file"
                  bind:this={capeInput}
                  accept="image/png"
                  on:change={upload}
                  aria-label={t.uploadCape}
                />
              </div>
              {#if cape}<div class="panel cape-card">
                  <img src={cape} alt={t.yourCape} /><span class="chip success"
                    >{t.active}</span
                  >
                </div>{:else}<div class="panel cape-empty">
                  <Icon name="shirt" size={46} />
                  <h3>{t.noCape}</h3>
                  <p>{account ? t.capeUploadHint : t.capeSignIn}</p>
                  {#if !account}<button on:click={() => navigate("account")}
                      >{t.signIn}</button
                    >{/if}
                </div>{/if}
              <p class="footnote">{t.capeRequirements}</p>
              <p class="footnote">{t.capeNote}</p>{:else}<div
                class="emote-layout"
              >
                <div class="emote-wheel">
                  <div class="wheel-center">
                    <kbd>{(modules?.emote_key || "b").toUpperCase()}</kbd><small
                      >{t.hold}</small
                    >
                  </div>
                  {#each emotes as name, i}<button
                      style:--angle={`${i * 60 - 90}deg`}
                      class:active={emote === name}
                      on:click={() => (emote = name)}
                      ><small>{String(i + 1).padStart(2, "0")}</small>{t[
                        name
                      ]}</button
                    >{/each}
                </div>
              </div>
              <div class="emote-instructions">
                <span
                  ><kbd>{(modules?.emote_key || "b").toUpperCase()}</kbd>
                  {t.emoteKey}</span
                ><span>{t.chooseEmote}</span><span>{t.releaseEmote}</span>
              </div>
              <p class="footnote">{t.emoteNote}</p>{/if}
          </section>
        </div>
      {:else if tab === "account"}
        <div class="page-heading">
          <div>
            <h1>{t.account}</h1>
            <p>{t.accountHint}</p>
          </div>
          <button
            disabled={busy || state.accounts?.sign_in_configured === false}
            on:click={() => run("sign-in")}>+ {t.addAccount}</button
          >
        </div>
        <div class="account-layout">
          <section>
            {#if state.action === "sign-in"}<section class="panel login-panel">
                <h2>{t.signIn}</h2>
                <p>{t.deviceHint}</p>
                <div class="device-code" aria-live="polite">
                  {code || "•••• – ••••"}
                </div>
                <div class="button-row">
                  <button disabled={!code} on:click={() => copy(code)}
                    ><Icon name="copy" size={15} />{t.copyCode}</button
                  ><button
                    class="primary"
                    disabled={!code}
                    on:click={openMicrosoft}
                    >{t.openMicrosoft}<Icon
                      name="arrow-up-right"
                      size={15}
                    /></button
                  >
                </div>
                <p class="footnote">
                  {code ? t.autoConnect : t.waitingForCode}
                </p>
                <button class="text-button" on:click={cancelOperation}
                  >{t.cancelOperation}</button
                >
              </section>{/if}{#each accounts as item}<section
                class="panel account-panel"
              >
                <div class="row">
                  <span class="avatar large"
                    >{item.name.slice(0, 1).toUpperCase()}</span
                  >
                  <div class="grow">
                    <h2>{item.name}</h2>
                    <p>Microsoft · Minecraft: Java Edition</p>
                  </div>
                  {#if item.selected}<span class="chip success"
                      >{t.activeAccount}</span
                    >{:else}<button
                      disabled={busy}
                      on:click={() => run("select", item.uuid)}
                      >{t.select}</button
                    >{/if}
                </div>
                <dl>
                  <div>
                    <dt>{t.entitlement}</dt>
                    <dd
                      class:success-text={item.selected && state.accounts.owner}
                    >
                      {item.selected && state.accounts.owner
                        ? t.ownsGame
                        : t.notVerified}
                    </dd>
                  </div>
                  <div>
                    <dt>{t.authentication}</dt>
                    <dd>
                      {item.authentication?.message ||
                        item.authentication?.state ||
                        t.connected}
                    </dd>
                  </div>
                  <div>
                    <dt>{t.currentProfile}</dt>
                    <dd>{selected?.name || "—"}</dd>
                  </div>
                </dl>
                <div class="row between">
                  <button disabled={busy} on:click={() => run("sign-in")}
                    >{t.renewLogin}</button
                  ><button
                    class="text-button"
                    disabled={busy}
                    on:click={() => {
                      signOutId = item.uuid;
                      dialog = "sign-out";
                    }}>{t.signOut}</button
                  >
                </div>
              </section>{:else}{#if state.action !== "sign-in"}<section
                  class="panel sign-in-panel"
                >
                  <Icon name="users" size={36} />
                  <h2>{t.signIn}</h2>
                  <p>{t.signInHint}</p>
                  <button
                    class="primary"
                    disabled={busy ||
                      state.accounts?.sign_in_configured === false}
                    on:click={() => run("sign-in")}
                    >{t.signIn}<Icon name="arrow-up-right" size={16} /></button
                  >
                  <p class="footnote">{t.requiresJava}</p>
                </section>{/if}{/each}{#if state.accounts?.sign_in_configured === false}<p
                class="footnote"
              >
                {t.noAuth}
              </p>{/if}
            <p class="footnote">{t.accountPrivacy}</p>
            {#if account}<button on:click={() => navigate("setup")}
                >{t.goSetup}<Icon name="chevron" size={14} /></button
              >{/if}
          </section>
          <aside class="panel account-help">
            <span class="caption">Microsoft</span>
            <h2>{t.signIn}</h2>
            <p>{t.accountPrivacy}</p>
            <div class="divider"></div>
            <p>{t.requiresJava}</p>
          </aside>
        </div>
      {:else if tab === "setup"}
        <div class="page-heading">
          <div>
            <h1>{t.setupTitle}</h1>
            <p>{t.setupHint}</p>
          </div>
          <span class="badge">Minecraft {version}</span>
        </div>
        <div class="setup-layout">
          <section class="panel setup-steps">
            <div class="setup-step">
              <span class="step-number">01</span>
              <div class="grow">
                <div class="row between">
                  <h3>
                    {state.installed ? t.clientInstalled : t.clientMissing}
                  </h3>
                  <span class="chip" class:success={state.installed}
                    >{state.installed ? t.installed : t.needsSetup}</span
                  >
                </div>
                <p>Turtle Native 0.1.0 · Minecraft {version} · Windows x64</p>
                <button disabled={busy} on:click={() => run("install")}
                  >{t.checkInstall}</button
                >
              </div>
            </div>
            <div class="setup-step">
              <span class="step-number">02</span>
              <div class="grow">
                <div class="row between">
                  <h3>{t.minecraftAccount}</h3>
                  <span class="chip" class:success={state.accounts.owner}
                    >{state.accounts.owner ? player : t.noAccount}</span
                  >
                </div>
                <p>{state.accounts.owner ? t.ownsGame : t.requiresJava}</p>
                <button class="text-button" on:click={() => navigate("account")}
                  >{t.account}<Icon name="chevron" size={13} /></button
                >
              </div>
            </div>
            <div class="setup-step">
              <span class="step-number">03</span>
              <div class="grow">
                <div class="row between">
                  <h3>{t.resources}</h3>
                  <span class="chip" class:success={state.prepared}
                    >{state.prepared
                      ? t.resourcesReady
                      : t.resourcesMissing}</span
                  >
                </div>
                <p>{t.resourceHint}</p>
                <label for="minecraft-path">{t.minecraftFolder}</label><input
                  id="minecraft-path"
                  bind:value={minecraftPath}
                  placeholder={t.automatic}
                />
                <div class="button-row">
                  <button
                    class="primary"
                    disabled={busy}
                    on:click={() => run("prepare", minecraftPath)}
                    >{t.prepare}</button
                  ><button
                    disabled={busy || !state.accounts.owner}
                    on:click={() => run("download")}>{t.download}</button
                  >
                </div>
                <p class="footnote">{t.firstPreparation}</p>
              </div>
            </div>
            {#if busy && !playing}<div class="operation-panel" role="status">
                <span class="spinner"></span>
                <div class="grow">
                  <strong>{status}</strong>
                  <p>{state.lines?.at(-1) || t.pleaseWait}</p>
                </div>
                <button on:click={cancelOperation}>{t.cancel}</button>
              </div>{/if}
          </section>
          <aside class="side-stack">
            <section class="panel">
              <h2>{t.storage}</h2>
              <p>{t.isolatedData}</p>
              <div class="divider"></div>
              <span class="caption">{t.dataFolder}</span>
              <p class="path">{state.data_home}</p>
              <button class="full" on:click={() => open("data")}
                ><Icon name="folder" size={15} />{t.openData}</button
              >
            </section>
            <section class="panel">
              <h3>Ocean-Rest</h3>
              <p>{t.capeServiceHint}</p>
              <button class="full" disabled={busy} on:click={() => run("api")}
                >{t.startCapeService}</button
              >
            </section>
          </aside>
        </div>
      {:else if tab === "settings"}
        <div class="page-heading">
          <div>
            <h1>{t.settings}</h1>
            <p>{t.settingsHint}</p>
          </div>
        </div>
        <div class="settings-layout">
          <section class="panel">
            <h3>{t.general}</h3>
            <div class="setting-row">
              <div><strong>{t.language}</strong></div>
              <select bind:value={preferences.locale} on:change={persist}
                ><option value="de">Deutsch</option><option value="en"
                  >English</option
                ></select
              >
            </div>
            {#each [["minimize", "minimizeHint"], ["restore", "restoreHint"], ["reducedMotion", "reducedMotionHint"]] as [property, description]}<div
                class="setting-row"
              >
                <div>
                  <strong>{t[property]}</strong>
                  <p>{t[description]}</p>
                </div>
                <button
                  class="toggle"
                  class:on={preferences[property]}
                  role="switch"
                  aria-checked={preferences[property]}
                  aria-label={t[property]}
                  on:click={() => {
                    preferences[property] = !preferences[property];
                    persist();
                  }}
                ></button>
              </div>{/each}
            <div class="setting-row">
              <div>
                <strong>{t.accent}</strong>
                <p>{t.accentHint}</p>
              </div>
              <div class="swatches">
                {#each accents as color}<button
                    style:background={color}
                    class:active={accent === color}
                    aria-label={`${t.accent}: ${color}`}
                    aria-pressed={accent === color}
                    on:click={() => {
                      preferences.accent = color;
                      persist();
                    }}
                  ></button>{/each}
              </div>
            </div>
            <div class="setting-row">
              <div>
                <strong>{t.dataFolder}</strong>
                <p>{t.isolatedData}</p>
              </div>
              <button on:click={() => open("data")}
                ><Icon name="folder" size={16} />{t.openData}</button
              >
            </div>
          </section>
          <aside class="panel settings-info">
            <span class="caption">Client</span>
            <h2>{t.nativeRuntime}</h2>
            <p>{t.nativeHint}</p>
            <dl>
              <div>
                <dt>Launcher</dt>
                <dd>0.1.0 Alpha</dd>
              </div>
              <div>
                <dt>Minecraft</dt>
                <dd>{version}</dd>
              </div>
              <div>
                <dt>{t.platform}</dt>
                <dd>Windows x64</dd>
              </div>
            </dl>
            <button class="full" on:click={() => navigate("activity")}
              >{t.diagnostics}</button
            >
          </aside>
        </div>
      {:else if tab === "activity"}
        <div class="page-heading">
          <div>
            <h1>{t.activity}</h1>
            <p>{t.activityHint}</p>
          </div>
          <div class="button-row">
            <button
              disabled={!state.lines?.length}
              on:click={() => copy(state.lines.join("\n"))}
              ><Icon name="copy" size={15} />{t.copyLog}</button
            ><button on:click={() => open("logs")}
              ><Icon name="folder" size={15} />{t.openLogs}</button
            >
          </div>
        </div>
        <div class="activity-layout">
          <section>
            <div class="panel activity-status">
              <div class="row">
                <span class="dot" class:working={busy}></span><strong
                  >{busy ? status : t.idle}</strong
                >
              </div>
              {#if busy && !playing}<button on:click={cancelOperation}
                  >{t.cancelOperation}</button
                >{/if}
            </div>
            <pre class="log" aria-live="polite">{state.lines?.join("\n") ||
                t.emptyLog}</pre>
            {#if notice}<div class="error-notice">
                <div>
                  <h3>{t.error}</h3>
                  <p>{notice}</p>
                  <button on:click={() => navigate("setup")}>{t.goSetup}</button
                  >
                </div>
              </div>{/if}
            <p class="path footnote">{state.data_home}</p>
          </section>
          <aside class="panel">
            <h3>{t.settings}</h3>
            <dl>
              <div>
                <dt>Launcher</dt>
                <dd>0.1.0</dd>
              </div>
              <div>
                <dt>Client</dt>
                <dd>Native</dd>
              </div>
              <div>
                <dt>Minecraft</dt>
                <dd>{version}</dd>
              </div>
              <div>
                <dt>{t.currentProfile}</dt>
                <dd>{selected?.name || "—"}</dd>
              </div>
            </dl>
            <button class="full" on:click={() => open("data")}
              >{t.openData}</button
            >
          </aside>
        </div>
      {/if}
    </main>
    <footer class="launchbar">
      <button class="launch-profile" on:click={() => navigate("profiles")}
        ><span class="game-icon small"><img src="/img/logo.png" alt="" /></span
        ><span
          ><strong
            >{selected?.name || "Turtle Client"}<Icon
              name="chevron"
              size={12}
            /></strong
          ><small>{version} · Native</small></span
        ></button
      >
      <div class="grow"></div>
      <div class="launch-status" role="status">
        <span class="dot" class:working={busy} class:not-ready={!ready}></span>
        <div>
          <strong>{status}</strong><small
            >{playing
              ? t.gameExit
              : account
                ? `${t.signedInAs} ${player}`
                : t.requiresJava}</small
          >
        </div>
      </div>
      <button
        class="primary play-button"
        disabled={busy || !state}
        on:click={() => play()}
        >{#if busy && !playing}<span class="spinner"></span>{:else}<Icon
            name="play"
            size={17}
          />{/if}{primaryLabel}</button
      >
    </footer>
  </div>
  {#if toast}<div class="toast" role="status">
      <Icon name="check" size={16} />{toast}
    </div>{/if}
  {#if dialog}<div
      class="modal-backdrop"
      role="presentation"
      on:click={(event) => {
        if (event.target === event.currentTarget && !busy) dialog = null;
      }}
    >
      <div
        class="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
      >
        <div class="row between">
          <span class="caption">Turtle Client</span><button
            class="icon-button"
            disabled={busy}
            aria-label={t.close}
            on:click={() => (dialog = null)}
            ><Icon name="close" size={18} /></button
          >
        </div>
        {#if dialog === "sign-out"}<h1 id="dialog-title">
            {t.signOutQuestion}
          </h1>
          <p>{t.signOutNote}</p>
          <div class="button-row modal-actions">
            <button disabled={busy} on:click={() => (dialog = null)}
              >{t.staySignedIn}</button
            ><button
              class="primary"
              disabled={busy}
              on:click={async () => {
                if (await run("sign-out", signOutId)) dialog = null;
              }}>{t.signOut}</button
            >
          </div>{:else}<h1 id="dialog-title">
            {dialog === "rename" ? t.renameProfile : t.newProfile}
          </h1>
          <p>{t.profileHint}</p>
          <form on:submit|preventDefault={saveProfile}>
            <label for="profile-name">{t.profileName}</label><input
              id="profile-name"
              bind:this={profileInput}
              bind:value={profileName}
              required
              maxlength="48"
              autocomplete="off"
              disabled={busy}
            /><span class="field-label">{t.version}</span>
            <div class="profile-version">
              <Icon name="check" size={19} /><span
                ><strong>Turtle Native {version}</strong><small
                  >Windows x64 · Alpha 0.1.0</small
                ></span
              ><span class="badge">{t.installed}</span>
            </div>
            <div class="button-row modal-actions">
              <button
                type="button"
                disabled={busy}
                on:click={() => (dialog = null)}>{t.cancel}</button
              ><button
                class="primary"
                type="submit"
                disabled={busy || !profileName.trim()}
                >{busy
                  ? t.pleaseWait
                  : dialog === "rename"
                    ? t.save
                    : t.create}</button
              >
            </div>
          </form>{/if}
      </div>
    </div>{/if}
</div>
