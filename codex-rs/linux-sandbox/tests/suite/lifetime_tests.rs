//! Launcher death must survive the exec and optional mount-cleanup boundaries.

#![cfg(target_os = "linux")]

use codex_protocol::models::PermissionProfile;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_cargo_bin::cargo_bin;
use pretty_assertions::assert_eq;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::process::Stdio;

#[test_case::test_case("isolate", "direct")]
#[test_case::test_case("no-proc", "direct")]
#[test_case::test_case("inherit", "direct")]
#[test_case::test_case("isolate", "cleanup")]
#[test_case::test_case("no-proc", "cleanup")]
#[test_case::test_case("inherit", "cleanup")]
fn launcher_death_ends_sandbox_after_exec(mode: &str, wrapper: &str) {
    let temp = tempfile::tempdir().expect("fixture directory");
    let lock_path = temp.path().join("workload-lock");
    let lock = File::create(&lock_path).expect("create workload lock");
    let root = AbsolutePathBuf::try_from(temp.path()).expect("absolute fixture path");
    let profile = match wrapper {
        "direct" => PermissionProfile::read_only(),
        "cleanup" => PermissionProfile::workspace_write_with(
            &[root],
            NetworkSandboxPolicy::Restricted,
            /*exclude_tmpdir_env_var*/ true,
            /*exclude_slash_tmp*/ true,
        ),
        _ => panic!("unknown wrapper"),
    };
    let mut command = Command::new(cargo_bin("codex-linux-sandbox").expect("sandbox binary"));
    command
        .args([
            "--sandbox-policy-cwd",
            temp.path().to_str().expect("fixture path"),
        ])
        .arg("--permission-profile")
        .arg(serde_json::to_string(&profile).expect("serialize profile"));
    match mode {
        "isolate" => {}
        "no-proc" => {
            command.arg("--no-proc");
        }
        "inherit" => {
            command.arg("--inherit-pid-namespace");
        }
        _ => panic!("unknown PID mode"),
    }
    let mut child = command
        .args([
            "--", "/usr/bin/python3", "-c",
            "import fcntl,os,sys\nf=open(sys.argv[1]); fcntl.flock(f,fcntl.LOCK_EX)\nif sys.argv[2]!='inherit': os.setsid()\nassert not any(os.readlink('/proc/self/fd/'+fd)=='anon_inode:[pidfd]' for fd in os.listdir('/proc/self/fd') if os.path.exists('/proc/self/fd/'+fd))\nprint('ready',flush=True)\nsys.stdin.read()",
        ])
        .arg(&lock_path)
        .arg(mode)
        .current_dir(temp.path())
        .env("HOME", temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .process_group(/*pgroup*/ 0)
        .spawn()
        .expect("launch sandbox");
    // Child::wait closes its stdin. Keep this writer independently so waiting
    // for the killed launcher cannot make a surviving workload exit on EOF.
    let _stdin = child.stdin.take().expect("sandbox stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("sandbox stdout"));
    let mut ready = [libc::pollfd {
        fd: stdout.get_ref().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    }];
    assert_eq!(
        unsafe { libc::poll(ready.as_mut_ptr(), 1, 5_000) },
        1,
        "workload readiness"
    );
    let mut line = String::new();
    stdout
        .read_line(&mut line)
        .expect("read workload readiness");
    assert_eq!(line, "ready\n");
    assert!(matches!(
        lock.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));

    // The ready workload never forks again. Record the actual chain to prove
    // that both the exec-only and synthetic-mount wrapper paths were exercised.
    let mut pid = child.id();
    let mut descendants = Vec::new();
    loop {
        let children = std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children"))
            .expect("read sandbox children");
        let Some(next) = children.split_whitespace().next() else {
            break;
        };
        pid = next.parse().expect("child PID");
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        assert!(fd >= 0, "open exact descendant identity");
        // SAFETY: pidfd_open returned a new owned descriptor.
        descendants.push(unsafe { File::from_raw_fd(fd as libc::c_int) });
    }
    let observed_depth = descendants.len();
    assert_eq!(
        unsafe { libc::killpg(child.id() as libc::pid_t, libc::SIGKILL) },
        0
    );
    child.wait().expect("reap outer launcher");
    let mut exited = Vec::new();
    for descendant in &descendants {
        let mut event = [libc::pollfd {
            fd: descendant.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }];
        exited.push(unsafe { libc::poll(event.as_mut_ptr(), 1, 5_000) } == 1);
    }
    let released = lock.try_lock().is_ok();
    // Clean up by pinned identities even on a red run; never leave the
    // deliberately detached isolated-mode workload behind after a failure.
    for descendant in &descendants {
        unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                descendant.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            );
        }
    }
    assert_eq!(observed_depth, if wrapper == "cleanup" { 3 } else { 2 });
    assert_eq!((exited, released), (vec![true; observed_depth], true));
}
