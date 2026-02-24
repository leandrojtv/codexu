const workspaceLabel = document.getElementById("workspaceLabel");
const workspaceBtn = document.getElementById("workspaceBtn");
const chatForm = document.getElementById("chatForm");
const chatInput = document.getElementById("chatInput");
const chatLog = document.getElementById("chatLog");
const planList = document.getElementById("planList");
const diffView = document.getElementById("diffView");
const logView = document.getElementById("logView");
const sendBtn = document.getElementById("sendBtn");
const runtimeBadge = document.getElementById("runtimeBadge");
const appTitle = document.getElementById("appTitle");
const runtimeHelp = document.getElementById("runtimeHelp");

let workspacePath = null;

function appendLog(message) {
  const line = `[${new Date().toLocaleTimeString()}] ${message}`;
  logView.textContent += `\n${line}`;
  logView.scrollTop = logView.scrollHeight;
  console.log(line);
}

function getTauriInvoke() {
  return (
    window.__TAURI__?.tauri?.invoke ||
    window.__TAURI__?.invoke ||
    window.__TAURI_INTERNALS__?.invoke ||
    null
  );
}

function isTauriRuntime() {
  return Boolean(getTauriInvoke());
}

function updateWorkspaceUI(path) {
  workspacePath = path || null;
  workspaceLabel.textContent = workspacePath
    ? `Workspace: ${workspacePath}`
    : "Nenhum workspace selecionado";
  sendBtn.disabled = !workspacePath;
}

function addChatMessage(role, text) {
  const line = document.createElement("p");
  line.className = `msg ${role}`;
  line.textContent = `${role === "user" ? "Você" : "Assistant"}: ${text}`;
  chatLog.appendChild(line);
  chatLog.scrollTop = chatLog.scrollHeight;
}

function updatePlan(steps = []) {
  planList.innerHTML = "";
  for (const step of steps) {
    const li = document.createElement("li");
    li.textContent = step;
    planList.appendChild(li);
  }
}


function setRuntimeBadge(text) {
  runtimeBadge.textContent = text;
}

function setRuntimeHelp(text, isWarning = false) {
  runtimeHelp.textContent = text;
  runtimeHelp.classList.toggle("warning", isWarning);
}

async function detectRuntimeMode() {
  if (!isTauriRuntime()) {
    setRuntimeBadge("runtime: browser fallback");
    setRuntimeHelp(
      "Você está no modo navegador/fallback. Se abriu via cargo run e mesmo assim caiu aqui, faça `cargo clean && cargo run -p codexu_desktop` e confira `withGlobalTauri: true` no tauri.conf.",
      true,
    );
    appendLog("modo navegador detectado: backend Tauri não disponível");
    appendLog(`debug runtime: __TAURI__=${Boolean(window.__TAURI__)}, __TAURI_IPC__=${Boolean(window.__TAURI_IPC__)}`);
    return;
  }

  try {
    const mode = await invoke("get_app_mode");
    const milestone = mode?.milestone || "M?";
    const version = mode?.version || "dev";
    appTitle.textContent = `Codexu (${milestone})`;
    setRuntimeBadge(`runtime: ${mode.runtime} v${version}`);
    setRuntimeHelp(
      "Modo desktop Tauri ativo. Se o seletor não abrir, verifique permissões de Arquivos e Pastas no macOS.",
      false,
    );
    appendLog(`runtime confirmado: ${mode.runtime} (${milestone}) v${version}`);
  } catch (error) {
    setRuntimeBadge("runtime: tauri (erro de handshake)");
    setRuntimeHelp(
      "Não foi possível confirmar modo Tauri. Reinicie com: cargo run -p codexu_desktop",
      true,
    );
    appendLog(`erro ao validar runtime Tauri: ${error}`);
  }
}

