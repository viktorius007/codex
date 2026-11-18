import os, socket, subprocess
from pathlib import Path

home = Path("/scratch/agent/failure-investigation/shell-home")
home.mkdir(parents=True, exist_ok=True)
(home / ".bashrc").write_text('printf "PROFILE flags=%s\\n" "$-" >&2\n')
env = {"HOME": str(home), "PATH": "/usr/bin:/bin"}
for kind in ["pipe", "socket"]:
    a, b = socket.socketpair()
    p = subprocess.run(
        ["/bin/bash", "-pc", "printf execution"],
        stdin=b if kind == "socket" else subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )
    print(kind, p.returncode, p.stdout, p.stderr)
    a.close()
    b.close()
