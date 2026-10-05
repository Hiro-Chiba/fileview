import sys
import subprocess,json,time,pathlib,math
repo=pathlib.Path(__file__).resolve().parents[3]
root=sys.argv[1]
out=repo/'docs/benchmarks/v3-capabilities-2026-10-05'
results=[]
for size in [100,1024]:
 for needle in ['FV_NEAR_START','FV_NEAR_END','NO_MATCH_MARKER']:
  samples=[]
  for i in range(10):
   start=time.monotonic()
   p=subprocess.run(['rg','--no-config','--fixed-strings','--json','--line-number','--',needle,root+f'/log-{size}mib.log'],capture_output=True)
   samples.append((time.monotonic()-start)*1000)
   assert p.returncode in [0,1]
  ordered=sorted(samples)
  results.append(dict(size_mib=size,needle=needle,samples_ms=samples,p50_ms=ordered[4],p95_ms=ordered[9],scope='warm complete process, stdout capture, fixed tiny-output fixtures only'))
  print(size,needle,ordered[9],flush=True)
(out/'rg-repeated.json').write_text(json.dumps(results,indent=2))
