# OpenKern Panel

Painel desktop local para operar o `kern`. Stdlib Python apenas; bind estrito
em `127.0.0.1:8150`; cada operação é um subprocess tipado do binário real
(sem shell, ambiente mínimo, timeout). Se o binário não existe: `BLOCKED`,
nunca simulação. EVIDENCE > CLAIMS.

## Rodar

```bash
python3 panel/server.py            # http://127.0.0.1:8150/
```

Requer o binário `kern` (`cargo build -p kern-cli`, ou `KERN_BIN=/caminho/kern`).

## App de Mesa (macOS)

```bash
panel/launcher/build_launcher.sh   # cria "~/Desktop/OpenKern — Panel.app"
```

Padrão DESKTOP_LAUNCHER_STANDARD V1: healthcheck → abrir; senão iniciar
idempotente → esperar → abrir. Ícone `.icns` gerado da geometria congelada E2
(`brand/exports/gen_icns.py`; build exige PIL no `python3` do PATH — o runtime
do servidor usa `/usr/bin/python3` puro). Log:
`~/Library/Logs/PantheonLaunchers/openkern-panel.log`.

## Superfície

| Rota | Ação |
|---|---|
| `GET /api/health` | resolve binário + `kern version` |
| `GET /api/sandbox` | `kern sandbox` (relatório fail-closed) |
| `POST /api/context/{index,stats,query,explain}` | `kern context …` (`query` com `--json`) |

Estados na UI: `EXECUTED` `VERIFIED` `ASK` `BLOCKED` `DENY` `REFUSED` — sempre
glifo + label + cor, nunca cor sozinha.
