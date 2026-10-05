import sys
import json,pathlib,subprocess,time,platform
repo=pathlib.Path(__file__).resolve().parents[3]
root=sys.argv[1]
out=repo/'docs/benchmarks/v3-capabilities-2026-10-05';out.mkdir(parents=True,exist_ok=True)
(out/'fixture.json').write_text((pathlib.Path(root)/'fixture.json').read_text())
(out/'environment.json').write_text(json.dumps({'platform':platform.platform(),'date_jst':'2026-10-05','cache':'warm, fixtures fully written and fsynced','runs':'sequential release profile','scope':'isolated prototypes, not full TUI'},indent=2))
def run(name,exe,args):
 t=time.monotonic()
 with (out/(name+'.json')).open('w') as stdout,(out/(name+'.time')).open('w') as stderr:
  p=subprocess.run(['/usr/bin/time','-l',str(repo/'target/release/examples'/exe),*map(str,args)],stdout=stdout,stderr=stderr,cwd=repo)
 print(name,p.returncode,round(time.monotonic()-t,2),flush=True)
 if p.returncode: raise SystemExit(name+' failed')
for size in [10,100,1024]:
 n=size*1024*1024
 run('viewport-'+str(size),'viewport_probe',[root+f'/log-{size}mib.log','--offset',n-2048,'--line',(n-2048)//128+1,'--iterations',40,'--render']+(['--whole'] if size==10 else []))
for size in [100,1024]:
 for needle in ['FV_NEAR_START','FV_NEAR_END','NO_MATCH_MARKER']:
  run(f'content-{size}-{needle}','content_search_probe',['--file',root+f'/log-{size}mib.log','--needle',needle,'--iterations',10,'--compare-rg']+(['--whole-read'] if size==100 else []))
 run(f'cancel-{size}','content_search_probe',['--file',root+f'/log-{size}mib.log','--needle','NO_MATCH_MARKER','--iterations',40,'--cancel-ms',5])
for needle in ['FV_NEAR_START','FV_NEAR_END']:
 run('integrated-'+needle,'content_view_probe',[root+'/log-1024mib.log',needle])
for size in [100,1024]:
 run('copy-'+str(size),'copy_probe',['--source',root+f'/log-{size}mib.log','--iterations',5,'--cancel-iterations',30,'--chunk-kib',1024])
