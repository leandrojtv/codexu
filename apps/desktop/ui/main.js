const workspaceLabel = document.getElementById("workspaceLabel");
const workspaceBtn = document.getElementById("workspaceBtn");
const chatForm = document.getElementById("chatForm");
const chatInput = document.getElementById("chatInput");
const chatLog = document.getElementById("chatLog");

async function invoke(cmd, args = {}) {
  if (!window.__TAURI__?.tauri?.invoke) {
    return null;
  }
  return window.__TAURI__.tauri.invoke(cmd, args);
}

workspaceBtn.addEventListener("click", async () => {
  const path = window.prompt("Informe caminho absoluto do workspace:");
  if (!path) return;
  const result = await invoke("set_workspace", { path });
  if (result === null) {
    workspaceLabel.textContent = `Workspace (mock): ${path}`;
    return;
  }
  workspaceLabel.textContent = `Workspace: ${path}`;
});

chatForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const text = chatInput.value.trim();
  if (!text) return;
  const line = document.createElement("p");
  line.textContent = `Você: ${text}`;
  chatLog.appendChild(line);
  chatInput.value = "";
});
