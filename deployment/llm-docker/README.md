# LLM em Docker para o Codexu (endpoint HTTP)

Este setup sobe um endpoint local compatível com o app usando **Ollama + CodeLlama**.

## 1) Subir o container

```bash
cd deployment/llm-docker
docker compose up -d
```

## 2) Baixar o modelo CodeLlama

```bash
docker compose run --rm ollama-init
```

## 3) Validar endpoint

```bash
curl http://127.0.0.1:11434/api/tags
curl http://127.0.0.1:11434/api/generate \
  -H 'Content-Type: application/json' \
  -d '{"model":"codellama:7b-instruct","prompt":"Responda: ok","stream":false}'
```

## 4) Configurar o app desktop

```bash
export CODEXU_USE_LOCAL_LLM=1
export CODEXU_LLM_BACKEND=endpoint
export CODEXU_LLM_ENDPOINT_URL="http://127.0.0.1:11434"
export CODEXU_LLM_MODEL="codellama:7b-instruct"

cargo run -p codexu_desktop
```

## Observação
- Se abrir o app via GUI (fora do terminal), as variáveis acima podem não ser herdadas.
- Nesse caso, abra pelo terminal ou configure as variáveis no ambiente do launcher.
