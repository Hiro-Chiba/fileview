import sys
import subprocess,pathlib,json,time
repo=pathlib.Path(__file__).resolve().parents[3]
root=sys.argv[1]
path=root+'/log-1024mib.log';out=repo/'docs/benchmarks/v3-capabilities-2026-10-05'
base=['rg','--no-config','--line-buffered','--fixed-strings','--json','--line-number','--']
trials=[]
for i in range(30):
 p=subprocess.Popen(base+['NO_MATCH_MARKER',path],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 time.sleep(.005)
 assert p.poll() is None
 start=time.monotonic();p.terminate();p.communicate(timeout=5)
 trials.append((time.monotonic()-start)*1000)
 assert p.returncode<0
handoffs=[]
for needle,expected in [('FV_NEAR_START',17),('FV_NEAR_END',8388593)]:
 p=subprocess.Popen(base+[needle,path],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 hit=None
 for raw in p.stdout:
  record=json.loads(raw)
  if record['type']=='match':hit=record['data'];break
 if p.poll() is None:p.terminate()
 p.communicate(timeout=5)
 assert hit['line_number']==expected
 result=subprocess.run([str(repo/'target/release/examples/viewport_probe'),path,'--offset',str(hit['absolute_offset']),'--line',str(hit['line_number']),'--iterations','1','--render'],capture_output=True,check=True)
 page=json.loads(result.stdout)
 assert needle in page['offscreen_render_rows'][0]
 handoffs.append({'needle':needle,'line':hit['line_number'],'offset':hit['absolute_offset'],'first_row':page['offscreen_render_rows'][0],'bytes_read':page['target_page']['bytes_read']})
(out/'rg-handoff-cancel.json').write_text(json.dumps({'cancel_request_to_reaped_ms':trials,'p95_ms':sorted(trials)[28],'handoffs':handoffs,'scope':'external rg SIGTERM/reap and stable-file offset handoff to isolated offscreen viewport; production lifecycle/stamp integration unimplemented'},indent=2))
print('cancel p95',sorted(trials)[28],flush=True)
