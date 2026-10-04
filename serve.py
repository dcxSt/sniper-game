#!/usr/bin/env python3
"""Dev server for www/ — never caches, and tells the page when a new build exists
(the page polls /__build and reloads itself; see main.js).

    ./serve.py [port]      # default 8731
"""
import http.server
import os
import socketserver
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "www")
WATCH = ["index.html", "main.js", "style.css", "pkg/sniper_game.js", "pkg/sniper_game_bg.wasm"]


def stamp() -> str:
    newest = 0
    for f in WATCH:
        try:
            newest = max(newest, os.stat(os.path.join(ROOT, f)).st_mtime_ns)
        except OSError:
            pass
    return str(newest)


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k):
        super().__init__(*a, directory=ROOT, **k)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store, max-age=0")
        super().end_headers()

    def do_GET(self):
        if self.path.split("?")[0] == "/__build":
            body = stamp().encode()
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        super().do_GET()

    def log_message(self, *args):
        pass


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8731
    with Server(("", port), Handler) as s:
        print(f"serving www/ on http://localhost:{port}  (no cache, live reload)")
        s.serve_forever()
