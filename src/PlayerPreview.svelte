<script>
  import { onMount } from "svelte";
  export let emote = "";
  export let back = false;
  export let cape = "";
  export let skin = null;
  export let reducedMotion = false;
  export let label = "";
  export let unavailable = "";
  let container,
    canvas,
    viewer,
    library,
    failed = false,
    systemReduced = false;
  let pointer = null,
    dragging = false,
    visible = true,
    documentVisible = true;
  let textureVersion = 0;

  // An original neutral texture, used only while signed out or when no skin is available.
  // No Mojang textures are distributed with the launcher.
  function placeholder() {
    const texture = document.createElement("canvas");
    texture.width = texture.height = 64;
    const context = texture.getContext("2d");
    context.fillStyle = "#62666c";
    context.fillRect(0, 0, 64, 32);
    context.fillRect(16, 48, 32, 16);
    context.fillStyle = "#a4a6aa";
    context.fillRect(0, 0, 32, 16);
    context.fillStyle = "#35373c";
    context.fillRect(0, 16, 16, 16);
    context.fillRect(16, 48, 16, 16);
    context.fillStyle = "#34363b";
    context.fillRect(10, 11, 2, 2);
    context.fillRect(14, 11, 2, 2);
    return texture;
  }
  async function loadAppearance(target, appearance, capeTexture) {
    const version = ++textureVersion;
    try {
      await target.loadSkin(appearance?.skin || placeholder(), {
        model: appearance?.model || "default",
      });
      if (version !== textureVersion || target.disposed) return;
      await target.loadCape(capeTexture || appearance?.cape || null);
      failed = false;
    } catch {
      if (version === textureVersion && !target.disposed) {
        target.loadSkin(placeholder());
        target.loadCape(null);
        failed = true;
      }
    }
  }
  function animate(target, selectedEmote, reduce) {
    const animation = new library.IdleAnimation();
    animation.addAnimation((player, time, _id) => {
      const head = player.skin.head.rotation;
      // Damp the head toward the cursor; while dragging the body, retain a relaxed forward gaze.
      const yaw =
        !dragging && pointer ? pointer.x * 0.55 : Math.sin(time * 0.45) * 0.18;
      const pitch =
        !dragging && pointer ? pointer.y * 0.3 : Math.sin(time * 0.6) * 0.05;
      head.y += (yaw - head.y) * 0.08;
      head.x += (pitch - head.x) * 0.08;
      if (!selectedEmote) return;
      const left = player.skin.leftArm.rotation,
        right = player.skin.rightArm.rotation;
      if (selectedEmote === "wave") {
        right.z = -2.5 + Math.sin(time * 7) * 0.25;
        right.x = -0.15;
      }
      if (selectedEmote === "point") {
        right.x = -Math.PI / 2;
        right.z = -0.1;
      }
      if (selectedEmote === "cheer") {
        right.z = -2.8;
        left.z = 2.8;
        player.position.y = Math.max(0, Math.sin(time * 6)) * 1.2;
      }
      if (selectedEmote === "clap") {
        right.x = left.x = -1.25;
        right.z = -0.5 + Math.sin(time * 9) * 0.2;
        left.z = -right.z;
      }
      if (selectedEmote === "dance") {
        player.rotation.z = Math.sin(time * 5) * 0.1;
        left.x = Math.sin(time * 5) * 0.6;
        right.x = -left.x;
        player.skin.leftLeg.rotation.x = Math.sin(time * 5) * 0.25;
        player.skin.rightLeg.rotation.x = -player.skin.leftLeg.rotation.x;
      }
      if (selectedEmote === "laugh") {
        head.x = 0.15 + Math.sin(time * 12) * 0.06;
        player.rotation.x = 0.08 + Math.sin(time * 12) * 0.02;
      }
    });
    const yaw = target.playerObject.rotation.y;
    target.animation = animation;
    target.playerObject.rotation.y = yaw;
    animation.paused = reduce;
    if (reduce) target.playerObject.skin.head.rotation.set(0, 0, 0);
  }
  function orientation(target, rear) {
    target.playerObject.rotation.y = rear ? Math.PI + 0.3 : 0.3;
  }
  function pause(target, isVisible, foreground) {
    target.renderPaused = !isVisible || !foreground;
  }
  function keyboard(event) {
    if (!viewer) return;
    if (["ArrowLeft", "ArrowRight", "Home"].includes(event.key)) {
      event.preventDefault();
      if (event.key === "Home") {
        viewer.controls.reset();
        orientation(viewer, false);
      } else
        viewer.playerObject.rotation.y +=
          event.key === "ArrowLeft" ? -0.25 : 0.25;
    }
  }
  function track(event) {
    const bounds = canvas.getBoundingClientRect();
    pointer = {
      x: ((event.clientX - bounds.left) / bounds.width) * 2 - 1,
      y: ((event.clientY - bounds.top) / bounds.height) * 2 - 1,
    };
  }
  $: if (viewer) loadAppearance(viewer, skin, cape);
  $: if (viewer) animate(viewer, emote, reducedMotion || systemReduced);
  $: if (viewer) orientation(viewer, back);
  $: if (viewer) pause(viewer, visible, documentVisible);
  onMount(() => {
    let alive = true,
      resize,
      intersection;
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const motion = () => (systemReduced = media.matches);
    const visibility = () => (documentVisible = !document.hidden);
    motion();
    visibility();
    media.addEventListener("change", motion);
    document.addEventListener("visibilitychange", visibility);
    (async () => {
      try {
        library = await import("skinview3d");
        if (!alive) return;
        const target = new library.SkinViewer({
          canvas,
          width: container.clientWidth,
          height: container.clientHeight,
          zoom: 0.82,
          pixelRatio: Math.min(window.devicePixelRatio, 2),
        });
        target.controls.enableZoom = false;
        target.controls.enablePan = false;
        target.controls.minPolarAngle = Math.PI * 0.27;
        target.controls.maxPolarAngle = Math.PI * 0.72;
        target.cameraLight.intensity = 0.8;
        target.globalLight.intensity = 1.7;
        viewer = target;
        resize = new ResizeObserver(([entry]) => {
          if (
            !target.disposed &&
            entry.contentRect.width &&
            entry.contentRect.height
          )
            target.setSize(entry.contentRect.width, entry.contentRect.height);
        });
        resize.observe(container);
        intersection = new IntersectionObserver(
          ([entry]) => (visible = entry.isIntersecting),
        );
        intersection.observe(container);
      } catch {
        failed = true;
      }
    })();
    return () => {
      alive = false;
      textureVersion++;
      resize?.disconnect();
      intersection?.disconnect();
      media.removeEventListener("change", motion);
      document.removeEventListener("visibilitychange", visibility);
      viewer?.dispose();
    };
  });
</script>

<div class="player-model" bind:this={container}>
  <canvas
    bind:this={canvas}
    tabindex="0"
    aria-label={label}
    on:keydown={keyboard}
    on:pointermove={track}
    on:pointerleave={() => {
      pointer = null;
      dragging = false;
    }}
    on:pointerdown={() => (dragging = true)}
    on:pointerup={() => (dragging = false)}
  ></canvas>
  {#if failed}<span class="renderer-error" role="status">{unavailable}</span
    >{/if}
</div>

<style>
  .player-model {
    position: relative;
    width: 230px;
    height: 344px;
    flex-shrink: 0;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
    touch-action: none;
  }
  canvas:active {
    cursor: grabbing;
  }
  canvas:focus-visible {
    outline: 2px solid var(--accent, #39d878);
    outline-offset: -3px;
    border-radius: 8px;
  }
  .renderer-error {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    text-align: center;
    padding: 20px;
    color: #9b9ba6;
    font-size: 11px;
    pointer-events: none;
  }
</style>
