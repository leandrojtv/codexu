const $ = (id) => document.getElementById(id);
const chatLog = $("chatLog"), chatForm=$("chatForm"), chatInput=$("chatInput"), sendBtn=$("sendBtn");
const planList=$("planList"), diffView=$("diffView"), logView=$("logView"), preview=$("preview");
const workspaceLabel=$("workspaceLabel"), statusBar=$("statusBar");
let workspacePath=null, settings=null;

function getTauriInvoke(){return window.__TAURI__?.tauri?.invoke||window.__TAURI__?.invoke||window.__TAURI_INTERNALS__?.invoke||null}
async function invoke(cmd,args={}){const i=getTauriInvoke(); if(!i) throw new Error("Tauri runtime indisponível"); return i(cmd,args)}
const isTauri=()=>Boolean(getTauriInvoke());

function log(level,msg){const line=`[${new Date().toLocaleTimeString()}] ${level}: ${msg}`; if($("logFilter").value==="all"||$("logFilter").value===level){logView.textContent+=`\n${line}`;logView.scrollTop=logView.scrollHeight;} console.log(line)}

function setStatus(extra="-"){statusBar.textContent=`provider: ${settings?.provider||"-"} | modelo: ${settings?.model||"-"} | workspace: ${workspacePath||"-"} | ${extra}`}

function parseMarkdown(text){
  const escaped=text.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;");
  return escaped
    .replace(/```([\s\S]*?)```/g,(_,code)=>`<div class='code'><button class='copy-code'>Copiar</button><pre>${code}</pre></div>`)
    .replace(/^### (.*)$/gm,"<h3>$1</h3>")
    .replace(/^## (.*)$/gm,"<h2>$1</h2>")
    .replace(/^# (.*)$/gm,"<h1>$1</h1>")
    .replace(/\*\*(.*?)\*\*/g,"<strong>$1</strong>")
    .replace(/\n/g,"<br>");
}

function addChat(role,text){
  const el=document.createElement("div"); el.className=`msg ${role}`;
  el.innerHTML=`<b>${role==="user"?"Você":"Assistant"}</b><div>${parseMarkdown(text)}</div>`;
  chatLog.appendChild(el); chatLog.scrollTop=chatLog.scrollHeight;
}

document.addEventListener("click", async (e)=>{
  if(e.target.matches(".copy-code")){const pre=e.target.parentElement.querySelector("pre"); await navigator.clipboard.writeText(pre.textContent||""); log("info","bloco copiado")}
});

function updatePlan(steps=[]){planList.innerHTML=""; steps.forEach(s=>{const li=document.createElement("li"); li.textContent=s; planList.appendChild(li)});}

async function loadSettings(){
  if(!isTauri()) return;
  settings = await invoke("get_settings");
  $("provider").value=settings.provider; $("dockerEndpoint").value=settings.dockerEndpoint; $("cloudEndpoint").value=settings.cloudEndpoint;
  $("cloudApiKey").value=settings.cloudApiKey||""; $("modelName").value=settings.model; $("temperature").value=settings.temperature;
  $("topP").value=settings.topP; $("maxTokens").value=settings.maxTokens; $("timeoutSecs").value=settings.timeoutSecs; $("retries").value=settings.retries;
  setStatus();
}

async function saveSettings(){
  settings={ provider:$("provider").value, dockerEndpoint:$("dockerEndpoint").value, cloudEndpoint:$("cloudEndpoint").value,
    cloudApiKey:$("cloudApiKey").value||null, model:$("modelName").value, temperature:Number($("temperature").value||0.2),
    topP:Number($("topP").value||0.95), maxTokens:Number($("maxTokens").value||512), timeoutSecs:Number($("timeoutSecs").value||180),
    retries:Number($("retries").value||1), theme:document.body.classList.contains("theme-light")?"light":"dark" };
  await invoke("save_settings",{settings}); log("info","settings salvos"); setStatus("settings atualizados");
}

async function restoreWorkspace(){
  if(!isTauri()) return;
  const p=await invoke("get_workspace"); if(p){workspacePath=p; workspaceLabel.textContent=`Workspace: ${p}`; sendBtn.disabled=false}
  const recents=await invoke("list_recent_workspaces"); if(recents?.length) log("info",`recentes: ${recents.slice(0,3).join(" | ")}`);
}

$("workspaceBtn").addEventListener("click", async ()=>{
  try{
    if(isTauri()){const p=await invoke("select_workspace"); if(p){workspacePath=p; workspaceLabel.textContent=`Workspace: ${p}`; sendBtn.disabled=false; setStatus();}}
    else {const manual=prompt("Informe o path do workspace"); if(manual){workspacePath=manual; workspaceLabel.textContent=`Workspace: ${manual}`; sendBtn.disabled=false;}}
  }catch(e){log("error",`workspace: ${e}`)}
});

chatForm.addEventListener("submit", async (e)=>{
  e.preventDefault(); const msg=chatInput.value.trim(); if(!msg) return;
  addChat("user",msg); chatInput.value=""; preview.textContent="gerando..."; const start=performance.now();
  try{
    if(!isTauri()) throw new Error("modo web sem backend");
    const res=await invoke("send_chat_message",{message:msg});
    addChat("assistant",res.assistantMessage||"(sem resposta)"); updatePlan(res.planSteps||[]); preview.textContent=res.assistantMessage||"";
    if(res.diffText) diffView.textContent=res.diffText;
    setStatus(`latência: ${Math.round(performance.now()-start)}ms`);
    log("info","response received");
  }catch(err){addChat("assistant",`Erro: ${err}`); log("error",String(err)); setStatus("erro")}
});

$("applyDiffBtn").addEventListener("click", async ()=>{ try{const msg=await invoke("apply_diff_text",{diffText:diffView.textContent}); log("info",msg)}catch(e){log("error",String(e))} });
$("rejectDiffBtn").addEventListener("click", ()=>{diffView.textContent="(sem diff)"; log("warn","diff rejeitado")});
$("saveSettingsBtn").addEventListener("click", ()=>saveSettings().catch(e=>log("error",String(e))));
$("copyLogsBtn").addEventListener("click", ()=>navigator.clipboard.writeText(logView.textContent));
$("themeBtn").addEventListener("click", ()=>document.body.classList.toggle("theme-light"));
$("logFilter").addEventListener("change", ()=>{});

Array.from(document.querySelectorAll('.tab')).forEach(btn=>btn.addEventListener('click',()=>{
  document.querySelectorAll('.tab').forEach(t=>t.classList.remove('active')); btn.classList.add('active');
  document.querySelectorAll('.tab-content').forEach(c=>c.classList.remove('active')); $(`tab-${btn.dataset.tab}`).classList.add('active');
}));

(async()=>{
  try{ if(isTauri()) { const mode=await invoke("get_app_mode"); $("appTitle").textContent=`Codexu (${mode.milestone})`; }}catch{}
  await loadSettings().catch(()=>{});
  await restoreWorkspace().catch(()=>{});
  if(isTauri()) {
    const setup=await invoke("validate_llama_setup");
    setup.details?.forEach((d)=>log(setup.ok?"info":"warn",d));
    setStatus(`endpoint: ${setup.endpoint}`);
  } else {
    log("warn","modo web/fallback ativo");
  }
})();
