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

async function invoke(cmd, args = {}) {
  const tauriInvoke = window.__TAURI__?.tauri?.invoke;
  if (!tauriInvoke) {
    throw new Error("Tauri invoke indisponível (rodando fora do app Tauri)");
  }
  return tauriInvoke(cmd, args);
}

async function restoreWorkspaceOnLoad() {
  appendLog("init: restoring workspace");

  const cached = window.localStorage.getItem("codexu.workspacePath");
  if (cached) {
    updateWorkspaceUI(cached);
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
