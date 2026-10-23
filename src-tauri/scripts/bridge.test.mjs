// Runs bridge.js the way an engine does: as a plain script, against a window.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('./bridge.js', import.meta.url), 'utf8');

const KNOWN_ACTIONS = [
  'play', 'pause', 'stop', 'seekbackward', 'seekforward', 'seekto', 'previoustrack', 'nexttrack', 'skipad',
];

// Mirrors the engines: accessors on the prototype, unknown actions throw.
class MediaSession {
  #metadata = null;
  #playbackState = 'none';
  native = new Map();

  get metadata() { return this.#metadata; }
  set metadata(value) { this.#metadata = value; }
  get playbackState() { return this.#playbackState; }
  set playbackState(value) { this.#playbackState = value; }

  setActionHandler(action, handler) {
    if (!KNOWN_ACTIONS.includes(action)) {
      throw new TypeError(`The provided value '${action}' is not a valid enum value of type MediaSessionAction.`);
    }
    if (handler) {
      this.native.set(action, handler);
    } else {
      this.native.delete(action);
    }
  }
}

class Clock {
  now = 0;
  #timers = [];
  #nextId = 1;

  setTimeout = (callback, delay) => this.#add(callback, delay, null);
  setInterval = (callback, delay) => this.#add(callback, delay, delay);

  #add(callback, delay, every) {
    const id = this.#nextId++;
    this.#timers.push({ id, at: this.now + delay, callback, every });
    return id;
  }

  async advance(ms) {
    const until = this.now + ms;
    for (;;) {
      const due = this.#timers.filter((t) => t.at <= until).sort((a, b) => a.at - b.at || a.id - b.id)[0];
      if (!due) break;
      this.now = due.at;
      if (due.every === null) {
        this.#timers = this.#timers.filter((t) => t !== due);
      } else {
        due.at += due.every;
      }
      due.callback();
      await settle();
    }
    this.now = until;
    await settle();
  }
}

const settle = () => new Promise((resolve) => setImmediate(resolve));

function setup({ native = true, ipc = true, invoke } = {}) {
  const clock = new Clock();
  const calls = [];
  const win = {
    navigator: {},
    setTimeout: clock.setTimeout,
    setInterval: clock.setInterval,
  };
  win.window = win;
  if (native) {
    win.navigator.mediaSession = new MediaSession();
  }
  const record = (command, args) => {
    calls.push({ command, args: structuredClone(args) });
    return Promise.resolve();
  };
  if (ipc) {
    win.__TAURI_INTERNALS__ = { invoke: invoke ?? record };
  }
  const context = vm.createContext(win);
  const run = () => vm.runInContext(source, context);
  run();
  return { win, clock, calls, record, run, session: win.navigator.mediaSession };
}

const metadata = (fields) => ({ title: '', artist: '', album: '', artwork: [], ...fields });
const lastSnapshot = (calls) => calls.at(-1).args.snapshot;

test('002 AC-1: installs once and a second run changes nothing', () => {
  const { win, session, run } = setup();
  const bridge = win.__deezerTauri;
  const wrapped = session.setActionHandler;
  run();
  assert.equal(win.__deezerTauri, bridge);
  assert.equal(session.setActionHandler, wrapped);
  assert.equal(bridge.version, 1);
});

test('002 AC-1: a popup does not get the bridge', () => {
  const clock = new Clock();
  const win = { navigator: { mediaSession: new MediaSession() }, setTimeout: clock.setTimeout, setInterval: clock.setInterval, opener: {} };
  win.window = win;
  vm.runInContext(source, vm.createContext(win));
  assert.equal(win.__deezerTauri, undefined);
  assert.equal(Object.hasOwn(win.navigator.mediaSession, 'setActionHandler'), false);
});

test('002 AC-1: the bridge cannot be replaced or extended by the page', () => {
  const { win } = setup();
  const bridge = win.__deezerTauri;
  assert.throws(() => { win.__deezerTauri = { dispatch: () => true }; }, TypeError);
  assert.throws(() => { delete win.__deezerTauri; }, TypeError);
  assert.equal(win.__deezerTauri, bridge);
  assert.ok(Object.isFrozen(bridge));
  assert.deepEqual(Object.keys(bridge).sort(), ['dispatch', 'version']);
});

test('002 AC-1: handlers reach the engine, are remembered and forgotten with null', async () => {
  const { session, clock, calls } = setup();
  const play = () => {};
  session.setActionHandler('play', play);
  session.setActionHandler('nexttrack', () => {});
  assert.equal(session.native.get('play'), play);
  await clock.advance(100);
  assert.deepEqual(lastSnapshot(calls).actions, ['nexttrack', 'play']);

  session.setActionHandler('play', null);
  assert.equal(session.native.has('play'), false);
  await clock.advance(100);
  assert.deepEqual(lastSnapshot(calls).actions, ['nexttrack']);
});

test('002 AC-1: an action the engine rejects still throws and is not remembered', async () => {
  const { session, clock, calls } = setup();
  assert.throws(() => session.setActionHandler('teleport', () => {}), TypeError);
  await clock.advance(1000);
  assert.equal(calls.length, 1);
  assert.deepEqual(lastSnapshot(calls).actions, []);
});

test('002 AC-2: an engine without mediaSession gets a minimal one that feeds the bridge', async () => {
  const { win, clock, calls } = setup({ native: false });
  const session = win.navigator.mediaSession;
  assert.equal(typeof session.setActionHandler, 'function');
  assert.equal(typeof session.setPositionState, 'function');
  assert.equal(session.playbackState, 'none');
  assert.equal(typeof win.MediaMetadata, 'function');

  session.metadata = new win.MediaMetadata({ title: 'Seize the Day', artist: 'Avenged Sevenfold' });
  session.playbackState = 'playing';
  session.setActionHandler('pause', () => {});
  await clock.advance(100);
  assert.deepEqual(lastSnapshot(calls), {
    title: 'Seize the Day',
    artist: 'Avenged Sevenfold',
    album: '',
    artwork: null,
    playbackState: 'playing',
    actions: ['pause'],
  });
});

test('002 AC-3: changes are sent once, together, within 250 ms', async () => {
  const { session, clock, calls } = setup();
  session.metadata = metadata({ title: 'One', artist: 'A', album: 'B' });
  session.playbackState = 'playing';
  session.setActionHandler('play', () => {});
  await clock.advance(99);
  assert.equal(calls.length, 0);
  await clock.advance(151);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].command, 'report_media');
  assert.deepEqual(lastSnapshot(calls), {
    title: 'One', artist: 'A', album: 'B', artwork: null, playbackState: 'playing', actions: ['play'],
  });
});

test('002 AC-3: an equal snapshot is not sent again', async () => {
  const { session, clock, calls } = setup();
  session.metadata = metadata({ title: 'One' });
  await clock.advance(100);
  session.metadata = metadata({ title: 'One' });
  session.playbackState = 'none';
  await clock.advance(3000);
  assert.equal(calls.length, 1);
});

test('002 AC-3: the one-second check sends what the hooks missed', async () => {
  const { session, clock, calls } = setup();
  const current = metadata({ title: 'One' });
  session.metadata = current;
  await clock.advance(100);
  current.title = 'Changed in place';
  await clock.advance(1000);
  assert.equal(calls.length, 2);
  assert.equal(lastSnapshot(calls).title, 'Changed in place');
});

test('002 AC-3: an engine whose properties cannot be hooked is followed by the check', async () => {
  const clock = new Clock();
  const calls = [];
  const session = { metadata: null, playbackState: 'none', setActionHandler() {} };
  Object.defineProperty(session, 'metadata', { value: null, writable: true, configurable: false });
  const win = {
    navigator: { mediaSession: session },
    setTimeout: clock.setTimeout,
    setInterval: clock.setInterval,
    __TAURI_INTERNALS__: { invoke: (command, args) => { calls.push(args.snapshot); return Promise.resolve(); } },
  };
  win.window = win;
  vm.runInContext(source, vm.createContext(win));
  session.metadata = metadata({ title: 'Unhooked' });
  await clock.advance(1000);
  assert.equal(calls.at(-1).title, 'Unhooked');
});

test('002 AC-3: strings are capped before they are sent', async () => {
  const { session, clock, calls } = setup();
  session.metadata = metadata({ title: 'x'.repeat(5000) });
  await clock.advance(100);
  assert.equal(lastSnapshot(calls).title.length, 1024);
});

test('002 AC-4: the artwork is the largest declared size', async () => {
  const { session, clock, calls } = setup();
  session.metadata = metadata({
    artwork: [
      { src: 'https://e.test/250.jpg', sizes: '250x250' },
      { src: 'https://e.test/1000.jpg', sizes: '1000x1000' },
      { src: 'https://e.test/500.jpg', sizes: '96x96 500x500' },
      { src: '', sizes: '2000x2000' },
    ],
  });
  await clock.advance(100);
  assert.equal(lastSnapshot(calls).artwork, 'https://e.test/1000.jpg');
});

test('002 AC-4: without sizes the last image wins, "any" beats any size, none gives null', async () => {
  const { session, clock, calls } = setup();
  session.metadata = metadata({ artwork: [{ src: 'https://e.test/a.jpg' }, { src: 'https://e.test/b.jpg' }] });
  await clock.advance(100);
  assert.equal(lastSnapshot(calls).artwork, 'https://e.test/b.jpg');

  session.metadata = metadata({
    artwork: [{ src: 'https://e.test/v.svg', sizes: 'any' }, { src: 'https://e.test/big.jpg', sizes: '4000x4000' }],
  });
  await clock.advance(100);
  assert.equal(lastSnapshot(calls).artwork, 'https://e.test/v.svg');

  session.metadata = metadata({ title: 'No art', artwork: [] });
  await clock.advance(100);
  assert.equal(lastSnapshot(calls).artwork, null);
});

test('002 AC-5: a snapshot due before IPC exists is sent once IPC appears', async () => {
  const { win, session, clock, record, calls } = setup({ ipc: false });
  session.metadata = metadata({ title: 'Early' });
  await clock.advance(1000);
  assert.equal(calls.length, 0);
  win.__TAURI_INTERNALS__ = { invoke: record };
  await clock.advance(1000);
  assert.equal(calls.length, 1);
  assert.equal(lastSnapshot(calls).title, 'Early');
});

test('002 AC-5: a failing invoke is retried by the check, never in a loop', async () => {
  let attempts = 0;
  let failing = true;
  const { session, clock } = setup({
    invoke: () => {
      attempts++;
      return failing ? Promise.reject(new Error('denied')) : Promise.resolve();
    },
  });
  session.metadata = metadata({ title: 'Retry' });
  await clock.advance(3000);
  assert.equal(attempts, 4);
  failing = false;
  await clock.advance(1000);
  assert.equal(attempts, 5);
  await clock.advance(5000);
  assert.equal(attempts, 5);
});

test('002 AC-5: an invoke that throws synchronously is treated as unavailable', async () => {
  let attempts = 0;
  const { session, clock } = setup({ invoke: () => { attempts++; throw new Error('boom'); } });
  session.metadata = metadata({ title: 'Throws' });
  await clock.advance(2000);
  assert.equal(attempts, 3);
});

test('002 AC-6: dispatch calls the registered handler with the action', () => {
  const { win, session } = setup();
  const seen = [];
  for (const action of ['play', 'pause', 'nexttrack', 'previoustrack']) {
    session.setActionHandler(action, (details) => seen.push(details.action));
  }
  for (const action of ['play', 'pause', 'nexttrack', 'previoustrack']) {
    assert.equal(win.__deezerTauri.dispatch(action), true, action);
  }
  assert.deepEqual(seen, ['play', 'pause', 'nexttrack', 'previoustrack']);
});

test('002 AC-6: toggle pauses while playing and plays otherwise', () => {
  const { win, session } = setup();
  const seen = [];
  session.setActionHandler('play', () => seen.push('play'));
  session.setActionHandler('pause', () => seen.push('pause'));
  session.playbackState = 'playing';
  win.__deezerTauri.dispatch('toggle');
  session.playbackState = 'paused';
  win.__deezerTauri.dispatch('toggle');
  session.playbackState = 'none';
  win.__deezerTauri.dispatch('toggle');
  assert.deepEqual(seen, ['pause', 'play', 'play']);
});

test('002 AC-6: unknown, unregistered or failing actions return false', () => {
  const { win, session } = setup();
  let seeks = 0;
  session.setActionHandler('seekto', () => seeks++);
  session.setActionHandler('pause', () => { throw new Error('player broke'); });
  assert.equal(win.__deezerTauri.dispatch('seekto'), false);
  assert.equal(win.__deezerTauri.dispatch('nexttrack'), false);
  assert.equal(win.__deezerTauri.dispatch('pause'), false);
  assert.equal(win.__deezerTauri.dispatch('__proto__'), false);
  assert.equal(win.__deezerTauri.dispatch(undefined), false);
  assert.equal(seeks, 0);
});
