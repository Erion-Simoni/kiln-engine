// All URLs remain relative to this exported module, including nested deployments.
const gate = document.getElementById('gate');
const status = document.getElementById('status');
const button = document.getElementById('start');
const canvas = document.getElementById('game');
for (const type of ['mousedown','mouseup','pointerdown','pointerup','touchstart','touchend','keydown','keyup']) {
  gate.addEventListener(type, event => event.stopPropagation());
}
function fail(message, error) {
  console.error(message, error);
  status.textContent = `${message} ${String(error?.message || error || '')}`;
  gate.dataset.state = 'error';
  button.disabled = false;
  button.textContent = 'Reload game';
  button.onclick = () => location.reload();
}
async function load() {
  if (!['http:','https:'].includes(location.protocol)) {
    status.textContent = 'Serve this game folder over HTTP or HTTPS, then open its address in your browser.';
    return;
  }
  try {
    gate.dataset.state = 'loading';
    status.textContent = 'Loading game runtime…';
    const runtime = await import(new URL('./runtime/kiln_web.js', import.meta.url));
    await runtime.default();
    status.textContent = 'Loading project and assets…';
    await runtime.prepare_game('game', new URL('./project/kiln.toml', import.meta.url).href);
    gate.dataset.state = 'ready';
    status.textContent = 'Ready to play. Sound starts when you press Play.';
    button.textContent = 'Play';
    button.disabled = false;
    button.onclick = async event => {
      event.stopPropagation();
      button.disabled = true;
      status.textContent = 'Starting game…';
      try {
        await runtime.start_game();
        canvas.style.visibility = 'visible';
        canvas.setAttribute('aria-hidden','false');
        gate.hidden = true;
        gate.dataset.state = 'playing';
        canvas.focus();
      } catch (error) {
        status.textContent = `Could not start game: ${String(error?.message || error)}. Try again.`;
        button.disabled = false;
      }
    };
  } catch (error) { fail('Could not load game.', error); }
}
load();
