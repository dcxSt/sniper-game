import init, { Game } from './pkg/sniper_game.js';

const $ = (id) => document.getElementById(id);
const glc = $('gl');
const hudc = $('hud');
const overlays = { menu: $('menu'), pause: $('pause'), dead: $('dead'), won: $('won'), error: $('error') };
const NOLOCK = new URLSearchParams(location.search).has('nolock'); // for automated testing

let game = null;
let lastState = -1;
let pendingReload = false;

// ---- dev convenience (served by ./serve.py): reload when a newer build lands ----------------
let buildStamp = null;
let pollTimer = null;
async function pollBuild() {
  try {
    const r = await fetch('/__build', { cache: 'no-store' });
    if (!r.ok) { clearInterval(pollTimer); return; } // static hosting: no dev server, stop asking
    const s = await r.text();
    if (buildStamp === null) buildStamp = s;
    else if (s !== buildStamp) {
      if (game && game.state() === 1) {   // mid-mission: don't yank the page, reload at the next pause
        pendingReload = true;
        $('toast').classList.add('show');
      } else {
        location.reload();
      }
    }
  } catch { clearInterval(pollTimer); /* static hosting: no /__build, nothing to do */ }
}
pollTimer = setInterval(pollBuild, 1200);
pollBuild();
fetch('pkg/BUILD', { cache: 'no-store' }).then((r) => r.ok ? r.text() : '').then((t) => {
  if (t.trim()) for (const el of document.querySelectorAll('.build')) el.textContent = 'build ' + t.trim();
}).catch(() => {});

// the same controls reference is shown on the title screen and when you press Esc
for (const el of document.querySelectorAll('[data-controls]')) {
  el.appendChild($('tpl-controls').content.cloneNode(true));
}

function fail(msg) {
  console.error(msg);
  $('error-msg').textContent = String(msg);
  show('error');
}

function show(name) {
  for (const [k, el] of Object.entries(overlays)) el.classList.toggle('show', k === name);
}

function fit() {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = Math.max(2, Math.floor(innerWidth * dpr));
  const h = Math.max(2, Math.floor(innerHeight * dpr));
  for (const c of [glc, hudc]) { c.width = w; c.height = h; }
  if (game) game.resize(w, h);
}

const locked = () => NOLOCK || document.pointerLockElement === glc;
const lock = () => { if (!NOLOCK) glc.requestPointerLock?.(); };

async function boot() {
  fit();
  try {
    await init();
    game = new Game(glc, hudc);
  } catch (e) {
    fail(e && e.message ? e.message : e);
    return;
  }
  window.game = game; // handy for debugging in the console
  fit();
  $('btn-start').disabled = false;
  $('load-hint').textContent = 'your mouse is captured while you play · Esc to pause';

  const unfocus = () => document.activeElement?.blur?.(); // so Space/Enter never re-press a button mid-game
  const begin = () => { unfocus(); game.start(); lock(); };
  $('btn-start').onclick = begin;
  $('btn-retry').onclick = begin;
  $('btn-again').onclick = begin;
  $('btn-restart-p').onclick = begin;
  const resume = () => { unfocus(); game.resume(); lock(); };
  $('btn-resume').onclick = resume;

  addEventListener('resize', fit);
  addEventListener('blur', () => { if (game.state() === 1 && !NOLOCK) document.exitPointerLock?.(); });

  document.addEventListener('pointerlockchange', () => {
    if (!locked() && game.state() === 1) game.pause();
  });
  document.addEventListener('pointerlockerror', () => {
    // couldn't capture the mouse: stay paused so the player can click resume
    if (game.state() === 1) game.pause();
  });

  const GAME_KEYS = new Set(['Space','ArrowUp','ArrowDown','ArrowLeft','ArrowRight','ControlLeft','Tab']);
  addEventListener('keydown', (e) => {
    const s = game.state();
    if (s === 2 && e.code === 'Enter') { e.preventDefault(); resume(); return; }
    if (s !== 1) return;
    if (e.code === 'Escape') { game.pause(); return; } // (normally the pointer-lock exit does this)
    if (GAME_KEYS.has(e.code)) e.preventDefault();
    if (!e.repeat) game.key(e.code, true);
  });
  addEventListener('keyup', (e) => { game.key(e.code, false); });
  addEventListener('mousemove', (e) => {
    if (game.state() === 1 && locked()) game.mouse_move(e.movementX, e.movementY);
  });
  addEventListener('mousedown', (e) => {
    if (game.state() === 1 && locked()) { e.preventDefault(); game.mouse_button(e.button, true); }
  });
  addEventListener('mouseup', (e) => { game.mouse_button(e.button, false); });
  // trackpads send a long burst of wheel events per swipe: count a whole burst as ONE notch
  let lastWheel = 0;
  addEventListener('wheel', (e) => {
    if (game.state() !== 1 || !locked()) return;
    e.preventDefault();
    const now = performance.now();
    const fresh = now - lastWheel > 100;
    lastWheel = now;
    if (fresh && Math.abs(e.deltaY) > 0.5) game.wheel(Math.sign(e.deltaY));
  }, { passive: false });
  addEventListener('contextmenu', (e) => e.preventDefault());

  const loop = (t) => {
    game.frame(t);
    const s = game.state();
    if (s !== lastState) {
      lastState = s;
      if (pendingReload && s !== 1) { location.reload(); return; }
      if (s === 0) show('menu');
      else if (s === 1) show(null);
      else if (s === 2) { $('pause-obj').textContent = game.objective_text(); show('pause'); }
      else if (s === 3) {
        $('dead-sum').textContent = game.summary();
        show('dead');
        document.exitPointerLock?.();
      } else if (s === 4) {
        $('won-sum').textContent = game.summary();
        show('won');
        document.exitPointerLock?.();
      }
    }
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

$('btn-start').disabled = true;
boot();
