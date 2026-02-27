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
cargo run -p codexu_desktop
```

No app, abra **Settings** e configure:
- Provider = `docker`
- Docker endpoint = `http://127.0.0.1:11434`
- Model = `codellama:7b-instruct`
