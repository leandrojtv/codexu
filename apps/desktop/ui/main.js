const $ = (id) => document.getElementById(id);

const ui = {
  workspacePill: $("workspacePill"),
  providerPill: $("providerPill"),
  modelPill: $("modelPill"),
  latencyPill: $("latencyPill"),
  threadList: $("threadList"),
  threadSearch: $("threadSearch"),
  recentList: $("recentList"),
  fileList: $("fileList"),
  chatFeed: $("chatFeed"),
  emptyState: $("emptyState"),
  suggestions: $("suggestions"),
  chatForm: $("chatForm"),
  chatInput: $("chatInput"),
  sendBtn: $("sendBtn"),
  stopBtn: $("stopBtn"),
  regenBtn: $("regenBtn"),
  planList: $("planList"),
  scratchpad: $("scratchpad"),
  diffMode: $("diffMode"),
  diffFiles: $("diffFiles"),
  diffView: $("diffView"),
  diffActions: $("diffActions"),
  applyAllBtn: $("applyAllBtn"),
  rejectAllBtn: $("rejectAllBtn"),
  logTimeline: $("logTimeline"),
  logLevelFilter: $("logLevelFilter"),
  logSearch: $("logSearch"),
  copyLogsBtn: $("copyLogsBtn"),
  settingsForm: $("settingsForm"),
  saveSettingsBtn: $("saveSettingsBtn"),
  workspaceBtn: $("workspaceBtn"),
  newThreadBtn: $("newThreadBtn"),
  themeBtn: $("themeBtn"),
  toast: $("toast"),
  attachBtn: $("attachBtn"),
  attachInput: $("attachInput"),
};

const state = {
  threadStore: {
    threads: [],
    activeId: null,
    streaming: false,
    lastUserPrompt: "",
  },
  workspaceStore: {
    current: null,
    recents: [],
  },
  providerStore: {
    settings: null,
  },
  diffStore: {
    raw: "",
    files: [],
    activeFileIndex: 0,
  },
  logStore: [],
};

const suggestions = [
  { title: "Create a plan", body: "Descreva um objetivo e eu gero passos executáveis." },
  { title: "Refactor module", body: "Refatoro com foco em legibilidade e manutenção." },
  { title: "Generate tests", body: "Crio casos de teste para cobertura prática." },
  { title: "Improve performance", body: "Proponho otimizações com métricas e trade-offs." },
  { title: "Explain architecture", body: "Mapeio fluxo, riscos e pontos de evolução." },
  { title: "Review changes", body: "Analiso um patch e proponho correções." },
];

const getTauriInvoke = () =>
  window.__TAURI__?.tauri?.invoke || window.__TAURI__?.invoke || window.__TAURI_INTERNALS__?.invoke || null;
const isTauriRuntime = () => Boolean(getTauriInvoke());
const invoke = async (cmd, args = {}) => {
  const i = getTauriInvoke();
  if (!i) throw new Error("Tauri runtime indisponível");
  return i(cmd, args);
};

function toast(message) {
  ui.toast.textContent = message;
  ui.toast.classList.add("show");
  setTimeout(() => ui.toast.classList.remove("show"), 1700);
}

function appendLog(level, event, payload = "") {
  const entry = {
    ts: new Date().toISOString(),
    level,
    event,
    payload,
  };
  state.logStore.push(entry);
  if (state.logStore.length > 800) state.logStore.shift();
  renderLogs();
}

function renderLogs() {
  const levelFilter = ui.logLevelFilter.value;
  const search = ui.logSearch.value.trim().toLowerCase();
  const filtered = state.logStore.filter((log) => {
    if (levelFilter !== "all" && log.level !== levelFilter) return false;
    if (!search) return true;
    return `${log.event} ${log.payload}`.toLowerCase().includes(search);
  });

  ui.logTimeline.innerHTML = filtered
    .map(
      (log) => `
      <div class="log-item">
        <div class="meta">
          <span>${new Date(log.ts).toLocaleTimeString()}</span>
          <span class="chip ${log.level}">${log.level.toUpperCase()}</span>
          <span>${log.event}</span>
        </div>
        <div>${escapeHtml(log.payload || "-")}</div>
      </div>
    `,
    )
    .join("");
}

