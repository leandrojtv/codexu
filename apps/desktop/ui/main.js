const workspaceLabel = document.getElementById("workspaceLabel");
const workspaceBtn = document.getElementById("workspaceBtn");
const chatForm = document.getElementById("chatForm");
const chatInput = document.getElementById("chatInput");
const chatLog = document.getElementById("chatLog");
const planList = document.getElementById("planList");
const diffView = document.getElementById("diffView");
const logView = document.getElementById("logView");
const sendBtn = document.getElementById("sendBtn");

let workspacePath = null;

function appendLog(message) {
  const line = `[${new Date().toLocaleTimeString()}] ${message}`;
  logView.textContent += `\n${line}`;
  logView.scrollTop = logView.scrollHeight;
  console.log(line);
}

function getTauriInvoke() {
  return window.__TAURI__?.tauri?.invoke || window.__TAURI__?.invoke || null;
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

function pickWorkspaceInBrowser() {
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
    appendLog("modo navegador detectado: restore backend desabilitado");
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
    const browserPath = await pickWorkspaceInBrowser();
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

restoreWorkspaceOnLoad();
