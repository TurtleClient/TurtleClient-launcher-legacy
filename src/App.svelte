<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { words } from './native-i18n.js';
  let locale = localStorage.getItem('turtle-language') || 'de';
  let tab = 'play', state = null, error = '', minecraftPath = '', polling = false;
  $: t = words[locale] || words.de;
  $: busy = !!state?.action;
  $: accounts = state?.accounts?.accounts || [];
  $: player = accounts.find(a => a.selected)?.name || t.noAccount;
  $: ready = !!(state?.installed && state?.prepared && state?.accounts?.owner);
  $: code = [...(state?.lines || [])].reverse().find(l => l.includes('To sign in, open'))?.match(/code\s+([A-Z0-9-]+)/)?.[1];
  $: notice = error || state?.error;
  async function refresh() {
    if (polling) return;
    polling = true;
    try { state = await invoke('get_state'); } catch(e) { error = String(e); }
    finally { polling = false; }
  }
  async function act(action, value = null) {
    error = '';
    try { await invoke('start_action', {action, value}); await refresh(); }
    catch(e) { error = String(e); }
  }
  async function open(location) { try { await invoke('open_location', {location}); } catch(e) { error = String(e); } }
  function language() { locale = locale === 'de' ? 'en' : 'de'; localStorage.setItem('turtle-language', locale); }
  onMount(() => {
    let active = true;
    (async () => { await refresh(); if (active && state && !state.installed) await act('install'); })();
    const timer = setInterval(refresh, 1800);
    return () => { active = false; clearInterval(timer); };
  });
</script>

