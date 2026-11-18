"""Exercise proposed group retention without changing the host sandbox policy."""

import fcntl
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile

WORKLOAD = r"""
import fcntl, os, sys
child = os.fork()
if child == 0:
    os.setsid()
    with open(sys.argv[1], 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('locked', flush=True)
        sys.stdin.read()
    os._exit(0)
os.waitpid(child, 0)
"""


def pidfd(pid):
    return os.pidfd_open(pid)


def exited(fd):
    # Only a hung-probe guard: readiness means this exact process has exited.
    return bool(select.select([fd], [], [], 5)[0])


with tempfile.TemporaryDirectory(
    prefix="lifecycle-containment-", dir="/scratch/agent"
) as directory:
    for isolated in [True, False]:
        lock = Path(directory) / ("isolated" if isolated else "inherited")
        args = [
            "/usr/bin/bwrap",
            "--die-with-parent",
            "--unshare-user",
            "--ro-bind",
            "/",
            "/",
            "--bind",
            directory,
            directory,
            "--dev",
            "/dev",
            "--cap-drop",
            "ALL",
        ]
        if isolated:
            args += ["--unshare-pid", "--as-pid-1"]
        args += ["--", "/usr/bin/python3", "-c", WORKLOAD, str(lock)]
        # Deliberately omit --new-session: this is the proposed group-retention control.
        process = subprocess.Popen(
            args,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        try:
            assert select.select([process.stdout], [], [], 5)[0], (
                "workload did not start"
            )
            assert process.stdout.readline() == b"locked\n", process.stderr.read()
            init = int(
                Path(f"/proc/{process.pid}/task/{process.pid}/children")
                .read_text()
                .split()[0]
            )
            child = int(
                Path(f"/proc/{init}/task/{init}/children").read_text().split()[0]
            )
            init_fd, child_fd = pidfd(init), pidfd(child)
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=5)
            init_exited, child_exited = exited(init_fd), exited(child_fd)
            with lock.open("r") as handle:
                try:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    held = False
                except BlockingIOError:
                    held = True
            print(
                json.dumps(
                    {
                        "mode": "isolate" if isolated else "inherit",
                        "outer_exit": process.returncode,
                        "supervisor_exited": init_exited,
                        "detached_child_exited": child_exited,
                        "lock_still_held": held,
                    }
                ),
                flush=True,
            )
            if not child_exited:
                signal.pidfd_send_signal(child_fd, signal.SIGKILL)
                assert exited(child_fd), "cleanup failed"
            if not init_exited:
                signal.pidfd_send_signal(init_fd, signal.SIGKILL)
                assert exited(init_fd), "cleanup failed"
            os.close(init_fd)
            os.close(child_fd)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            process.stdin.close()
            process.stdout.close()
            process.stderr.close()
