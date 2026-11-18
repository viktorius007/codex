import subprocess,os,signal,json,select
from pathlib import Path
p=subprocess.Popen(['/usr/bin/bwrap','--as-pid-1','--new-session','--die-with-parent','--ro-bind','/','/','--dev','/dev','--unshare-user','--unshare-pid','--proc','/proc','--cap-drop','ALL','--','/usr/bin/python3','-c','import ctypes,os; x=ctypes.c_int(); ctypes.CDLL(None).prctl(2,ctypes.byref(x)); print("ready pdeath",x.value,flush=True); input()'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,start_new_session=True)
assert select.select([p.stdout],[],[],5)[0]
print('ready',p.stdout.readline(), 'outer',p.pid,flush=True)
children=Path(f'/proc/{p.pid}/task/{p.pid}/children').read_text().split()
print('children',children,flush=True)
os.killpg(p.pid,signal.SIGKILL);print('outer exit',p.wait(),flush=True)
print('stdout EOF ready',bool(select.select([p.stdout],[],[],2)[0]),flush=True)
for pid in children:
 q=Path('/proc')/pid
 if q.exists():
  print('survivor',pid,(q/'stat').read_text(),flush=True)
  os.kill(int(pid),signal.SIGKILL)
