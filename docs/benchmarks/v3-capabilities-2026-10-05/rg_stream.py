import sys
import subprocess,json,time,pathlib,statistics,re
repo=pathlib.Path(__file__).resolve().parents[3]
root=sys.argv[1]
out=repo/'docs/benchmarks/v3-capabilities-2026-10-05'
results=[]
for needle in ['FV_NEAR_START','FV_NEAR_END','NO_MATCH_MARKER']:
 trials=[]
 for i in range(10):
  start=time.monotonic()
  child=subprocess.Popen(['/usr/bin/time','-l','rg','--no-config','--line-buffered','--fixed-strings','--json','--line-number','--',needle,root+'/log-1024mib.log'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  first=None;matches=[]
  for raw in child.stdout:
   record=json.loads(raw)
   if record['type']=='match':
    if first is None:first=(time.monotonic()-start)*1000
    data=record['data']; matches.append({'line':data['line_number'],'offset':data['absolute_offset']})
  stderr=child.stderr.read().decode(); status=child.wait()
  assert status in (0,1)
  rss=re.search(r'(\d+)\s+maximum resident set size',stderr)
  trials.append({'first_ms':first,'total_ms':(time.monotonic()-start)*1000,'max_rss_bytes':int(rss.group(1)) if rss else None,'matches':matches})
 results.append({'needle':needle,'trials':trials,'scope':'rg --line-buffered JSON pipe, /usr/bin/time process RSS includes mapped pages; warm 1GiB fixture'})
 print(needle,'first',trials[0]['first_ms'],'rss',trials[0]['max_rss_bytes'],flush=True)
(out/'rg-streamed.json').write_text(json.dumps(results,indent=2))
