#!/usr/bin/env python3
"""Re-render editable Mermaid sources in a temporary browser page, then embed them."""
import json
import re
import secrets
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parent
INDEX = ROOT / "index.html"
DATA = re.compile(r'(<script id="flow-data" type="application/json">)(.*?)(</script>)', re.S)
FLOW_IDS = ("start", "install", "working", "change", "recover", "storage", "product-lifetime")
CONFIG = {
    "startOnLoad": False, "securityLevel": "strict", "theme": "base",
    "deterministicIds": True, "deterministicIDSeed": "product-flow-review",
    "themeVariables": {
        "fontFamily": "system-ui, sans-serif", "fontSize": "17px",
        "primaryColor": "#eaf0f8", "primaryTextColor": "#203047",
        "primaryBorderColor": "#54739b", "lineColor": "#687b93",
        "edgeLabelBackground": "#ffffff",
    },
    "flowchart": {
        "useMaxWidth": True, "htmlLabels": True, "curve": "linear",
        "nodeSpacing": 28, "rankSpacing": 42,
    },
}


def script_json(value):
    # Script elements end at </script>, even inside a JSON string.
    return json.dumps(value, ensure_ascii=False).replace("<", "\\u003c")


def main():
    template = INDEX.read_text()
    if len(DATA.findall(template)) != 1:
        raise SystemExit("Expected exactly one embedded flow-data block in index.html")
    sources = {flow: (ROOT / f"{flow}.mmd").read_text() for flow in FLOW_IDS}
    token = secrets.token_hex(16)
    page = f'''<!doctype html><meta charset="utf-8">
<title>Regenerate product flow viewer</title>
<style>body{{font:17px system-ui,sans-serif;padding:24px}}#render{{position:absolute;left:-10000px}}</style>
<h1>Regenerate product flow viewer</h1><p id="status">Loading pinned Mermaid renderer…</p><div id="render"></div>
<script type="module">
const status=document.getElementById('status');
try{{
const {{default:mermaid}}=await import('https://cdn.jsdelivr.net/npm/mermaid@11.12.0/dist/mermaid.esm.min.mjs');
mermaid.initialize({script_json(CONFIG)});
const sources={script_json(sources)},svgs={{}};
for(const [id,source] of Object.entries(sources)){{
status.textContent=`Rendering ${{id}}…`;
svgs[id]=(await mermaid.render(`flow-${{id}}`,source,document.getElementById('render'))).svg;
}}
const response=await fetch('/{token}',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify(svgs)}});
if(!response.ok)throw new Error(await response.text());
status.textContent='Done. index.html contains all seven diagrams and their Mermaid sources. The temporary server has stopped.';
}}catch(error){{status.textContent=`Regeneration failed: ${{error.message}}. index.html was not replaced. Stop the command and try again.`}}
</script>'''.encode()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def send(self, code, content, content_type="text/plain;charset=utf-8"):
            self.send_response(code)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(content)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(content)

        def do_GET(self):
            if self.path == f"/{token}":
                self.send(200, page, "text/html;charset=utf-8")
            else:
                self.send(404, b"Not found")

        def do_POST(self):
            if self.path != f"/{token}":
                self.send(404, b"Not found")
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 < length <= 2_000_000:
                    raise ValueError("Unexpected render size")
                svgs = json.loads(self.rfile.read(length))
                if set(svgs) != set(FLOW_IDS):
                    raise ValueError("Expected all seven diagrams")
                if any(not isinstance(svg, str) or not svg.startswith("<svg ") for svg in svgs.values()):
                    raise ValueError("Expected rendered SVGs")
                payload = {flow: {"source": sources[flow], "svg": svgs[flow]} for flow in FLOW_IDS}
                output = DATA.sub(lambda match: match[1] + script_json(payload) + match[3], template)
                temporary = INDEX.with_suffix(".html.tmp")
                temporary.write_text(output)
                temporary.replace(INDEX)
            except (ValueError, OSError) as error:
                self.send(400, str(error).encode())
                return
            self.send(200, b"Saved index.html")
            self.server.finished = True

    with HTTPServer(("127.0.0.1", 0), Handler) as server:
        server.finished = False
        print(f"Open http://127.0.0.1:{server.server_port}/{token} in a browser.", flush=True)
        print("Rendering needs internet access only during regeneration. Ctrl-C cancels.", flush=True)
        try:
            while not server.finished:
                server.handle_request()
        except KeyboardInterrupt:
            print("Cancelled; viewer unchanged.")
    if server.finished:
        print("Updated index.html; temporary server stopped.")


if __name__ == "__main__":
    main()
