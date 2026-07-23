# Eunice Webapp UI v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reskin the embedded webapp UI (`webapp/index.html`) to the approved "sacristy-gilt" v2 mock — top-tab cockpit shell, gilt light/dark theme, 24-hour cron strip — with zero feature loss, then bump to 1.0.9, deploy to the live `eunice.xeb.ai` systemd service, and cut a full public release.

**Architecture:** The UI is one file embedded via `include_str!("../../webapp/index.html")` in `src/webapp/handlers.rs`. No Rust, routes, or APIs change. Work proceeds as sequential slices of that single file: shell/theme/routing first, then chat, sessions, agents+strip, editor restyle, then version+deploy+release. Each slice is a fully-correct, independently-reviewable change; un-migrated slices may look transitional between tasks.

**Tech Stack:** Vanilla HTML/CSS/JS (no framework), markdown-it via CDN, IBM Plex fonts via Google Fonts CDN, hash-based SPA routing, SSE. Rust/cargo for build; systemd user service + cloudflared for deploy; wrangler for the release CDN.

## Reference files (read before editing)

- **Mock (design source):** `/home/xeb/x/ideas/eunice_ui2.html` (447 lines). CSS = lines 12–245; markup = 250–330; sample JS render/routing = 332–444. Line ranges below cite this file.
- **Target:** `/media/xeb/GreyArea/projects/eunice/webapp/index.html` (4651 lines). `<style>` 11–1738; markdown-it CDN 1740; body/markup 1742–1905; app JS 1907–4649.
- **Spec:** `docs/superpowers/specs/2026-07-22-eunice-webapp-ui-v2-design.md`.

## Global Constraints