function setStatus(extra = "-") {
  const s = state.providerStore.settings || {};
  ui.providerPill.textContent = `provider: ${s.provider || "-"}`;
  ui.modelPill.textContent = `model: ${s.model || "-"}`;
  ui.latencyPill.textContent = extra.startsWith("latência") ? extra : `latência: ${extra}`;
  ui.workspacePill.textContent = `Workspace: ${state.workspaceStore.current || "nenhum"}`;
}

function createThread(initialTitle = "New thread") {
  const id = crypto.randomUUID?.() || `t-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const thread = { id, title: initialTitle, messages: [], createdAt: Date.now() };
  state.threadStore.threads.unshift(thread);
  state.threadStore.activeId = id;
  renderThreads();
  renderChat();
  return thread;
}

function activeThread() {
  return state.threadStore.threads.find((t) => t.id === state.threadStore.activeId) || null;
}

function renderThreads() {
  const q = ui.threadSearch.value.trim().toLowerCase();
  const items = state.threadStore.threads.filter((t) =>
    !q ? true : t.title.toLowerCase().includes(q),
  );
  ui.threadList.innerHTML = items
    .map(
      (t) => `<button class="ghost thread-item ${t.id === state.threadStore.activeId ? "active" : ""}" data-thread-id="${t.id}">${escapeHtml(t.title)}</button>`,
    )
    .join("");
}

function renderSuggestions() {
  ui.suggestions.innerHTML = suggestions
    .map(
      (s, idx) => `
      <button class="suggestion-card" data-suggestion-index="${idx}">
        <h4>${escapeHtml(s.title)}</h4>
        <p>${escapeHtml(s.body)}</p>
      </button>
    `,
    )
    .join("");
}

function updateEmptyState() {
  const hasMessages = Boolean(activeThread()?.messages?.length);
  ui.emptyState.style.display = hasMessages ? "none" : "block";
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function parseCodeBlocks(raw) {
  const blocks = [];
  const regex = /```([\w-]*)\n([\s\S]*?)```/g;
  let match;
  let html = escapeHtml(raw);
  let idx = 0;
  while ((match = regex.exec(raw)) !== null) {
    const lang = match[1] || "text";
    const code = match[2] || "";
    const token = `__CODE_BLOCK_${idx}__`;
    blocks.push({ token, lang, code });
    html = html.replace(escapeHtml(match[0]), token);
    idx += 1;
  }

  html = html
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/^### (.+)$/gm, "<h3>$1</h3>")
    .replace(/^## (.+)$/gm, "<h2>$1</h2>")
    .replace(/^# (.+)$/gm, "<h1>$1</h1>")
    .replace(/^- (.+)$/gm, "<li>$1</li>")
    .replace(/\n/g, "<br>");

  for (const block of blocks) {
    const looksLikeDiff = /^(diff --git|@@|\+\+\+|--- )/m.test(block.code);
    const actions = [
      `<button class="ghost tiny" data-code-action="copy" data-code="${encodeURIComponent(block.code)}">Copy</button>`,
      `<button class="ghost tiny" data-code-action="insert" data-code="${encodeURIComponent(block.code)}">Insert</button>`,
      `<button class="ghost tiny" data-code-action="save" data-code="${encodeURIComponent(block.code)}">Save file</button>`,
      looksLikeDiff
        ? `<button class="ghost tiny" data-code-action="review" data-code="${encodeURIComponent(block.code)}">Review changes</button>`
        : "",
    ].join("");

    html = html.replace(
      block.token,
      `<div class="code-block"><div class="code-head"><span>${escapeHtml(block.lang)}</span><div class="code-actions">${actions}</div></div><pre class="code-body"><code>${escapeHtml(block.code)}</code></pre></div>`,
    );
  }

  return html;
}

function renderChat() {
  const thread = activeThread();
  if (!thread) {
    ui.chatFeed.innerHTML = "";
    updateEmptyState();
    return;
  }

  ui.chatFeed.innerHTML = thread.messages
    .map(
      (m) => `
      <article class="msg ${m.role}">
        <div class="who">${m.role === "user" ? "Você" : "Assistant"}</div>
        <div class="content">${parseCodeBlocks(m.text)}</div>
      </article>
    `,
    )
    .join("");

  ui.chatFeed.scrollTop = ui.chatFeed.scrollHeight;
  updateEmptyState();
}

function renderPlan(steps = []) {
  ui.planList.innerHTML = "";
  if (!steps.length) {
    ui.planList.innerHTML = "<li>Aguardando prompt...</li>";
    return;
  }
  for (const step of steps) {
    const li = document.createElement("li");
    li.textContent = step;
    ui.planList.appendChild(li);
  }
}

function parseDiffByFile(rawDiff) {
  const lines = rawDiff.split("\n");
  const files = [];
  let current = null;

  for (const line of lines) {
    if (line.startsWith("diff --git")) {
      if (current) files.push(current);
      const fileName = line.split(" b/")[1] || line;
      current = { file: fileName, diff: `${line}\n`, hunks: [] };
    } else if (current) {
      current.diff += `${line}\n`;
      if (line.startsWith("@@")) current.hunks.push(line);
    }
  }
  if (current) files.push(current);
  return files;
}

function renderDiffPanel() {
  if (!state.diffStore.raw.trim()) {
    ui.diffFiles.innerHTML = "<div class='muted'>Sem arquivos</div>";
    ui.diffView.textContent = "(sem diff)";
    ui.diffActions.innerHTML = "";
    return;
  }

  state.diffStore.files = parseDiffByFile(state.diffStore.raw);
  if (state.diffStore.activeFileIndex >= state.diffStore.files.length) {
    state.diffStore.activeFileIndex = 0;
  }

  ui.diffFiles.innerHTML = state.diffStore.files
    .map(
      (f, idx) => `<button class="ghost diff-file-item ${idx === state.diffStore.activeFileIndex ? "active" : ""}" data-diff-file-index="${idx}">${escapeHtml(f.file)}</button>`,
    )
    .join("");

  const active = state.diffStore.files[state.diffStore.activeFileIndex];
  ui.diffView.textContent = active?.diff || state.diffStore.raw;
  ui.diffActions.innerHTML = active
    ? `<button class="primary tiny" data-diff-action="apply-file">Apply file</button><button class="ghost tiny" data-diff-action="reject-file">Reject file</button>`
    : "";

  if (ui.diffMode.value === "side") {
    ui.diffView.classList.add("side-by-side");
  } else {
    ui.diffView.classList.remove("side-by-side");
  }
}

async function applyDiff(raw) {
  if (!raw.trim()) return;
  try {
    if (!isTauriRuntime()) throw new Error("apply disponível apenas no Tauri");
    const out = await invoke("apply_diff_text", { diffText: raw });
    appendLog("info", "diff.apply", out);
    toast("Diff aplicado");
  } catch (err) {
    appendLog("error", "diff.apply.error", String(err));
    toast("Falha ao aplicar diff");
  }
}

function filterDiffToFile(targetFile) {
  const files = parseDiffByFile(state.diffStore.raw);
  const found = files.find((f) => f.file === targetFile);
  return found?.diff || "";
}

function showContextTab(name) {
  document.querySelectorAll(".context-tab").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.contextTab === name);
  });
  document.querySelectorAll(".context-content").forEach((panel) => {
    panel.classList.toggle("active", panel.id === `tab-${name}`);
  });
}

function loadSettingsToForm(s) {
  $("provider").value = s.provider;
  $("dockerEndpoint").value = s.dockerEndpoint;
  $("cloudEndpoint").value = s.cloudEndpoint;
  $("cloudApiKey").value = s.cloudApiKey || "";
  $("modelName").value = s.model;
  $("temperature").value = s.temperature;
  $("topP").value = s.topP;
  $("maxTokens").value = s.maxTokens;
  $("timeoutSecs").value = s.timeoutSecs;
  $("retries").value = s.retries;
}

function readSettingsFromForm() {
  return {
    provider: $("provider").value,
    dockerEndpoint: $("dockerEndpoint").value.trim(),
    cloudEndpoint: $("cloudEndpoint").value.trim(),
    cloudApiKey: $("cloudApiKey").value.trim() || null,
    model: $("modelName").value.trim(),
    temperature: Number($("temperature").value || 0.2),
    topP: Number($("topP").value || 0.95),
    maxTokens: Number($("maxTokens").value || 512),
    timeoutSecs: Number($("timeoutSecs").value || 180),
    retries: Number($("retries").value || 1),
    theme: document.body.classList.contains("theme-dark") ? "dark" : "light",
  };
}

function ensureActiveThread() {
  if (!activeThread()) {
    createThread("Nova conversa");
  }
}

function buildStreamingText(fullText, onChunk) {
  return new Promise((resolve) => {
    let cursor = 0;
    state.threadStore.streaming = true;
    ui.stopBtn.disabled = false;

    const tick = () => {
      if (!state.threadStore.streaming) {
        resolve();
        return;
      }
      cursor += Math.max(1, Math.floor(fullText.length / 90));
      onChunk(fullText.slice(0, cursor));
      if (cursor >= fullText.length) {
        state.threadStore.streaming = false;
        ui.stopBtn.disabled = true;
        resolve();
        return;
      }
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
}

async function sendPrompt(prompt, asRegenerate = false) {
  ensureActiveThread();
  const thread = activeThread();
  if (!thread) return;

  state.threadStore.lastUserPrompt = prompt;

  if (!asRegenerate) {
    thread.messages.push({ role: "user", text: prompt });
  }
  const assistant = { role: "assistant", text: "" };
  thread.messages.push(assistant);
  renderChat();

  appendLog("info", "chat.send", prompt);
  const started = performance.now();
  ui.sendBtn.disabled = true;
  ui.regenBtn.disabled = true;

  try {
    if (!state.workspaceStore.current) {
      throw new Error("Selecione um workspace antes de enviar mensagens.");
    }

    if (!isTauriRuntime()) {
      throw new Error("Modo web sem backend Tauri: use fallback manual ou execute desktop.");
    }

    const response = await invoke("send_chat_message", { message: prompt });
    const fullAnswer = response.assistantMessage || "(sem resposta)";
    await buildStreamingText(fullAnswer, (partial) => {
      assistant.text = `${partial}${state.threadStore.streaming ? "▍" : ""}`;
      renderChat();
    });
    assistant.text = fullAnswer;
    renderChat();

    renderPlan(response.planSteps || []);
    if (response.diffText) {
      state.diffStore.raw = response.diffText;
      renderDiffPanel();
      appendLog("info", "diff.received", `tamanho=${response.diffText.length}`);
    }

    const latency = `${Math.round(performance.now() - started)}ms`;
    setStatus(`latência: ${latency}`);
    appendLog("info", "chat.received", latency);
  } catch (err) {
    assistant.text = `Erro: ${err}`;
    renderChat();
    appendLog("error", "chat.error", String(err));
  } finally {
    state.threadStore.streaming = false;
    ui.stopBtn.disabled = true;
    ui.sendBtn.disabled = !state.workspaceStore.current;
    ui.regenBtn.disabled = false;
    thread.title = thread.messages.find((m) => m.role === "user")?.text?.slice(0, 42) || thread.title;
    renderThreads();
  }
}

async function loadInitial() {
  renderSuggestions();
  createThread("Boas-vindas");
  renderPlan([]);
  renderDiffPanel();

  if (!isTauriRuntime()) {
    appendLog("warn", "runtime", "modo web/fallback ativo");
    state.providerStore.settings = readSettingsFromForm();
    setStatus("-");
    return;
  }

  try {
    const mode = await invoke("get_app_mode");
    appendLog("info", "runtime", `${mode.runtime} ${mode.version}`);

    const settings = await invoke("get_settings");
    state.providerStore.settings = settings;
    loadSettingsToForm(settings);
    setStatus("-");

    const setup = await invoke("validate_llama_setup");
    appendLog(setup.ok ? "info" : "warn", "setup", `${setup.provider} | ${setup.endpoint}`);
    for (const d of setup.details || []) appendLog(setup.ok ? "info" : "warn", "setup.detail", d);

    const ws = await invoke("get_workspace");
    if (ws) {
      state.workspaceStore.current = ws;
      ui.sendBtn.disabled = false;
      setStatus("ready");
    }

    const recents = await invoke("list_recent_workspaces");
    state.workspaceStore.recents = recents || [];
    renderRecents();
  } catch (err) {
    appendLog("error", "bootstrap", String(err));
  }

  ui.workspacePill.textContent = `Workspace: ${state.workspaceStore.current || "nenhum"}`;
}

function renderRecents() {
  ui.recentList.innerHTML = "";
  if (!state.workspaceStore.recents.length) {
    ui.recentList.innerHTML = "<span>Sem recentes</span>";
    return;
  }

  for (const ws of state.workspaceStore.recents) {
    const btn = document.createElement("button");
    btn.className = "ghost tiny";
    btn.textContent = ws;
    btn.addEventListener("click", () => {
      state.workspaceStore.current = ws;
      ui.sendBtn.disabled = false;
      setStatus("workspace manual");
      toast("Workspace selecionado");
    });
    ui.recentList.appendChild(btn);
  }
}


async function readAttachmentText(file) {
  if (!file) return "";
  const maxBytes = 200 * 1024;
  if (file.size > maxBytes) {
    throw new Error("Arquivo muito grande (limite: 200KB)");
  }
  return file.text();
}

async function appendAttachmentToPrompt(file) {
  const content = await readAttachmentText(file);
  const header = `\n\n[anexo: ${file.name}]\n`;
  ui.chatInput.value = `${ui.chatInput.value}${header}${content}`.trim();
  ui.chatInput.focus();
  appendLog("info", "attachment.loaded", `${file.name} (${file.size} bytes)`);
  toast(`Anexo inserido: ${file.name}`);
}

function bindEvents() {
  ui.threadSearch.addEventListener("input", renderThreads);
  ui.logLevelFilter.addEventListener("change", renderLogs);
  ui.logSearch.addEventListener("input", renderLogs);

  ui.newThreadBtn.addEventListener("click", () => {
    createThread("Nova thread");
    appendLog("info", "thread.new", "created");
  });

  ui.workspaceBtn.addEventListener("click", async () => {
    try {
      if (isTauriRuntime()) {
        const selected = await invoke("select_workspace");
        if (selected) {
          state.workspaceStore.current = selected;
          ui.sendBtn.disabled = false;
          ui.workspacePill.textContent = `Workspace: ${selected}`;
          appendLog("info", "workspace.select", selected);
          const recents = await invoke("list_recent_workspaces");
          state.workspaceStore.recents = recents || [];
          renderRecents();
        }
      } else {
        const manual = window.prompt("No modo web, informe o path do workspace:");
        if (manual?.trim()) {
          state.workspaceStore.current = manual.trim();
          ui.workspacePill.textContent = `Workspace: ${state.workspaceStore.current}`;
          ui.sendBtn.disabled = false;
          appendLog("warn", "workspace.fallback", state.workspaceStore.current);
        }
      }
    } catch (err) {
      appendLog("error", "workspace.error", String(err));
    }
  });

  ui.attachBtn.addEventListener("click", () => {
    ui.attachInput.click();
  });

  ui.attachInput.addEventListener("change", async () => {
    const [file] = ui.attachInput.files || [];
    ui.attachInput.value = "";
    if (!file) return;

    try {
      await appendAttachmentToPrompt(file);
    } catch (err) {
      appendLog("error", "attachment.error", String(err));
      toast("Falha ao ler anexo");
    }
  });

  ui.chatForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const msg = ui.chatInput.value.trim();
    if (!msg) return;
    ui.chatInput.value = "";
    await sendPrompt(msg);
  });

  ui.chatInput.addEventListener("keydown", async (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      const msg = ui.chatInput.value.trim();
      if (!msg) return;
      ui.chatInput.value = "";
      await sendPrompt(msg);
    }
  });

  ui.stopBtn.addEventListener("click", () => {
    state.threadStore.streaming = false;
    ui.stopBtn.disabled = true;
    appendLog("warn", "stream.stop", "usuário interrompeu geração");
  });

  ui.regenBtn.addEventListener("click", async () => {
    const prompt = state.threadStore.lastUserPrompt;
    if (!prompt) return;
    await sendPrompt(prompt, true);
  });

  ui.themeBtn.addEventListener("click", () => {
    document.body.classList.toggle("theme-dark");
    appendLog("info", "theme.toggle", document.body.classList.contains("theme-dark") ? "dark" : "light");
  });

  ui.copyLogsBtn.addEventListener("click", async () => {
    await navigator.clipboard.writeText(
      state.logStore.map((l) => `${l.ts} [${l.level}] ${l.event} ${l.payload || ""}`).join("\n"),
    );
    toast("Logs copiados");
  });

  ui.settingsForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const settings = readSettingsFromForm();
    state.providerStore.settings = settings;
    setStatus("settings atualizados");
    if (isTauriRuntime()) {
      try {
        await invoke("save_settings", { settings });
        appendLog("info", "settings.save", settings.provider);
        toast("Settings salvos");
      } catch (err) {
        appendLog("error", "settings.save.error", String(err));
      }
    } else {
      appendLog("warn", "settings.save", "modo web: persistência não segura");
      toast("Modo web: settings em memória");
    }
  });

  ui.diffMode.addEventListener("change", renderDiffPanel);

  ui.applyAllBtn.addEventListener("click", () => applyDiff(state.diffStore.raw));
  ui.rejectAllBtn.addEventListener("click", () => {
    state.diffStore.raw = "";
    state.diffStore.files = [];
    renderDiffPanel();
    toast("Diff rejeitado");
  });

  document.addEventListener("click", async (event) => {
    const target = event.target;

    if (target.matches(".context-tab") || target.matches("[data-context-tab]")) {
      const tab = target.dataset.contextTab;
      if (tab) showContextTab(tab);
      return;
    }

    if (target.matches(".thread-item")) {
      const id = target.dataset.threadId;
      state.threadStore.activeId = id;
      renderThreads();
      renderChat();
      return;
    }

    if (target.matches(".suggestion-card")) {
      const idx = Number(target.dataset.suggestionIndex);
      const txt = suggestions[idx]?.title || "";
      ui.chatInput.value = txt;
      ui.chatInput.focus();
      return;
    }

    if (target.matches(".diff-file-item")) {
      state.diffStore.activeFileIndex = Number(target.dataset.diffFileIndex || 0);
      renderDiffPanel();
      return;
    }

    if (target.matches("[data-diff-action='apply-file']")) {
      const active = state.diffStore.files[state.diffStore.activeFileIndex];
      if (!active) return;
      await applyDiff(filterDiffToFile(active.file));
      return;
    }

    if (target.matches("[data-diff-action='reject-file']")) {
      const active = state.diffStore.files[state.diffStore.activeFileIndex];
      if (!active) return;
      const files = parseDiffByFile(state.diffStore.raw).filter((f) => f.file !== active.file);
      state.diffStore.raw = files.map((f) => f.diff).join("\n");
      renderDiffPanel();
      toast("Arquivo rejeitado no diff");
      return;
    }

    if (target.matches("[data-code-action]")) {
      const action = target.dataset.codeAction;
      const code = decodeURIComponent(target.dataset.code || "");

      if (action === "copy") {
        await navigator.clipboard.writeText(code);
        toast("Código copiado");
      }

      if (action === "insert") {
        ui.scratchpad.value = `${ui.scratchpad.value}\n${code}`.trim();
        showContextTab("plan");
        toast("Inserido no scratchpad");
      }

      if (action === "save") {
        const relative = window.prompt("Salvar como (path relativo ao workspace)", "scratch/generated.txt");
        if (!relative) return;
        try {
          if (!isTauriRuntime()) throw new Error("salvar arquivo só no Tauri");
          const out = await invoke("save_text_in_workspace", { relativePath: relative, content: code });
          appendLog("info", "workspace.save", out);
          toast("Arquivo salvo");
        } catch (err) {
          appendLog("error", "workspace.save.error", String(err));
          toast("Falha ao salvar arquivo");
        }
      }

      if (action === "review") {
        state.diffStore.raw = code;
        renderDiffPanel();
        showContextTab("diff");
        toast("Diff enviado para review");
      }
    }
  });
}

bindEvents();
loadInitial();
