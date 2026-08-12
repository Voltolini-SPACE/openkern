#!/usr/bin/env python3
"""OpenKern Panel — servidor local do painel operacional.

Stdlib apenas. Bind estrito em 127.0.0.1. O painel é um invólucro do binário
`kern` real: cada operação vira um subprocess tipado (lista de argv, sem shell),
com timeout e ambiente mínimo. Se o binário não existe, o painel diz BLOCKED
em vez de fingir. EVIDENCE > CLAIMS.

Uso: python3 server.py [--port 8150]
Compatível com /usr/bin/python3 (3.9)."""
import json
import os
import shutil
import subprocess
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HOST = "127.0.0.1"
PORT = int(sys.argv[sys.argv.index("--port") + 1]) if "--port" in sys.argv else 8150
HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)

MINIMAL_ENV = {
    "PATH": "/usr/bin:/bin:/usr/local/bin",
    "HOME": os.environ.get("HOME", ""),
}

CONTEXT_OPS = {"index", "stats", "query", "explain"}


def find_kern():
    """Resolve o binário kern: KERN_BIN > release > debug > PATH. Fail-closed."""
    cand = os.environ.get("KERN_BIN")
    if cand and os.path.isfile(cand) and os.access(cand, os.X_OK):
        return cand
    for rel in ("target/release/kern", "target/debug/kern"):
        p = os.path.join(REPO, rel)
        if os.path.isfile(p) and os.access(p, os.X_OK):
            return p
    return shutil.which("kern")


def run_kern(args, timeout=120):
    """Executa o kern com argv tipado. Retorna dict de evidência."""
    kern = find_kern()
    if not kern:
        return {"state": "BLOCKED", "rc": None, "stdout": "",
                "stderr": "binario kern nao encontrado (KERN_BIN, target/, PATH)",
                "ms": 0, "argv": ["kern"] + args}
    t0 = time.time()
    try:
        p = subprocess.run([kern] + args, capture_output=True, text=True,
                           timeout=timeout, env=MINIMAL_ENV, cwd=REPO)
        ms = int((time.time() - t0) * 1000)
        state = "EXECUTED" if p.returncode == 0 else "DENY"
        return {"state": state, "rc": p.returncode, "stdout": p.stdout,
                "stderr": p.stderr, "ms": ms, "argv": ["kern"] + args}
    except subprocess.TimeoutExpired:
        return {"state": "BLOCKED", "rc": None, "stdout": "",
                "stderr": "timeout de %ss excedido" % timeout,
                "ms": int((time.time() - t0) * 1000), "argv": ["kern"] + args}


class Handler(BaseHTTPRequestHandler):
    server_version = "OpenKernPanel/1.0"

    def log_message(self, fmt, *args):  # log honesto e curto no stdout
        sys.stdout.write("%s %s\n" % (self.address_string(), fmt % args))
        sys.stdout.flush()

    def _send(self, code, body, ctype="application/json; charset=utf-8"):
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path in ("/", "/index.html"):
            try:
                with open(os.path.join(HERE, "index.html"), "rb") as f:
                    self._send(200, f.read(), "text/html; charset=utf-8")
            except OSError:
                self._send(500, {"state": "BLOCKED", "stderr": "index.html ausente"})
        elif self.path == "/api/health":
            kern = find_kern()
            ver = run_kern(["version"], timeout=10) if kern else None
            self._send(200, {
                "ok": bool(kern and ver and ver["rc"] == 0),
                "kern": kern or None,
                "version": (ver["stdout"].strip().splitlines() or [""])[0] if ver else None,
                "repo": REPO,
            })
        elif self.path == "/api/sandbox":
            self._send(200, run_kern(["sandbox"], timeout=30))
        else:
            self._send(404, {"state": "DENY", "stderr": "rota nao permitida"})

    def do_POST(self):
        if not self.path.startswith("/api/context/"):
            self._send(404, {"state": "DENY", "stderr": "rota nao permitida"})
            return
        op = self.path.rsplit("/", 1)[-1]
        if op not in CONTEXT_OPS:
            self._send(400, {"state": "DENY", "stderr": "operacao nao permitida: %s" % op})
            return
        try:
            n = int(self.headers.get("Content-Length", "0"))
            body = json.loads(self.rfile.read(n) or b"{}")
        except (ValueError, json.JSONDecodeError):
            self._send(400, {"state": "DENY", "stderr": "JSON invalido"})
            return
        path = os.path.abspath(os.path.expanduser(str(body.get("path", ""))))
        if not os.path.isdir(path):
            self._send(400, {"state": "DENY", "rc": None, "stdout": "", "ms": 0,
                             "argv": [], "stderr": "path nao e diretorio existente: %s" % path})
            return
        args = ["context", op, path]
        text = str(body.get("text", "")).strip()
        if op in ("query", "explain"):
            if not text:
                self._send(400, {"state": "DENY", "rc": None, "stdout": "", "ms": 0,
                                 "argv": [], "stderr": "texto da consulta vazio"})
                return
            args += text.split()
            if op == "query":
                args.append("--json")
        self._send(200, run_kern(args))


def main():
    srv = ThreadingHTTPServer((HOST, PORT), Handler)
    print("OpenKern Panel em http://%s:%s/  (repo: %s)" % (HOST, PORT, REPO))
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