- **Only `webapp/index.html` changes** in Tasks 1–5. No Rust, no API, no systemd/tunnel/DNS/Access edits.
- **No feature regression.** Preserve: SSE chat (`sendQuery`/`processSSEData`/`handleEvent`), markdown-it rendering, tool/agent event cards, thinking indicator, `restoreSessionHistory`, live `reconnectToEvents`, session list + delete + new, the full agent editor, the client-side cron engine (`cronParseField`…`nextRuns`…`renderCronPreview`), save/delete/toggle/reload, the dirty-navigation guard, confirm dialog, toasts.
- **The cron engine is logic, not chrome.** Do not alter `parseCron`, `describeCron`, `nextRuns`, or any `cron*` function body. Only `renderCronPreview`'s output markup may be restyled, and only its class names — not its logic.
- **Deploy keeps `127.0.0.1:8812` bind and the Cloudflare Access 302.** No change to `eunice.service`, `~/.cloudflared/config.yml`, DNS, or the Access policy.
- **Version target:** `1.0.9` (from `1.0.8`).
- **Repo:** `/media/xeb/GreyArea/projects/eunice`, branch `eunice-xeb-ai-deploy`. Commit messages end with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- **Verification is build-and-load, not unit tests** (the UI is an embedded string; this repo has no JS test harness). Each task's gate is a local run + manual/`curl` check. `cargo build` must succeed (it re-embeds the file) and must show no new warnings from the HTML (there are none — it's a string).
- **Local scratch run pattern** (used by every task's verification), from the repo root:
  ```bash
  cargo build 2>&1 | tail -5
  ./target/debug/eunice --webapp --no-persist --host 127.0.0.1 --port 8899 \
    --agents /home/xeb/.eunice/webapp/agents.toml &
  SRV=$!; sleep 2
  curl -s -o /dev/null -w 'app:%{http_code}\n' http://127.0.0.1:8899/
  # ... task-specific checks ...
  kill $SRV
  ```
  Open `http://127.0.0.1:8899/` in a browser for the visual checks. `--agents …/agents.toml` makes the Agents tab and editor live locally. Gemma replies require gemmad up (it is, per the running service) — if a live chat reply is needed and port 8899's own gemmad resolution matters, the `--gemmad` flag can be added, but for UI checks the shell renders without it.

---

### Task 1: Shell foundation — theme tokens, top cockpit bar, view routing

Lays the new skeleton: fonts, design tokens, the sticky topbar with 3 tabs + status + theme toggle + New, the `.view` scaffolding for Chat and Sessions, and the router reconciliation across four surfaces (chat / sessions / agents-index / editor). The drawer and old header are removed. Chat still streams; message/card styling is finished in Task 2.

**Files:**
- Modify: `webapp/index.html` — `<head>` (add fonts), `<style>` (prepend tokens + shell CSS), body top (replace drawer + header with topbar + view wrappers), and the JS routing/visibility/status functions.

**Interfaces:**
- Produces (relied on by Tasks 2–5):
  - CSS custom properties from mock 12–53 (`--paper --panel --panel-2 --ink --ink-2 --muted --line --line-2 --nav --nav-ink --nav-border --accent --accent-ink --accent-wash --gilt --btn-bg --btn-fg --btn-bg-hover --ok --warn --err --cool --grid --shadow --mono --sans`), in `:root`, `:root[data-theme="dark"]`, and the `prefers-color-scheme` block.
  - Topbar element IDs kept for status wiring: `#status-dot` (the `.led`), `#status-text`, `#model-badge` (the `.modelchip`), `#themebtn`, `#newbtn`.
  - Tab buttons: `<button class="tab" data-route="chat|sessions|agents">`.
  - View containers: `<section class="view" id="view-chat">` wrapping the existing `#messages` + input; `<section class="view" id="view-sessions">` with `#session-list` inside and a `#session-filter` input. `#agent-index-page` and `#agent-page` remain as-is (restyled in Tasks 4–5).
  - `showView(name)` helper where `name ∈ {'chat','sessions','agents','editor'}`; `setActiveTab(name)`; extended `parseRoute()` returning `view ∈ {'chat','sessions','agentIndex','agent'}`.

- [ ] **Step 1: Add the fonts link to `<head>`**

Immediately after the markdown-it `<script>` tag on line 1740 (or just before `</head>` on 1741), insert (copied from mock 8–10):
```html
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600;700&family=IBM+Plex+Sans:wght@400;500;600&display=swap" rel="stylesheet">
```

- [ ] **Step 2: Prepend the new tokens + shell stylesheet**

Directly after the `<style>` open on line 11, insert a new block, then leave the existing CSS below it (it is progressively overridden/removed by later tasks; new rules that share a selector must come *after* to win, so this block is additive at the top only for tokens — see note). Copy verbatim from the mock:
- Tokens: mock **12–53** (`:root`, `:root[data-theme="dark"]`, `@media (prefers-color-scheme:dark)`).
- Reset + body + grid bg + `button`/`a`/`::selection`: mock **54–66**.
- Topbar + brand + tabs + status + modelchip + iconbtn + btn-primary + focus: mock **68–111**.
- Shell/views/wrap/page-head/search/eyebrow: mock **113–128**.
- Responsive shell + reduced-motion: mock **211–245**.

Then set `body { font-family: var(--sans); background: var(--paper); color: var(--ink); }` — the mock's body rule (56–63) already does this; ensure it is included so the old body background is overridden.

> Note on ordering: the existing stylesheet (old lines 12→) defines legacy selectors like `.container`, `.message`, `.session-item`, `.agent-card`, `.form-input`. Those are re-themed or replaced in Tasks 2–5. In THIS task, only add the tokens + shell rules above; do not yet delete legacy rules. Chat will look transitional until Task 2. That is the accepted gate state.

- [ ] **Step 3: Replace the drawer + header markup with the topbar and view wrappers**

Delete two body regions: the drawer block (lines **1743–1762**: `#session-drawer` through `#drawer-overlay`) and the entire chat `.container` block (lines **1860–1905**: `<div class="container">` through its closing `</div>`, which includes the old `<header>`, `#messages`, `.input-area`, and `.status-bar`). Keep `#agent-index-page` (1764–1774), `#agent-page` (1776–1838), `#confirm-dialog` (1840–1854), and `#toast` (1858) exactly as they are — they stay as body-level siblings between the topbar and the status-bar.

In place of the deleted regions, the body now reads (topbar first, then the kept overlays, then `<main>` + status-bar). Insert the topbar + `<main>` before `#agent-index-page`, and the status-bar after `#toast` (adapting mock 250–330):

```html
<header class="topbar">
  <a class="brand" href="#/" aria-label="eunice home">EUNICE<span class="caret" aria-hidden="true"></span></a>
  <nav class="tabs" aria-label="Primary">
    <button class="tab" data-route="chat">Chat<span class="u"></span></button>
    <button class="tab" data-route="sessions">Sessions<span class="u"></span></button>
    <button class="tab" data-route="agents" id="tab-agents-btn">Agents<span class="u"></span></button>
  </nav>
  <div class="spacer"></div>
  <div class="status"><span class="led" id="status-dot" title="connection"></span><span id="status-text">connecting</span></div>
  <div class="modelchip" id="model-badge">loading…</div>
  <button class="iconbtn" id="themebtn" title="Toggle theme" aria-label="Toggle theme">◐</button>
  <button class="btn-primary" id="newbtn">+ New</button>
</header>

<main>
  <section class="view" id="view-chat">
    <div class="wrap">
      <div class="messages" id="messages"></div>
      <div class="composer">
        <div class="composer-inner">
          <textarea class="query-input" id="query-input" rows="1" placeholder="Message eunice…"></textarea>
          <button class="btn btn-cancel" id="cancel-btn">CANCEL</button>
          <button class="btn-primary" id="send-btn">Send</button>
        </div>
        <div class="hint" id="composer-hint"></div>
      </div>
    </div>
  </section>

  <section class="view" id="view-sessions">
    <div class="wrap">
      <div class="page-head">
        <h1 class="page-title">Sessions</h1>
        <span class="page-count" id="session-count"></span>
        <div class="spacer"></div>
        <input class="search" id="session-filter" placeholder="Filter sessions" aria-label="Filter sessions">
      </div>
      <div class="list" id="session-list"></div>
    </div>
  </section>
</main>

<div class="status-bar" id="status-bar"><span id="version-display">-</span></div>
```

(The `#agent-index-page` and `#agent-page` remain OUTSIDE `<main>`, as body-level siblings, exactly as before. `#messages`, `#query-input`, `#send-btn`, `#cancel-btn`, `#version-display` keep their IDs so existing JS handles resolve. The old `.btn-menu` hamburger is gone.)

- [ ] **Step 4: Rewrite the JS element handles that referenced deleted nodes**

The goal of this step: after it, **every route loads with no `ReferenceError`** — including Sessions and Agents, whose renderers are only *styled* in Tasks 3–4 but must not throw now. This is mechanical but wide; `grep -n` each name below and fix each hit.

- Top-of-script handles (1908–1918): keep `modelBadge`/`statusDot`/`statusText` (IDs kept). Delete `modeDisplay`, `agentDisplay`, `serversDisplay`, `toolsDisplay` and their only uses (in the old `fetchStatus`, replaced wholesale in Step 5).
- Drawer handles: delete every declaration and use of `sessionDrawer`, `drawerOverlay`, `menuBtn`, `drawerNewBtn`, `drawerNewAgentBtn`, `drawerTabs`, `persistenceStatus`, `agentsFooter`, `agentsTabBtn` **except** replace `agentsTabBtn` toggles with the `#tab-agents-btn` `.style.display` shown in Task 4 Step 4 (for Task 1 it is safe to leave that toggle as `document.getElementById('tab-agents-btn').style.display = data.agents_file ? '' : 'none'`).
- Delete `openDrawer`/`closeDrawer` (2666–2680) and their listeners `menuBtn.addEventListener` / `drawerOverlay.addEventListener` (2682–2683).
- Delete the `drawerNewBtn.addEventListener(...)` block (2871–2874).
- In `selectSession`, delete the `closeDrawer();` call (2814).
- In the OLD `loadSessions` (kept until Task 3 restyles it): delete the persistence-badge block (2742–2750, which reads `persistenceStatus.querySelector('.persistence-badge')`) and the two `sessionList.querySelectorAll(...)` handlers still work against `.session-item` markup — leave them. Keep `persistenceEnabled = data.persistent;`. The old `.session-item` rows render **unstyled** on the Sessions tab until Task 3 — that is the accepted transitional state; they must not throw.
- In the OLD `loadAgents` (kept until Task 4): delete only the drawer-node `.classList` toggles (`drawerTabs`, `drawerNewAgentBtn`, `agentsTabBtn` → replace per above), `switchDrawerTab(...)` calls, and any `renderAgentsFooter()` call that touches the deleted `agentsFooter`; keep the data assignments and whichever renderer paints `#agent-index-list`. Agent cards render **unstyled** until Task 4.
- `grep -n 'chatContainer\|switchDrawerTab\|activeDrawerTab\|renderAgentsFooter\|setAgentsActionError\|startAgentsRefresh\|stopAgentsRefresh'` and, for each, either delete (drawer-only) or leave (index-driving) — `startAgentsRefresh`/`stopAgentsRefresh` stay (they drive the index's relative-time refresh). If `switchDrawerTab`/`renderAgentsFooter`/`setAgentsActionError` are now only self-referential drawer code, delete their definitions and calls.

After this step `cargo build` succeeds and, in the browser, all four tabs (Chat/Sessions/Agents/editor via a card) load with **zero console errors** — Sessions and Agents just look unstyled.

- [ ] **Step 5: Rewrite `fetchStatus` to touch only surviving elements**

Replace `fetchStatus` (lines 1991–2031) with:
```javascript
        async function fetchStatus() {
            try {
                const res = await fetch('/api/status');
                const data = await res.json();
                const provider = (data.mode || data.provider || '').toString();
                modelBadge.innerHTML = `${escapeHtml(data.model || '')}${provider ? ` · <b>${escapeHtml(provider.toLowerCase())}</b>` : ''}`;
                statusDot.classList.remove('disconnected');
                statusText.textContent = 'connected';
                document.getElementById('version-display').textContent = `v${data.version}`;
                if (data.agent) currentAgent = data.agent;
                if (data.authenticated_user) {
                    authenticatedUser = data.authenticated_user;
                    statusText.textContent = authenticatedUser;
                    sessionId = null;
                }
            } catch (err) {
                statusDot.classList.add('disconnected');
                statusText.textContent = 'error';
            }
        }
```
Add a small CSS rule for the disconnected LED in the block from Step 2: `.led.disconnected{background:var(--err)}`.

- [ ] **Step 6: Add the theme toggle (persisted)**

After `fetchStatus`, add:
```javascript
        (function initTheme(){
            const saved = localStorage.getItem('eunice_theme');
            if (saved === 'dark' || saved === 'light') document.documentElement.setAttribute('data-theme', saved);
        })();
        document.getElementById('themebtn').addEventListener('click', () => {
            const cur = document.documentElement.getAttribute('data-theme');
            const sysDark = matchMedia('(prefers-color-scheme:dark)').matches;
            const next = cur === 'dark' ? 'light' : (cur === 'light' ? 'dark' : (sysDark ? 'light' : 'dark'));
            document.documentElement.setAttribute('data-theme', next);
            localStorage.setItem('eunice_theme', next);
        });
```

- [ ] **Step 7: Reconcile the router across four surfaces**

Replace `parseRoute` (4335–4356) with:
```javascript
        function parseRoute() {
            const hash = window.location.hash;
            const path = hash.startsWith('#/') ? hash.substring(2) : '';
            const segments = path.split('/').filter(s => s.length > 0);
            if (segments.length === 2 && segments[0] === 'agent') {
                const name = decodeURIComponent(segments[1]);
                return { view: 'agent', name: name === 'new' ? null : name };
            }
            // Agents index tab; accept legacy '#/agent/' too.
            if (segments.length === 1 && (segments[0] === 'agents' || segments[0] === 'agent')) {
                return { view: 'agentIndex' };
            }
            if (segments.length === 1 && segments[0] === 'sessions') {
                return { view: 'sessions' };
            }
            // Chat, optionally naming a session. 'chat' is the bare-chat alias.
            const seg = segments.length === 1 ? decodeURIComponent(segments[0]) : null;
            return { view: 'chat', session: (seg === 'chat') ? null : seg };
        }
```
(Known edge, note it in a comment: a session literally named `chat`, `sessions`, or `agents` is not URL-openable — generated names never collide.)

- [ ] **Step 8: Add `showView` + `setActiveTab` and route through them**

Add near the agent-page lifecycle helpers:
```javascript
        const chatViewEl = document.getElementById('view-chat');
        const sessionsViewEl = document.getElementById('view-sessions');
        function setActiveTab(name){ // name: chat | sessions | agents
            document.querySelectorAll('.tab').forEach(t => t.setAttribute('aria-current', t.dataset.route === name ? 'page' : 'false'));
        }
        // Shows exactly one surface and lights the owning tab. <main> is hidden on the
        // agent routes so its flex:1 does not push the agent pages below the fold.
        function showView(name){
            const inMain = (name === 'chat' || name === 'sessions');
            document.querySelector('main').style.display = inMain ? '' : 'none';
            chatViewEl.classList.toggle('active', name === 'chat');
            sessionsViewEl.classList.toggle('active', name === 'sessions');
            agentIndexPage.classList.toggle('hidden', name !== 'agents');
            agentPage.classList.toggle('hidden', name !== 'editor');
            setActiveTab(name === 'editor' ? 'agents' : name);
        }
```
Update `showAgentPage`/`showAgentIndexPage`/`closeAgentEditor` to route through `showView` instead of poking `chatContainer` directly:
- `showAgentPage()` → body becomes `showView('editor');`
- `showAgentIndexPage()` → `showView('agents');`
- `hideAgentIndexPage()` → keep as `agentIndexPage.classList.add('hidden');` (applyRoute re-shows the right view).
- `closeAgentEditor()`: keep all state resets (lines 3999–4011) but replace the visibility pair `agentPage.classList.add('hidden'); chatContainer.classList.remove('hidden');` with just `agentPage.classList.add('hidden');` — the following `applyRoute` (or caller) calls `showView`. Delete the now-unused `const chatContainer = …` handle (3819) and any other `chatContainer` reference.

- [ ] **Step 9: Extend `applyRoute` for chat + sessions and wire tab clicks**

In `applyRoute` (4489–4522), after the `agent` branch, replace the chat fallthrough (4516–4521) with:
```javascript
            if (route.view === 'sessions') {
                currentRouteView = 'sessions';
                closeAgentEditor();
                showView('sessions');
                loadSessions();
                return;
            }
            const wasElsewhere = currentRouteView !== 'chat';
            currentRouteView = 'chat';
            closeAgentEditor();
            showView('chat');
            if (wasElsewhere) queryInput.focus();
```
And in the `agentIndex`/`agent` branches, the existing `openAgentIndexRoute`/`openAgentRoute` already call `showAgentIndexPage`/`showAgentPage` (now `showView`-based), so tabs light correctly.

Wire the tabs + brand at the end of the script (near line 4578, before `applyRoute(); init();`):
```javascript
        document.querySelectorAll('.tab').forEach(t => t.addEventListener('click', () => {
            const r = t.dataset.route;
            const target = r === 'chat' ? (currentSessionName ? '#/' + currentSessionName : '#/chat')
                         : r === 'sessions' ? '#/sessions' : '#/agents';
            if (window.location.hash === target) applyRoute(); else window.location.hash = target;
        }));
        document.getElementById('session-filter').addEventListener('input', e => renderSessionFilter(e.target.value));
```
(`renderSessionFilter` is defined in Task 3; for Task 1 add a temporary `function renderSessionFilter(){}` stub so the listener binds without error, and remove the stub in Task 3.)

Rewire `#newbtn` (the old `newBtn` handle, 2877): keep its body but drop the `loadSessions()` drawer refresh line if it references a deleted node; ensure it still creates/clears a session and focuses the input. Keep the send/cancel/textarea listeners (1974–1988) unchanged.

- [ ] **Step 10: Build, run, and verify the shell**

Run the scratch-run pattern (port 8899). Verify:
```bash
curl -s -o /dev/null -w 'app:%{http_code}\n' http://127.0.0.1:8899/    # app:200
curl -s http://127.0.0.1:8899/api/status | python3 -c "import sys,json; print('model', json.load(sys.stdin)['model'])"
```
In the browser: topbar renders with gilt band; clicking Chat/Sessions/Agents switches the visible surface and lights the tab; the theme toggle flips light/dark and survives reload; the LED reads "connected" and the model chip shows the real model; no console errors. Chat can still send a message and stream a reply (styling transitional).

- [ ] **Step 11: Commit**

```bash
git add webapp/index.html
git commit -m "UI v2: gilt shell, top-tab nav, router reconciliation

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Chat thread — sacristy message, tool card, and composer styling

Restyles the chat surface to the mock: a `.thread` of `.msg` blocks, `.tool` cards for tool/agent events, an animated `.thinking` row, and the sticky `.composer`. All SSE/markdown logic is preserved; only the markup the builders emit and the CSS change.

**Files:**
- Modify: `webapp/index.html` — chat CSS (add mock chat rules + card rules), the message builders in `handleEvent`/`sendQuery`, and `addMessage`.

**Interfaces:**
- Consumes: `#messages`, `#view-chat`, `.composer` from Task 1; `renderContent`, `escapeHtml`, `escapeAttr` (unchanged).
- Produces: message DOM using classes `.msg`, `.msg.you`, `.msg.eunice`, `.role`, `.body`, `.tool`, `.tool-head`, `.tool-body`, `.thinking`.

- [ ] **Step 1: Add the chat CSS**

Into the Task-1 CSS block (or a new block just below it), copy mock **130–157** (`.thread`, `.msg .role`, `.msg .body`, `.tool*`, `.thinking*`, `.composer*`). Then add rules for the message variants the mock doesn't cover but the app emits, themed with the tokens:
```css
.messages{display:flex; flex-direction:column; gap:22px}
.msg.error .body{color:var(--err)}
.msg.system .body{color:var(--muted); font-family:var(--mono); font-size:12.5px}
.tool.tool-result .tool-body{max-height:320px; overflow:auto}
.truncated-notice{font-family:var(--mono); font-size:11px; color:var(--warn); padding:6px 12px}
.cursor{color:var(--accent); font-weight:700}
.query-input{flex:1; border:0; resize:none; background:none; color:var(--ink); font-family:var(--mono); font-size:14px; line-height:1.5; min-height:24px; outline:none}
.btn-cancel{display:none; font-family:var(--mono); font-size:12px; font-weight:600; color:var(--err); background:none; border:1px solid color-mix(in srgb,var(--err) 45%,transparent); border-radius:8px; padding:8px 12px}
.btn-cancel.visible{display:inline-block}
/* rendered markdown inside a response body */
.msg .body p{margin:0 0 10px} .msg .body p:last-child{margin:0}
.msg .body pre{background:var(--panel-2); border:1px solid var(--line); border-radius:8px; padding:10px 12px; overflow:auto; font-family:var(--mono); font-size:12.5px}
.msg .body code{font-family:var(--mono); font-size:.92em}
.msg .body a{color:var(--accent-ink); text-decoration:underline}
```
Remove the legacy chat CSS from the old stylesheet: search for and delete the old `.message`, `.message.user`, `.message.tool-call`, `.tool-name`, `.tool-args`, `.tool-result-content`, `.agent-invoke`, `.input-area`, `.input-wrapper`, `.query-input`, `.btn`, `.btn-primary` (chat-scoped), `.status-bar`, `.logo`, header/`.header-*` rules — anything now unused. (Keep `.btn-form*`, `.modal*`, `.agent-*`, `.form-*`, `.cron-*`, `.toast`, `.session-*` legacy rules; Tasks 3–5 handle them.)

- [ ] **Step 2: Restyle `addMessage` to emit `.msg` wrappers**

Replace `addMessage` (2033–2041) with a builder that maps the internal `type` to the new markup:
```javascript
        function addMessage(type, content) {
            const el = document.createElement('div');
            if (type === 'user') {
                el.className = 'msg you';
                el.innerHTML = `<div class="role">You</div><div class="body">${content}</div>`;
            } else if (type === 'response') {
                el.className = 'msg eunice';
                el.innerHTML = `<div class="role">Eunice</div><div class="body">${content}</div>`;
            } else if (type === 'thinking') {
                el.className = 'msg eunice';
                el.innerHTML = `<div class="thinking" aria-label="thinking"><i></i><i></i><i></i> ${content}</div>`;
            } else if (type === 'tool-call' || type === 'tool-result' || type === 'agent-invoke' || type === 'agent-result') {
                el.className = 'msg eunice';
                el.innerHTML = content; // content is a full .tool card, built by handleEvent
            } else { // error | system
                el.className = `msg ${type}`;
                el.innerHTML = `<div class="body">${content}</div>`;
            }
            messagesEl.appendChild(el);
            el.scrollIntoView({ behavior: 'instant', block: 'end' });
            return el;
        }
```
(The `thinking` content passed by callers becomes the label after the three dots; update the thinking call sites to pass a plain label — see Step 3. `<span class="cursor">` is no longer needed but harmless if left; prefer the dotted indicator.)

- [ ] **Step 3: Restyle the tool/agent cards in `handleEvent`**

In `handleEvent` (2145–2231), replace each card-emitting `addMessage(...)` payload with `.tool` markup, and simplify the thinking re-adds to a label:
- `tool_call` (2154–2163):
```javascript
                case 'tool_call':
                    removeThinking();
                    addMessage('tool-call', `<div class="tool"><div class="tool-head"><span class="cmd">⌘ ${escapeHtml(event.name)}</span><span class="arg">${escapeHtml(event.arguments)}</span></div></div>`);
                    thinkingEl = addMessage('thinking', 'running');
                    break;
```
- `tool_result` (2165–2176):
```javascript
                case 'tool_result':
                    removeThinking();
                    const lastTool = messagesEl.querySelector('.msg:last-child .tool, .tool:last-of-type');
                    const body = `<div class="tool-body">${escapeHtml(event.result)}</div>${event.truncated ? '<div class="truncated-notice">… output truncated</div>' : ''}`;
                    addMessage('tool-result', `<div class="tool tool-result"><div class="tool-head"><span class="done">done</span></div>${body}</div>`);
                    thinkingEl = addMessage('thinking', 'composing');
                    break;
```
- `agent_invoke` (2178–2188):
```javascript
                case 'agent_invoke':
                    removeThinking();
                    addMessage('agent-invoke', `<div class="tool"><div class="tool-head"><span class="cmd">🔀 invoking</span><span class="arg">${escapeHtml(event.agent_name)}</span></div><div class="tool-body">${escapeHtml(event.task)}</div></div>`);
                    thinkingEl = addMessage('thinking', 'agent running');
                    break;
```
- `agent_result` (2190–2204):
```javascript
                case 'agent_result':
                    removeThinking();
                    addMessage('agent-result', `<div class="tool tool-result"><div class="tool-head"><span class="cmd">✓ ${escapeHtml(event.agent_name)}</span><span class="done">returned</span></div><div class="tool-body">${escapeHtml(event.result)}</div>${event.truncated ? '<div class="truncated-notice">… output truncated</div>' : ''}</div>`);
                    thinkingEl = addMessage('thinking', 'composing');
                    break;
```
- In `sendQuery`, the initial thinking add (2064) becomes `thinkingEl = addMessage('thinking', 'thinking');`. The `response` case (2206–2209) already calls `renderContent`; leave it — it now lands in a `.msg.eunice .body`.

- [ ] **Step 4: Wire the composer hint + cancel button**

Set the hint text once status resolves. In `fetchStatus` success, after setting the model chip, add:
```javascript
                document.getElementById('composer-hint').textContent = `${data.model || 'model'} · ⏎ send · ⇧⏎ newline`;
```
The cancel button now toggles via the `.visible` class (already used by `cancelBtn.classList.add/remove('visible')` in `sendQuery`); the Step-1 CSS makes `.btn-cancel.visible` show. No JS change needed there.

- [ ] **Step 5: Build, run, verify chat**

Scratch-run on 8899. In the browser, on the Chat tab: send "list the files here" (or any prompt). Confirm: your message renders as a `You` block; a `.tool` card appears for the Bash call with the command in the head and output in the mono body; the dotted thinking indicator animates; the final answer renders as an `Eunice` block with markdown. Toggle dark mode — cards and code blocks remain legible. No console errors.

- [ ] **Step 6: Commit**

```bash
git add webapp/index.html
git commit -m "UI v2: sacristy chat thread, tool cards, composer

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Sessions view — full-page list with filter

Promotes the former drawer session list to the routed Sessions view: mock `.list`/`.row` styling, a client-side filter, chats-vs-agent-runs grouping preserved, and open/delete/new wired.

**Files:**
- Modify: `webapp/index.html` — sessions CSS, `renderSessionItem`, `loadSessions`, add `renderSessionFilter`, adjust `selectSession`/`deleteSession`.

**Interfaces:**
- Consumes: `#session-list`, `#session-filter`, `#session-count` (Task 1); `/api/sessions`, `/api/session/delete|new|clear`; `runStatusMark`, `selectSession`, `deleteSession`, `createNewSession` (kept).
- Produces: `.row` rendering; module-level `let allSessions = []` cache for filtering; `renderSessionFilter(term)`.

- [ ] **Step 1: Add the sessions CSS**

Copy mock **159–176** (`.list`, `.row`, `.row::before`, `.dot`, `.dot.agent`, `.r-main`, `.r-name`, `.r-tag`, `.r-prev`, `.r-meta*`). Add the run-status mark + section header + empty-state rules:
```css
.session-section{font-family:var(--mono); font-size:10px; letter-spacing:.14em; text-transform:uppercase; color:var(--muted); padding:12px 16px 4px}
.run-status{font-family:var(--mono); margin-right:6px}
.run-status.success{color:var(--ok)} .run-status.failed{color:var(--err)} .run-status.running{color:var(--accent-ink)} .run-status.interrupted{color:var(--warn)}
.no-sessions{padding:18px 16px; color:var(--muted); font-family:var(--mono); font-size:13px}
```
Delete the legacy `.session-item`, `.session-info`, `.session-name`, `.session-meta`, `.session-delete`, `.session-drawer`, `.drawer-*`, `.persistence-badge` rules from the old stylesheet.

- [ ] **Step 2: Rewrite `renderSessionItem` to `.row` markup**

Replace `renderSessionItem` (2711–2733) with:
```javascript
        function renderSessionItem(session) {
            const isAgentRun = !!session.agent_name;
            const primary = isAgentRun ? session.agent_name : session.name;
            const model = session.model ? escapeHtml(session.model) : '';
            const turns = escapeHtml(String(session.turn_count));
            const time = escapeHtml(session.relative_time || '');
            const sub = isAgentRun ? `${escapeHtml(session.name)}` : (escapeHtml(session.preview || '') || '—');
            const activeCls = session.id === sessionId ? ' active' : '';
            return `
                <button class="row${activeCls}" data-id="${escapeAttr(session.id)}" data-name="${escapeAttr(session.name)}">
                    <span class="dot ${isAgentRun ? 'agent' : ''}"></span>
                    <span class="r-main">
                        <span class="r-name">${runStatusMark(session.run_status)}${escapeHtml(primary)}${isAgentRun ? '<span class="r-tag">agent</span>' : ''}</span>
                        <span class="r-prev">${sub}</span>
                    </span>
                    <span class="r-meta">
                        ${model ? `<span class="m-model">${model}</span>` : ''}
                        <span class="m-turns"><b>${turns}</b> turns</span>
                        <span>${time}</span>
                        <span class="row-del" data-del="${escapeAttr(session.id)}" title="Delete" role="button">✕</span>
                    </span>
                </button>`;
        }
```
Add CSS `.row-del{color:var(--muted); padding:0 2px} .row-del:hover{color:var(--err)}`. (`session.preview`/`session.model` may be absent from `/api/sessions`; the `|| ''` guards keep the row valid — the preview line falls back to the session name/em-dash. Do not invent fields.)

- [ ] **Step 3: Cache sessions and rewrite `loadSessions` + add filter**

Add `let allSessions = [];` near the other session state. Replace `loadSessions` (2735–2792) so it caches then renders through a shared painter, and drop all drawer/persistence-badge code:
```javascript
        async function loadSessions() {
            try {
                const res = await fetch('/api/sessions');
                const data = await res.json();
                persistenceEnabled = data.persistent;
                allSessions = Array.isArray(data.sessions) ? data.sessions : [];
                paintSessions(allSessions);
            } catch (err) {
                console.error('[webapp] Failed to load sessions:', err);
                document.getElementById('session-list').innerHTML = '<div class="no-sessions">Failed to load sessions</div>';
            }
        }
        function paintSessions(list) {
            const box = document.getElementById('session-list');
            const countEl = document.getElementById('session-count');
            const chats = list.filter(s => !s.agent_name);
            const runs = list.filter(s => s.agent_name);
            if (countEl) countEl.textContent = `${list.length} total${runs.length ? ` · ${runs.length} agent runs` : ''}`;
            if (list.length === 0) { box.innerHTML = '<div class="no-sessions">No sessions yet</div>'; return; }
            const grouped = chats.length > 0 && runs.length > 0;
            box.innerHTML =
                (grouped ? '<div class="session-section">Chats</div>' : '') +
                chats.map(renderSessionItem).join('') +
                (grouped ? '<div class="session-section">Agent runs</div>' : '') +
                runs.map(renderSessionItem).join('');
            box.querySelectorAll('.row').forEach(row => {
                row.addEventListener('click', (e) => {
                    if (e.target.closest('.row-del')) { e.stopPropagation(); deleteSession(e.target.closest('.row-del').dataset.del); return; }
                    selectSession(row.dataset.id, row.dataset.name);
                });
            });
        }
        function renderSessionFilter(term) {
            const t = (term || '').toLowerCase();
            const filtered = allSessions.filter(s => ((s.name || '') + (s.agent_name || '') + (s.preview || '')).toLowerCase().includes(t));
            paintSessions(filtered);
        }
```
Remove the temporary `renderSessionFilter(){}` stub from Task 1.

- [ ] **Step 4: Point `selectSession` at chat and refresh from the tab**

In `selectSession` (2794–2821): delete the `closeDrawer();` line. After a session is chosen it should land on the chat view — the existing `updateSessionUrl(name)` / `window.location.hash = '#/' + name` already routes to chat, which `applyRoute` now shows. Confirm `deleteSession`'s success path calls `loadSessions()` (2844) — keep it so the list repaints in place.

- [ ] **Step 5: Build, run, verify sessions**

Scratch-run on 8899 (use `--persist`-style by dropping `--no-persist` if you want pre-existing rows, or create a couple of chats first). On the Sessions tab: rows render in mock style with dot, name, preview/subline, model, turns, time; typing in the filter narrows the list live; the count updates; clicking a row opens it in Chat with the URL becoming `#/<name>`; the ✕ deletes after the confirm dialog. No console errors.

- [ ] **Step 6: Commit**

```bash
git add webapp/index.html
git commit -m "UI v2: full-page Sessions view with live filter

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Agents view — cards with the 24-hour cron strip

Restyles the agent index (`#agent-index-page`, routed at `#/agents`) into mock `.card`s, each carrying the signature 24-hour `.strip` with the mark derived from the real schedule via the existing cron engine, plus a live "now" line. Card opens the editor; enable/disable preserved.

**Files:**
- Modify: `webapp/index.html` — agents/card/strip CSS, `renderAgentIndexRow`/`renderAgentIndex` (the index surface's renderers; confirm exact names at 4247/4280), the agents-refresh plumbing that Task 1 partially unwired, and a new `agentStrip(agent)` helper.

**Interfaces:**
- Consumes: `#agent-index-list` (inside `#agent-index-page`), `#agent-index-note`; `/api/agents`; `nextRuns`, `cronZone`, `describeCron`, `relativeFuture`, `relativePast`, `navigateToAgent`, `toggleAgentEnabled`, `agentsServerTimezone`, `agentsServerModel` (all kept).
- Produces: `.card` rendering with `.strip`; `agentStrip(agent)` returning the strip HTML.

- [ ] **Step 1: Add the agents + strip CSS**

Copy mock **178–204** (`.agents`, `.card`, `.card-top`, `.a-name`, `.chip*`, `.a-desc`, `.a-foot*`, and the full `.strip*` block). Add the disabled/needs-attention affordances themed with tokens:
```css
.card.disabled{opacity:.72}
.agent-index-header{display:flex; align-items:center; gap:14px; max-width:960px; margin:0 auto; padding:26px 20px 6px}
.agent-page-title{font-family:var(--mono); font-size:22px; letter-spacing:.18em; text-transform:uppercase}
.agent-index-note,.agent-index-header-spacer{flex:1}
.agent-index-note{max-width:960px; margin:8px auto; padding:0 20px; color:var(--muted); font-family:var(--mono); font-size:12px}
.agents-warning{color:var(--warn); white-space:pre-wrap; margin-top:6px}
#agent-index-list{max-width:960px; margin:0 auto; padding:8px 20px 40px; display:grid; gap:16px}
```
Delete the legacy drawer-scoped `.agent-card*`, `.agent-detail*`, `.agent-prompt`, `.btn-agent`, `.agent-cron` rules that styled the old drawer list (the index page reused some; verify each is replaced before deleting).

- [ ] **Step 2: Add the `agentStrip` helper**

Add near `renderAgentIndexRow`:
```javascript
        // The 24h tick-strip. The mark is the agent's next fire, mapped to its local
        // hour-of-day; disabled or unparseable agents render the strip without a mark.
        function agentStrip(agent) {
            let ticks = '';
            for (let h = 0; h <= 24; h += 6) {
                const left = (h / 24 * 100).toFixed(2);
                ticks += `<span class="q" style="left:${left}%"></span><span class="lbl" style="left:${left}%">${String(h).padStart(2,'0')}</span>`;
            }
            const now = new Date();
            const nowPos = ((now.getHours() + now.getMinutes()/60) / 24 * 100).toFixed(2);
            let mark = '';
            if (agent.enabled) {
                const zone = agentsServerTimezone || Intl.DateTimeFormat().resolvedOptions().timeZone;
                const res = nextRuns(agent.schedule, Date.now(), zone, 1);
                if (res.ok && res.runs && res.runs.length) {
                    const parts = cronWallParts(res.runs[0], cronZone(zone));
                    const pos = ((parts.hour + parts.minute/60) / 24 * 100).toFixed(2);
                    const hhmm = `${String(parts.hour).padStart(2,'0')}:${String(parts.minute).padStart(2,'0')}`;
                    mark = `<span class="mark" style="left:${pos}%"><span class="t">${hhmm}</span></span>`;
                }
            }
            return `<div class="strip">${ticks}<span class="now" style="left:${nowPos}%"></span>${mark}</div>`;
        }
```
(`cronWallParts(instant, zone)` already exists in the cron engine — see 3507 — returning `{year,month,day,hour,minute,...}` in the given zone. Confirm its return shape when wiring; if it exposes `hour`/`minute`, use them as above.)

- [ ] **Step 3: Rewrite `renderAgentIndexRow` to a `.card`**

Read the current `renderAgentIndexRow` (4247–4279) and `renderAgentIndex` (4280–4307) to keep their data fields and click wiring, then replace the row markup with:
```javascript
        function renderAgentIndexRow(agent, serverModel) {
            const model = agent.model ? escapeHtml(agent.model) : `${escapeHtml(serverModel)} (default)`;
            const nextRun = agent.enabled ? (relativeFuture(agent.next_run_at) || '—') : 'disabled';
            const last = relativePast(agent.last_run_at);
            const lastCls = agent.last_error ? 'err' : 'ok';
            const lastTxt = agent.last_error ? `error · ${last || 'recently'}` : (last ? `ok · ${last}` : 'never run');
            const human = agentScheduleDescription(agent.schedule); // existing helper at 4238
            return `
                <div class="card${agent.enabled ? '' : ' disabled'}" data-name="${escapeAttr(agent.name)}">
                    <div class="card-top">
                        <span class="a-name">${escapeHtml(agent.name)}</span>
                        <span class="chip ${agent.enabled ? 'on' : 'off'}">${agent.enabled ? 'enabled' : 'disabled'}</span>
                        ${agent.last_error ? '<span class="chip err">check</span>' : ''}
                    </div>
                    <div class="a-desc">${escapeHtml(agent.prompt_preview || '')}</div>
                    ${agentStrip(agent)}
                    <div class="a-foot">
                        <span class="cron">${escapeHtml(agent.schedule)} · ${escapeHtml(human)}</span>
                        <span class="next">next run <b>${escapeHtml(nextRun)}</b></span>
                        <span class="last ${lastCls}">last: ${escapeHtml(lastTxt)}</span>
                        <span class="a-model">${model}</span>
                    </div>
                </div>`;
        }
```
Add CSS: `.a-foot .a-model{color:var(--cool)}`. In `renderAgentIndex`, keep the loop and the click handler that opens the editor, but bind the click to the whole `.card` (not a summary sub-row): `list.querySelectorAll('.card').forEach(c => c.addEventListener('click', () => navigateToAgent(c.dataset.name)));`. Preserve the `+ NEW AGENT` button wiring (`#agent-index-new-btn`) if present, routing to `navigateToAgent(null)`.

- [ ] **Step 4: Repair the agents plumbing Task 1 unwired**

`loadAgents` (3143→) still references drawer nodes (`agentsTabBtn`, `drawerTabs`, `drawerNewAgentBtn`) removed in Task 1. Excise those: keep the `data.agents_file`/`editable`/`fingerprint`/`reload_error`/`server_model`/`server_timezone` assignments and the `renderAgents`→ index render, but delete every `.classList` toggle on a deleted node. The Agents tab button (`#tab-agents-btn`) should hide when the server has no agents: replace the old `agentsTabBtn.classList.add('hidden')` with `document.getElementById('tab-agents-btn').style.display = 'none'` in the no-`agents_file` branch, and `= ''` in the has-file branch. The `renderAgents(agents, model)` call that painted the drawer list can be dropped if the index renderer (`renderAgentIndex`) is what the Agents view uses; verify which renderer feeds `#agent-index-list` and keep only that path. Keep `startAgentsRefresh`/`stopAgentsRefresh` (they drive the index's relative-time refresh); ensure they target the index, not the drawer.

- [ ] **Step 5: Build, run, verify agents + strip**

Scratch-run on 8899 with `--agents /home/xeb/.eunice/webapp/agents.toml`. On the Agents tab: each agent renders as a gilt card; the 24h strip shows tick labels 00/06/12/18/24, a faint "now" line at the current time, and — for `daily-health-email` (enabled, `0 7 * * *`) — a gilt mark at 07:00 with the `07:00` tag; a disabled agent's card is dimmed with no mark. Clicking a card opens the editor. The `check` chip appears only when `last_error` is set. Toggle dark mode. No console errors.

- [ ] **Step 6: Commit**

```bash
git add webapp/index.html
git commit -m "UI v2: agent cards with 24h cron strip

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Agent editor, cron preview, and dialogs — gilt re-theme

Re-themes the surviving editor page, its form, the cron preview, the confirm dialog, and the toast to the gilt tokens. No logic changes — the cron engine, dirty-nav guard, save/delete/toggle/reload, and fingerprint conflict check are untouched.

**Files:**
- Modify: `webapp/index.html` — CSS for `.agent-page*`, `.form-*`, `.modal*`, `.btn-form*`, `.cron-*`, `.toast`; nothing in the editor JS except confirming class names still match.

**Interfaces:**
- Consumes: all editor DOM/handles from Task 1's preserved markup (`#agent-page`, `#agent-form-*`, `#confirm-dialog`, `#toast`).
- Produces: themed editor; no new JS symbols.

- [ ] **Step 1: Re-theme the editor page + form**

Replace the legacy `.agent-page*`, `.agent-page-header`, `.agent-page-back`, `.agent-page-form`, `.form-field`, `.form-row`, `.form-label`, `.form-input`, `.form-textarea`, `.form-hint`, `.form-check`, `.form-error`, `.modal-actions*`, `.btn-form*` rules with token-based equivalents:
```css
.agent-page{max-width:860px; margin:0 auto; padding:26px 20px 60px; animation:rise .4s ease both}
.agent-page-header{display:flex; align-items:center; gap:14px; margin-bottom:18px}
.agent-page-back{font-family:var(--mono); font-size:12px; letter-spacing:.06em; color:var(--accent-ink); background:none; border:1px solid var(--line); border-radius:8px; padding:7px 12px}
.agent-page-back:hover{border-color:var(--accent)}
.agent-page-form{display:flex; flex-direction:column; gap:18px}
.form-field{display:flex; flex-direction:column; gap:6px}
.form-row{display:grid; grid-template-columns:1fr 1fr; gap:16px}
.form-label{font-family:var(--mono); font-size:11px; font-weight:600; letter-spacing:.14em; text-transform:uppercase; color:var(--muted)}
.form-input,.form-textarea{font-family:var(--mono); font-size:13.5px; color:var(--ink); background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:9px 12px}
.form-input:focus,.form-textarea:focus{outline:none; border-color:var(--accent); box-shadow:0 0 0 3px var(--accent-wash)}
.form-input[readonly]{color:var(--muted); background:var(--panel-2)}
.form-textarea{min-height:220px; resize:vertical; line-height:1.5}
.form-field-prompt .form-textarea{min-height:300px}
.form-hint{font-family:var(--mono); font-size:11px; color:var(--muted)}
.form-check{display:flex; align-items:center; gap:9px; font-family:var(--mono); font-size:12.5px; color:var(--ink-2)}
.form-error{font-family:var(--mono); font-size:12.5px; color:var(--err); border:1px solid color-mix(in srgb,var(--err) 40%,transparent); border-radius:8px; padding:9px 12px}
.form-error.hidden,.cron-preview.hidden,.hidden{display:none}
.modal-actions{display:flex; align-items:center; gap:10px; margin-top:6px}
.modal-actions-spacer{flex:1}
.btn-form{font-family:var(--mono); font-size:12px; font-weight:600; letter-spacing:.05em; text-transform:uppercase; color:var(--ink); background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:9px 15px}
.btn-form:hover{border-color:var(--ink-2)}
.btn-form-primary{color:var(--btn-fg); background:var(--btn-bg); border-color:var(--btn-bg)}
.btn-form-primary:hover{background:var(--btn-bg-hover)}
.btn-form-danger{color:var(--err); border-color:color-mix(in srgb,var(--err) 45%,transparent)}
```
(Verify the `.hidden { display:none }` rule exists exactly once and wins — many handlers toggle `.hidden`.)

- [ ] **Step 2: Re-theme the cron preview**

Replace the legacy `.cron-preview`, `.cron-line`, `.cron-mark`, `.cron-good`, `.cron-bad`, `.cron-warn`, `.cron-detail` rules:
```css
.cron-preview{font-family:var(--mono); font-size:12px; border:1px solid var(--line); border-radius:8px; padding:8px 12px; background:var(--panel-2); display:flex; flex-direction:column; gap:3px}
.cron-line{display:flex; gap:7px; align-items:baseline}
.cron-mark{font-weight:700}
.cron-good{color:var(--ok)} .cron-good .cron-mark{color:var(--ok)}
.cron-bad{color:var(--err)} .cron-bad .cron-mark{color:var(--err)}
.cron-warn{color:var(--warn)} .cron-warn .cron-mark{color:var(--warn)}
.cron-detail{color:var(--muted)}
```

- [ ] **Step 3: Re-theme the confirm dialog + toast**

Replace `.modal-overlay`, `.modal`, `.modal-confirm`, `.modal-header`, `.modal-title`, `.confirm-message`, `.toast`:
```css
.modal-overlay{position:fixed; inset:0; z-index:70; display:none; align-items:center; justify-content:center; background:rgba(20,17,11,.5); backdrop-filter:blur(2px)}
.modal-overlay.visible{display:flex}
.modal{background:var(--panel); border:1px solid var(--line); border-radius:14px; box-shadow:var(--shadow); width:min(440px,92vw); padding:20px}
.modal-title{font-family:var(--mono); font-size:14px; font-weight:700; letter-spacing:.1em; text-transform:uppercase; color:var(--ink); margin-bottom:10px}
.confirm-message{color:var(--ink-2); font-size:14px; margin-bottom:16px}
.toast{position:fixed; left:50%; bottom:22px; transform:translateX(-50%) translateY(8px); z-index:80; font-family:var(--mono); font-size:12px; color:var(--btn-fg); background:var(--btn-bg); padding:9px 14px; border-radius:8px; box-shadow:var(--shadow); opacity:0; pointer-events:none; transition:opacity .2s, transform .2s}
.toast.visible{opacity:1; transform:translateX(-50%) translateY(0)}
```
Confirm the JS still adds `.visible` to `#confirm-dialog` and `#toast` (search `confirm-dialog` / `showToast`); the mock's `.visible` toggles match. If the confirm dialog uses a different show class, align the CSS to whatever `confirmDialog()` sets.

- [ ] **Step 4: Build, run, verify the editor end-to-end**

Scratch-run on 8899 with `--agents`. Open an agent from the Agents tab. Verify: the form renders in gilt style; typing in SCHEDULE updates the cron preview (green check + "Next run in …"); an invalid expression shows the red line; the both-day-fields warning still appears for e.g. `0 9 1 * 1`; SAVE persists (toast confirms), DISABLE/ENABLE toggles, DELETE arms then removes after confirm, RELOAD works; `← AGENTS` and CANCEL return to the Agents tab; editing then navigating away raises the discard dialog. Toggle dark mode inside the editor. No console errors.

- [ ] **Step 5: Full regression pass + commit**

Do one combined pass on 8899: Chat streams with cards; Sessions filters/opens/deletes; Agents cards + strip; editor cron preview + save; theme toggle persists; mobile widths (resize to ≤760 and ≤400) keep the topbar's second gold row and stack the page heads. Then:
```bash
git add webapp/index.html
git commit -m "UI v2: gilt agent editor, cron preview, dialogs

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Version bump, docs, build, and deploy to eunice.xeb.ai

Bumps to 1.0.9, updates README metrics, runs the test suite, installs the new binary, restarts the live service, and verifies the endpoint — keeping the loopback bind and Access gate intact.

**Files:**
- Modify: `Cargo.toml` (version), `README.md` (LOC + binary size).

- [ ] **Step 1: Run the test suite**

Run: `cd /media/xeb/GreyArea/projects/eunice && cargo test 2>&1 | tail -20`
Expected: all tests pass (328 + any). If a test references the UI string, fix or update it; there should be none — the UI is opaque to Rust tests.

- [ ] **Step 2: Update LOC + binary size in README**

Compute LOC with the CLAUDE.md snippet:
```bash
total=0; for file in src/*.rs src/tools/*.rs src/webapp/*.rs src/tui/*.rs; do test -f "$file" || continue; ts=$(grep -n "^#\[cfg(test)\]" "$file" 2>/dev/null | cut -d: -f1 | head -1); if [ -n "$ts" ]; then lines=$((ts-1)); else lines=$(wc -l < "$file"); fi; total=$((total+lines)); done; echo "Total: $total lines"
cargo build --release 2>&1 | tail -3 && ls -lh target/release/eunice
```
Update the LOC number and the `target/release/eunice` size in `README.md` to the printed values. (The UI is not counted by that snippet — it globs `.rs` only — so LOC may be unchanged; update the binary size regardless.)

- [ ] **Step 3: Bump the version**

Edit `Cargo.toml`: `version = "1.0.8"` → `version = "1.0.9"`. Run `cargo build --release 2>&1 | tail -3` again so `Cargo.lock` and the embedded `GIT_HASH`/version refresh.

- [ ] **Step 4: Commit the release**

```bash
git add Cargo.toml Cargo.lock README.md
git commit -m "Release v1.0.9: webapp UI v2 (sacristy-gilt reskin)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 5: Install the new binary and restart the live service**

```bash
cd /media/xeb/GreyArea/projects/eunice && cargo install --path . --force 2>&1 | tail -3
/home/xeb/.cargo/bin/eunice --version          # must show 1.0.9
systemctl --user restart eunice.service
sleep 4
systemctl --user is-active eunice.service      # active
```

- [ ] **Step 6: Verify the live deploy (security + version)**

```bash
ss -tlnp | grep 8812                                   # 127.0.0.1:8812 ONLY — no 0.0.0.0
curl -s http://127.0.0.1:8812/api/status | python3 -c "import sys,json; print('served', json.load(sys.stdin)['version'])"   # served 1.0.9
curl -sI https://eunice.xeb.ai | head -n 3             # HTTP/2 302 -> kockerbeck.cloudflareaccess.com
```
Expected: loopback-only bind; served version `1.0.9`; edge returns the Access 302. If `ss` shows `0.0.0.0`, STOP and fix `--host`. Ask Mark to load `https://eunice.xeb.ai` in a browser and confirm the v2 UI renders and a gemma reply comes back (the SSE chat is awkward to script).

---

### Task 7: Full public release (longrunningagents.com + push)

Publishes 1.0.9 to the update channel and pushes the branch so `eunice --update` distributes it.

**Files:**
- Modify: `~/p/longrunningagents.com/version.txt`.

- [ ] **Step 1: Push the branch**

```bash
cd /media/xeb/GreyArea/projects/eunice && git push -u origin eunice-xeb-ai-deploy
```
Expected: the branch pushes to `origin`. (This is outward-facing — it publishes the branch; proceed since the user approved the full release.)

- [ ] **Step 2: Update and deploy the version channel**

```bash
printf '1.0.9\n' > /home/xeb/p/longrunningagents.com/version.txt
wrangler pages deploy /home/xeb/p/longrunningagents.com --project-name=longrunningagents --commit-dirty=true --branch=master 2>&1 | tail -8
```
Expected: wrangler reports a successful Pages deployment with a deployment URL. (Follow `~/p/longrunningagents.com/CLAUDE.md` if the deploy command there differs.)

- [ ] **Step 3: Verify the published version and the updater**

```bash
curl -s https://longrunningagents.com/version.txt      # 1.0.9
eunice --update 2>&1 | tail -10
```
Expected: the channel serves `1.0.9`; `eunice --update` reports already-current (the local install is already 1.0.9) or updates cleanly. Report the final `eunice --version`.

---

## Final verification checklist (all must pass)

- [ ] Local scratch run: Chat streams with tool cards; Sessions filters/opens/deletes; Agents cards show the 24h strip with correct marks; editor cron preview + save/delete/toggle/reload work; theme toggle persists; mobile widths hold.
- [ ] `cargo test` green; `Cargo.toml` = `1.0.9`; README LOC + binary size updated.
- [ ] `eunice --version` (installed) = `1.0.9`; `systemctl --user is-active eunice.service` = active.
- [ ] `ss -tlnp | grep 8812` → `127.0.0.1:8812` only.
- [ ] `curl -s 127.0.0.1:8812/api/status` → version `1.0.9`.
- [ ] `curl -sI https://eunice.xeb.ai` → `302` → `kockerbeck.cloudflareaccess.com`; Mark confirms the v2 UI + gemma reply in-browser.
- [ ] Branch pushed; `https://longrunningagents.com/version.txt` = `1.0.9`; `eunice --update` verified.
