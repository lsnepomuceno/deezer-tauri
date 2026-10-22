// Media bridge of Deezer Tauri (specs/features/002-media-bridge).
// A plain ES2019 script injected before the page's own code, in the main frame only.
(function (win) {
  'use strict';

  if (win.__deezerTauri) {
    return;
  }

  var VERSION = 1;
  var DISPATCHABLE = ['play', 'pause', 'nexttrack', 'previoustrack'];
  var MAX_TEXT = 1024;
  var DEBOUNCE_MS = 100;
  var CHECK_MS = 1000;

  var session = win.navigator.mediaSession || installMediaSession(win);
  var handlers = Object.create(null);
  var lastSent = null;
  var inFlight = false;
  var flushAgain = false;
  var timer = null;

  wrapSetActionHandler();
  hookProperty('metadata');
  hookProperty('playbackState');
  win.setInterval(flush, CHECK_MS);

  Object.defineProperty(win, '__deezerTauri', {
    value: Object.freeze({ version: VERSION, dispatch: dispatch }),
    writable: false,
    configurable: false,
    enumerable: false
  });

  function installMediaSession(target) {
    var state = { metadata: null, playbackState: 'none' };
    var polyfill = {
      setActionHandler: function () {},
      setPositionState: function () {}
    };
    ['metadata', 'playbackState'].forEach(function (name) {
      Object.defineProperty(polyfill, name, {
        configurable: true,
        enumerable: true,
        get: function () { return state[name]; },
        set: function (value) { state[name] = value; }
      });
    });
    Object.defineProperty(target.navigator, 'mediaSession', {
      value: polyfill,
      configurable: true,
      enumerable: true
    });
    if (typeof target.MediaMetadata !== 'function') {
      target.MediaMetadata = function MediaMetadata(init) {
        init = init || {};
        this.title = init.title || '';
        this.artist = init.artist || '';
        this.album = init.album || '';
        this.artwork = init.artwork || [];
      };
    }
    return polyfill;
  }

  function wrapSetActionHandler() {
    var original = session.setActionHandler;
    session.setActionHandler = function (action, handler) {
      var result = original.call(session, action, handler);
      if (typeof handler === 'function') {
        handlers[action] = handler;
      } else {
        delete handlers[action];
      }
      schedule();
      return result;
    };
  }

  function hookProperty(name) {
    var descriptor = findDescriptor(session, name);
    if (!descriptor || typeof descriptor.get !== 'function' || typeof descriptor.set !== 'function') {
      return;
    }
    try {
      Object.defineProperty(session, name, {
        configurable: true,
        enumerable: descriptor.enumerable,
        get: function () { return descriptor.get.call(session); },
        set: function (value) {
          descriptor.set.call(session, value);
          schedule();
        }
      });
    } catch (error) {
      // The one-second check keeps the state current without the hook.
    }
  }

  function findDescriptor(object, name) {
    for (var current = object; current; current = Object.getPrototypeOf(current)) {
      var descriptor = Object.getOwnPropertyDescriptor(current, name);
      if (descriptor) {
        return descriptor;
      }
    }
    return null;
  }

  function schedule() {
    if (timer !== null) {
      return;
    }
    timer = win.setTimeout(function () {
      timer = null;
      flush();
    }, DEBOUNCE_MS);
  }

  function flush() {
    if (inFlight) {
      flushAgain = true;
      return;
    }
    var current = snapshot();
    var serialized = JSON.stringify(current);
    if (serialized === lastSent) {
      return;
    }
    var sending = send(current);
    if (!sending) {
      return;
    }
    inFlight = true;
    sending.then(function () {
      lastSent = serialized;
    }, function () {
      // Retried by the next change or check, never in a loop.
    }).then(function () {
      inFlight = false;
      if (flushAgain) {
        flushAgain = false;
        flush();
      }
    });
  }

  function send(current) {
    var internals = win.__TAURI_INTERNALS__;
    if (!internals || typeof internals.invoke !== 'function') {
      return null;
    }
    try {
      return Promise.resolve(internals.invoke('report_media', { snapshot: current }));
    } catch (error) {
      return null;
    }
  }

  function snapshot() {
    var metadata = null;
    try {
      metadata = session.metadata;
    } catch (error) {
      metadata = null;
    }
    return {
      title: text(metadata && metadata.title),
      artist: text(metadata && metadata.artist),
      album: text(metadata && metadata.album),
      artwork: pickArtwork(metadata && metadata.artwork),
      playbackState: text(session.playbackState) || 'none',
      actions: Object.keys(handlers).sort()
    };
  }

  function text(value) {
    return typeof value === 'string' ? value.slice(0, MAX_TEXT) : '';
  }

  function pickArtwork(images) {
    if (!images || typeof images.length !== 'number') {
      return null;
    }
    var best = null;
    var bestArea = -1;
    for (var i = 0; i < images.length; i++) {
      var image = images[i];
      if (!image || typeof image.src !== 'string' || image.src === '') {
        continue;
      }
      var area = largestArea(image.sizes);
      if (area >= bestArea) {
        best = image.src;
        bestArea = area;
      }
    }
    return best === null ? null : best.slice(0, MAX_TEXT);
  }

  function largestArea(sizes) {
    if (typeof sizes !== 'string') {
      return 0;
    }
    return sizes.split(/\s+/).reduce(function (largest, size) {
      if (size.toLowerCase() === 'any') {
        return Infinity;
      }
      var match = /^(\d+)[xX](\d+)$/.exec(size);
      return match ? Math.max(largest, Number(match[1]) * Number(match[2])) : largest;
    }, 0);
  }

  function dispatch(action) {
    if (action === 'toggle') {
      action = session.playbackState === 'playing' ? 'pause' : 'play';
    }
    if (DISPATCHABLE.indexOf(action) === -1) {
      return false;
    }
    var handler = handlers[action];
    if (typeof handler !== 'function') {
      return false;
    }
    try {
      handler.call(session, { action: action });
      return true;
    } catch (error) {
      return false;
    }
  }
})(window);
