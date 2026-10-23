#!/usr/bin/env bash
# Prepare the demo workspace used by the VHS tapes in this directory.
#
# Binary assets (images, video, PDF, archives) are generated here instead of
# being committed, so the repository stays small. Requires ffmpeg and python3.
set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/sample-project"
DEST="${1:-/tmp/nebula}"

case "$DEST" in
  /tmp/*) ;;
  *) echo "refusing to use $DEST, the demo workspace must live under /tmp" >&2; exit 1 ;;
esac

rm -rf "$DEST"
cp -R "$SRC" "$DEST"
cd "$DEST"

ff() { ffmpeg -hide_banner -loglevel error -y "$@"; }

ff -f lavfi -i "mandelbrot=s=960x540:maxiter=300" -frames:v 1 assets/screenshot.png
ff -f lavfi -i "gradients=s=1200x400:c0=0x1e1b4b:c1=0x7c3aed:c2=0xf472b6:c3=0x22d3ee:n=4:seed=7" -frames:v 1 assets/banner.jpg
ff -f lavfi -i "color=c=0x0f1020:s=512x512" -vf "geq=r='if(lt(hypot(X-256,Y-256),200),110+120*Y/512,15)':g='if(lt(hypot(X-256,Y-256),200),60+40*X/512,16)':b='if(lt(hypot(X-256,Y-256),200),240,32)'" -frames:v 1 assets/logo.png
ff -f lavfi -i "cellauto=s=640x360:rule=110" -frames:v 1 docs/rule110.png
ff -f lavfi -i "testsrc2=s=640x360:d=6:r=30" -f lavfi -i "sine=f=440:d=6" -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest assets/intro.mp4

python3 - <<'EOF'
import os, random, tarfile, zipfile, io

# A small three page PDF written by hand so no PDF library is needed.
pages = [
    ("NEBULA", ["Whitepaper", "v0.4"]),
    ("SCHEDULING", ["Graph", "Queue", "Tick"]),
    ("BENCHMARKS", ["120k jobs/s", "p99 0.8 ms"]),
]
objs = []
def add(body):
    objs.append(body)
    return len(objs)
font = add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
kids = []
pages_id = len(objs) + 1 + 2 * len(pages)
for title, lines in pages:
    ops = ["0.31 0.18 0.62 rg 0 700 612 92 re f", "BT /F1 64 Tf 1 1 1 rg 40 720 Td (%s) Tj ET" % title]
    for i, line in enumerate(lines):
        ops.append("BT /F1 56 Tf 0.1 0.1 0.2 rg 50 %d Td (%s) Tj ET" % (590 - i * 80, line))
    ops.append("0.55 0.36 0.96 rg 50 80 %d 50 re f 50 160 %d 50 re f 50 240 %d 50 re f" % (500, 380, 240))
    stream = "\n".join(ops).encode()
    content = add(b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream")
    kids.append(add(b"<< /Type /Page /Parent %d 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 %d 0 R >> >> /Contents %d 0 R >>" % (pages_id, font, content)))
add(b"<< /Type /Pages /Kids [%s] /Count %d >>" % (b" ".join(b"%d 0 R" % k for k in kids), len(kids)))
catalog = add(b"<< /Type /Catalog /Pages %d 0 R >>" % pages_id)
out = io.BytesIO()
out.write(b"%PDF-1.4\n")
offsets = []
for i, body in enumerate(objs, 1):
    offsets.append(out.tell())
    out.write(b"%d 0 obj\n" % i + body + b"\nendobj\n")
xref = out.tell()
out.write(b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1))
for off in offsets:
    out.write(b"%010d 00000 n \n" % off)
out.write(b"trailer\n<< /Size %d /Root %d 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, catalog, xref))
os.makedirs("docs/design", exist_ok=True)
open("docs/design/whitepaper.pdf", "wb").write(out.getvalue())

with zipfile.ZipFile("data/backup-2026-10.zip", "w", zipfile.ZIP_DEFLATED) as z:
    for root in ["crates", "services", "config"]:
        for dirpath, _, files in os.walk(root):
            for f in files:
                z.write(os.path.join(dirpath, f))
with tarfile.open("data/logs-2026-10.tar.gz", "w:gz") as t:
    for day in range(1, 8):
        data = open("data/app.log", "rb").read()
        info = tarfile.TarInfo("logs/scheduler-2026-10-%02d.log" % day)
        info.size = len(data)
        t.addfile(info, io.BytesIO(data))

random.seed(1)
open("data/model.bin", "wb").write(b"NEBULA\x00\x04" + bytes(random.randrange(256) for _ in range(4096)))
EOF

git init -q
git add -A
git -c user.name=nebula -c user.email=dev@nebula.invalid commit -q -m "Initial import"

# Leave a few changes behind so Git status and diffs show up in the tree.
cat >> crates/core/src/scheduler.rs <<'EOF'

impl Scheduler {
    /// Number of jobs currently holding a worker slot.
    pub fn running(&self) -> usize {
        self.running.len()
    }
}
EOF
sed -i.bak 's/const visible = jobs.filter/const visible = jobs\n    .slice(-500)\n    .filter/' web/src/App.tsx && rm web/src/App.tsx.bak
printf '\n## `GET /metrics`\n\nPrometheus metrics for the gateway.\n' >> docs/api.md
git add docs/api.md
cat > services/gateway/metrics.go <<'EOF'
package main

import "net/http"

func (r *Router) metrics(w http.ResponseWriter, _ *http.Request) {
	w.Write([]byte("nebula_gateway_requests_total " + itoa(r.requests.Load()) + "\n"))
}
EOF