async function pickWorkspaceInBrowser() {
  if (window.showDirectoryPicker) {
    try {
      const handle = await window.showDirectoryPicker();
      if (handle?.name) {
        return `(browser) ${handle.name}`;
      }
    } catch (error) {
      if (error?.name === "AbortError") {
        return null;
      }
      appendLog(`showDirectoryPicker falhou: ${error}`);
    }
  }

  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.webkitdirectory = true;
    input.directory = true;

    input.addEventListener("change", () => {
      const first = input.files?.[0];
      if (!first) {
        resolve(null);
        return;
      }

      const rel = first.webkitRelativePath || "";
      const topFolder = rel.split("/")[0] || null;
      resolve(topFolder ? `(browser) ${topFolder}` : "(browser) workspace");
    });

    input.click();
  });
}

async function invoke(cmd, args = {}) {
  const tauriInvoke = getTauriInvoke();
  if (!tauriInvoke) {
    throw new Error("Tauri runtime indisponível");
  }
  return tauriInvoke(cmd, args);
}

async function restoreWorkspaceOnLoad() {
  appendLog("init: restoring workspace");

  const cached = window.localStorage.getItem("codexu.workspacePath");
  if (cached) {
    updateWorkspaceUI(cached);
  }

  if (!isTauriRuntime()) {
    return;
  }

  try {
    const path = await invoke("get_workspace");
    if (path) {
      window.localStorage.setItem("codexu.workspacePath", path);
      updateWorkspaceUI(path);
      appendLog(`workspace restored: ${path}`);
    }
  } catch (error) {
    appendLog(`erro ao restaurar workspace: ${error}`);
  }
}

workspaceBtn.addEventListener("click", async () => {
  appendLog("action: select_workspace");

  if (!isTauriRuntime()) {
    appendLog("modo navegador: usando seletor de pasta web (fallback)");
    let browserPath = await pickWorkspaceInBrowser();

    if (!browserPath) {
      const manual = window.prompt(
        "No navegador não é possível obter caminho absoluto com confiabilidade. Informe manualmente o nome/caminho do workspace:",
        "workspace-local",
      );
      if (manual?.trim()) {
        browserPath = `(browser-manual) ${manual.trim()}`;
      }
    }

    if (!browserPath) {
      appendLog("workspace selection canceled");
      return;
    }

    window.localStorage.setItem("codexu.workspacePath", browserPath);
    updateWorkspaceUI(browserPath);
    appendLog(`workspace selected: ${browserPath}`);
    return;
  }

  try {
    const path = await invoke("select_workspace");
    if (!path) {
      appendLog("workspace selection canceled");
      return;
    }

    window.localStorage.setItem("codexu.workspacePath", path);
    updateWorkspaceUI(path);
    appendLog(`workspace selected: ${path}`);
  } catch (error) {
    appendLog(`erro em select_workspace: ${error}`);
  }
});

chatForm.addEventListener("submit", async (event) => {
  event.preventDefault();

  const text = chatInput.value.trim();
  if (!text) {
    return;
  }

  addChatMessage("user", text);
  chatInput.value = "";

  if (!workspacePath) {
    const warning = "Selecione um workspace antes de enviar mensagens.";
    addChatMessage("assistant", warning);
    appendLog("send blocked: workspace not selected");
    return;
  }

  if (!isTauriRuntime()) {
    addChatMessage(
      "assistant",
      "Rodando no navegador (fallback). Abra pelo Tauri para respostas do backend real.",
    );
    updatePlan([
      "Confirmar workspace selecionado",
      "Executar app via Tauri",
      "Reenviar mensagem",
    ]);
    appendLog("send fallback: tauri runtime indisponível");
    return;
  }

  appendLog(`action: send_chat_message (${text.length} chars)`);

  try {
    const response = await invoke("send_chat_message", { message: text });
    addChatMessage("assistant", response.assistantMessage || "(sem resposta)");
    updatePlan(response.planSteps || []);
    if (response.diffText) {
      diffView.textContent = response.diffText;
    }
    appendLog("response received");
  } catch (error) {
    addChatMessage("assistant", "Falha ao processar mensagem. Verifique o log.");
    appendLog(`erro em send_chat_message: ${error}`);
  }
});

detectRuntimeMode();
restoreWorkspaceOnLoad();