<div class="shell">
  <aside>
    <div class="brand"><img src="/img/logo.png" alt="" /><div>TURTLE<span>CLIENT</span></div></div>
    <div class="nav-label">{t.workspace}</div>
    <nav aria-label={t.workspace}>
      {#each [['play','◈'],['account','◎'],['setup','⚙'],['activity','≡']] as [page, icon]}
        <button class:active={tab === page} on:click={() => tab = page}><span class="nav-icon">{icon}</span>{t[page]}</button>
      {/each}
    </nav>
    <div class="sidebar-bottom"><span class="green-dot"></span>{t.native}<small>26.3 · Windows x64</small><button class="language" on:click={language}>{locale === 'de' ? 'DE / EN' : 'EN / DE'}</button></div>
  </aside>
  <main>
    <header><div class="breadcrumb">Turtle Launcher <span>/</span> {t[tab]}</div><span class="version-badge">0.1.0 <i>ALPHA</i></span></header>
    <div class="content">
      {#if notice}<div role="alert" class="error"><strong>{t.error}</strong><p>{notice}</p><button on:click={() => open('logs')}>{t.openLogs} ↗</button></div>{/if}
      {#if tab === 'play'}
        <section class="hero">
          <div class="hero-tag"><span class="green-dot"></span> TURTLE NATIVE</div>
          <h1>{t.heroA}<br/><em>{t.heroB}</em></h1>
          <p>{t.heroText}</p>
          <div class="hero-bottom"><span>MINECRAFT 26.3</span><span>RUST / HORIZON</span></div>
        </section>
        <div class="launch-card">
          <div><span class="eyebrow">{t.selected}</span><h2>TurtleClient <span class="tag">26.3</span></h2><p><span class:green-dot={ready} class:muted-dot={!ready}></span>{busy ? t.working[state.action] || state.action : ready ? t.ready : t.needsSetup}</p></div>
          <button class="primary launch" disabled={busy || !state} on:click={() => ready ? act('play') : tab = !state?.accounts?.owner ? 'account' : 'setup'}>{busy ? state.action === 'play' ? t.running : t.pleaseWait : ready ? t.start : t.getReady}<span>↗</span></button>
        </div>
        <div class="feature-grid">
          <article><kbd>B</kbd><h3>{t.emotes}</h3><p>{t.emoteText}</p></article>
          <article><kbd>C</kbd><h3>{t.zoom}</h3><p>{t.zoomText}</p></article>
          <article><span class="cape-symbol">◧</span><h3>{t.capes}</h3><p>{t.capeText}</p></article>
        </div>
        <p class="fineprint">{t.alphaNote}</p>
      {:else if tab === 'account'}
        <div class="page-heading"><span class="eyebrow">MICROSOFT / MINECRAFT</span><h1>{t.accountTitle}</h1><p>{t.accountText}</p></div>
        <section class="panel"><h2>{t.signedIn}</h2>
          {#if accounts.length === 0}<p class="empty">{t.noAccount}</p>{/if}
          {#each accounts as account}<div class="account-row"><div class="avatar">{account.name.slice(0,1)}</div><div><strong>{account.name}</strong><small>{account.selected ? t.selectedAccount : t.availableAccount}</small></div><button disabled={busy || account.selected} on:click={() => act('select', account.uuid)}>{t.select}</button><button disabled={busy} on:click={() => act('sign-out', account.uuid)}>{t.signOut}</button></div>{/each}
          <button class="primary" disabled={busy || state?.accounts?.sign_in_configured === false} on:click={() => act('sign-in')}>{t.signIn} ↗</button>
          {#if state?.accounts?.sign_in_configured === false}<p class="error">{t.noAuth}</p>{/if}
        </section>
        {#if state?.action === 'sign-in'}<section class="panel login-code"><span class="eyebrow">{t.deviceCode}</span><strong>{code || '··· ···'}</strong><p>{t.codeText}</p><button class="primary" on:click={() => invoke('open_sign_in').catch(e => error = String(e))}>{t.openMicrosoft} ↗</button><button class="text-button" on:click={() => invoke('cancel_action')}>{t.cancel}</button></section>{/if}
        <p class="fineprint">{t.accountNote}</p>
      {:else if tab === 'setup'}
        <div class="page-heading"><span class="eyebrow">TURTLECLIENT 26.3</span><h1>{t.setupTitle}</h1><p>{t.setupText}</p></div>
        <section class="panel step"><span class="step-number">01</span><div><h2>{t.nativeInstall}</h2><p>{state?.installed ? t.installed : t.installText}</p><button disabled={busy} on:click={() => act('install')}>{t.checkInstall}</button></div><span class="state-label">{state?.installed ? '✓' : '—'}</span></section>
        <section class="panel step"><span class="step-number">02</span><div class="grow"><h2>{t.resources}</h2><p>{state?.prepared ? t.resourcesReady : t.resourceText}</p><label for="minecraft-path">{t.minecraftFolder}</label><input id="minecraft-path" bind:value={minecraftPath} placeholder={t.automatic}/><div class="button-row"><button class="primary" disabled={busy} on:click={() => act('prepare', minecraftPath)}>{t.prepare}</button><button disabled={busy || !state?.accounts?.owner} on:click={() => act('download')}>{t.download}</button></div></div><span class="state-label">{state?.prepared ? '✓' : '—'}</span></section>
        <section class="panel step"><span class="step-number">03</span><div><h2>Ocean-Rest</h2><p>{t.apiText}</p><button disabled={busy} on:click={() => act('api')}>{t.startApi}</button></div></section>
        {#if busy && state.action !== 'play'}<button class="text-button" on:click={() => invoke('cancel_action')}>{t.cancel}</button>{/if}
      {:else}
        <div class="page-heading"><span class="eyebrow">{t.diagnostics}</span><h1>{t.activity}</h1><p>{busy ? t.working[state.action] || state.action : t.idle}</p></div>
        <div class="button-row"><button on:click={() => open('logs')}>{t.openLogs} ↗</button><button on:click={() => open('data')}>{t.openData} ↗</button></div>
        <pre aria-live="polite">{(state?.lines || []).join('\n') || t.emptyLog}</pre><p class="fineprint path">{state?.data_home || ''}</p>
      {/if}
    </div>
    <footer><span class="footer-status"><span class="green-dot"></span>{busy ? t.working[state.action] || state.action : t.native}</span><span>{player} <b>·</b> {t.localEmotes}</span></footer>
  </main>
</div>
