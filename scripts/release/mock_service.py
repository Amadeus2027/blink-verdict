"""Release-only loopback fixture. Never stores images, text or credentials."""
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

state = Path(sys.argv[1])
count = 0

def save(value):
    temporary = state.with_suffix('.tmp')
    temporary.write_text(json.dumps(value), encoding='utf-8')
    temporary.replace(state)



class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        global count
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        content = body['messages'][1]['content']
        assert self.path == '/v1/chat/completions'
        assert 'release-desktop-fixture' in body['messages'][0]['content']
        assert any(x.get('image_url', {}).get('url', '').startswith('data:image/png;base64,') for x in content)
        count += 1
        save({'requests': count, 'image_input': True})
        time.sleep(0.6)  # Allows the harness to verify repeated triggers while busy.
        data = json.dumps({'choices': [{'message': {'content': '{"choice":"option_2"}'}}]}).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *_):
        pass


server = HTTPServer(('127.0.0.1', 0), Handler)
save({'port': server.server_port, 'requests': 0})
server.serve_forever()
