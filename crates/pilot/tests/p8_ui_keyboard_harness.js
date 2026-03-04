const fs = require('fs');
const path = require('path');

// --- Minimal DOM Mock for pilot_ui.js ---
class MockElement {
  constructor(tag, id = '') {
    this.tagName = tag.toUpperCase();
    this.id = id;
    this.className = '';
    this.attributes = {};
    this.style = {};
    this.textContent = '';
    this.innerHTML = '';
    this.children = [];
    this.value = '';
    this._listeners = {};
    this.isFocused = false;
  }
  setAttribute(k, v) { this.attributes[k] = v; }
  getAttribute(k) { return this.attributes[k]; }
  appendChild(el) { this.children.push(el); }
  remove() { }
  insertBefore(el, ref) { this.children.unshift(el); }
  focus() {
    if (global.document.activeElement) {
      global.document.activeElement.isFocused = false;
    }
    this.isFocused = true;
    global.document.activeElement = this;
  }
  click() {
    this.dispatchEvent(new MockEvent('click'));
  }
  addEventListener(event, fn) {
    if(!this._listeners[event]) this._listeners[event] = [];
    this._listeners[event].push(fn);
  }
  dispatchEvent(event) {
    event.target = this;
    if (this._listeners[event.type]) {
      this._listeners[event.type].forEach(fn => fn(event));
    }
  }
  querySelector(selector) { return null; }
  get classList() {
    return {
      toggle: (c, v) => {},
      add: (c) => {},
      remove: (c) => {}
    };
  }
}

class MockEvent {
  constructor(type, props = {}) {
    this.type = type;
    Object.assign(this, props);
    this.defaultPrevented = false;
  }
  preventDefault() { this.defaultPrevented = true; }
}

const mockDoc = {
  elements: {},
  activeElement: null,
  getElementById(id) {
    if (!this.elements[id]) {
        this.elements[id] = new MockElement('div', id);
        this.elements[id].id = id;
    }
    return this.elements[id];
  },
  createElement(tag) { return new MockElement(tag); },
  addEventListener: function(evt, fn) {
    if (evt === 'keydown') {
        this.keydownListener = fn;
    }
  },
  dispatchEvent: function(event) {
    if (event.type === 'keydown' && this.keydownListener) {
        this.keydownListener(event);
    }
  },
  body: new MockElement('body'),
  querySelector: () => null
};

mockDoc.body.appendChild = function() {};

global.document = mockDoc;
global.window = {
  location: { search: '' },
  localStorage: { getItem: () => null, setItem: () => {} }
};
global.navigator = { clipboard: { writeText: () => Promise.resolve() } };
global.fetch = async () => ({ text: async () => '{}', json: async () => ({}) });
global.EventSource = class { addEventListener() {} close() {} };
global.setTimeout = (fn, t) => fn();
global.setInterval = () => {};
global.console = { ...console, error: () => {}, log: () => {} }; // Silence standard logs during test

// --- Load pilot_ui.js ---
const uiCode = fs.readFileSync(path.join(__dirname, '../src/pilot_ui.js'), 'utf8');
try {
  eval(uiCode);
} catch(e) { /* ignore some initialization errors missing full DOM */ }

// Replace console to actually print our test results
global.console = {
  log: process.stdout.write.bind(process.stdout),
  error: process.stderr.write.bind(process.stderr),
  warn: () => {}
};

// --- Test Cases ---
let passed = 0;
let failed = 0;

function assert(condition, msg) {
  if (condition) {
    passed++;
    console.log(`[PASS] ${msg}\n`);
  } else {
    failed++;
    console.error(`[FAIL] ${msg}\n`);
  }
}

console.log('--- Running Custom Node.js DOM Accessibility Harness ---\n');

// 1. Enter/Space triggers chip/button actions for keyboard users
let actionFired = false;
const fakeSpanBtn = new MockElement('span', 'fake-span-btn');
fakeSpanBtn.setAttribute('role', 'button');
fakeSpanBtn.addEventListener('click', () => { actionFired = true; });

// Simulate Enter
const enterEvent = new MockEvent('keydown', { key: 'Enter', target: document.getElementById('body') });
document.dispatchEvent(enterEvent);
assert(!actionFired, 'Enter on normal div/body should ignore click');

// Set target to correct span
enterEvent.target = fakeSpanBtn;
document.activeElement = fakeSpanBtn; // Focus is required or event fires normally but UI only looks at activeElement if used (in UI code we check e.target or activeElement)
document.dispatchEvent(enterEvent);
assert(actionFired, 'Enter key on role="button" span triggers click');

actionFired = false;
const spaceEvent = new MockEvent('keydown', { key: ' ', target: fakeSpanBtn });
document.activeElement = fakeSpanBtn;
document.dispatchEvent(spaceEvent);
assert(actionFired, 'Space key on role="button" span triggers click');
assert(spaceEvent.defaultPrevented, 'Space key default scroll is prevented');

// 2. Error UX applies role="alert" and actionable remediation hints
// call dashVerifyEvidence with empty path
document.getElementById('dash-verify-path').value = '';
dashVerifyEvidence().then(() => {
  const out = document.getElementById('dash-verify-out');
  assert(out.getAttribute('role') === 'alert', 'Empty path error sets role="alert"');
  assert(out.innerHTML.includes('Mitigation:'), 'Error contains actionable Mitigation hint');

  global.fetch = async () => ({
    json: async () => ({ is_valid: false, reason_code: 'missing_file', details: 'not found' })
  });

  document.getElementById('dash-verify-path').value = '/some/path.json';
  return dashVerifyEvidence();
}).then(() => {
  const out = document.getElementById('dash-verify-out');
  assert(out.getAttribute('role') === 'alert', 'Failed verify sets role="alert"');
  assert(out.innerHTML.includes('Mitigation: Ensure the bundle'), 'Error contains explicit mitigation text');
  
  if (failed > 0) process.exit(1);
  else process.exit(0);
}).catch(e => {
  console.error("Test Harness Error: " + e + "\n");
  process.exit(1);
});
