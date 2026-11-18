//! Preserve launcher death observation across LSM transitions that clear PDEATHSIG.

use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;

pub(crate) fn open_launcher_lifetime() -> File {
    // Open our own identity before exec/fork; neither PID reuse nor a parent
    // exiting before the inner stage starts can lose the death notification.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0) };
    if fd < 0 {
        panic!(
            "failed to open sandbox launcher pidfd: {}",
            std::io::Error::last_os_error()
        );
    }
    // SAFETY: pidfd_open returned a new owned descriptor.
    unsafe { File::from_raw_fd(fd as libc::c_int) }
}

pub(crate) fn take_launcher_lifetime(fd: libc::c_int) -> File {
    // Bubblewrap inherited this fd for the trusted inner stage only. Restore
    // CLOEXEC so the workload cannot inherit a handle to the outside launcher.
    if fd <= libc::STDERR_FILENO || unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0
    {
        panic!("invalid sandbox launcher lifetime descriptor");
    }
    // SAFETY: the outer stage transferred this descriptor through bubblewrap.
    unsafe { File::from_raw_fd(fd) }
}

pub(crate) fn wait_for_command(command_pid: libc::pid_t, launcher: Option<File>) -> libc::c_int {
    let mut previous_mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    let notifications = launcher.as_ref().map(|launcher| {
        let mut mask: libc::sigset_t = unsafe { std::mem::zeroed() };
        unsafe {
            libc::sigemptyset(&mut mask);
            libc::sigaddset(&mut mask, libc::SIGCHLD);
            if libc::sigprocmask(libc::SIG_BLOCK, &mask, &mut previous_mask) < 0 {
                panic!("failed to block sandbox child notifications");
            }
        }
        let fd = unsafe { libc::signalfd(-1, &mask, libc::SFD_CLOEXEC) };
        if fd < 0 {
            panic!("failed to open sandbox child notifications");
        }
        // SAFETY: signalfd returned a new owned descriptor.
        (launcher.as_raw_fd(), unsafe { File::from_raw_fd(fd) })
    });

    loop {
        // Drain exits before waiting: SIGCHLD may already have arrived before
        // it was blocked, and multiple exits can share a single notification.
        let mut status = 0;
        let flags = if notifications.is_some() {
            libc::WNOHANG
        } else {
            0
        };
        let reaped_pid = unsafe { libc::waitpid(-1, &mut status, flags) };
        if reaped_pid == command_pid {
            if notifications.is_some() {
                unsafe {
                    libc::sigprocmask(libc::SIG_SETMASK, &previous_mask, std::ptr::null_mut())
                };
            }
            return status;
        }
        if reaped_pid > 0 {
            continue;
        }
        if reaped_pid < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            panic!("failed to reap sandboxed child: {error}");
        }

        if let Some((launcher_fd, child_signal)) = &notifications {
            let mut events = [
                libc::pollfd {
                    fd: *launcher_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: child_signal.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            let result =
                unsafe { libc::poll(events.as_mut_ptr(), events.len() as libc::nfds_t, -1) };
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                panic!("failed to observe sandbox launcher: {error}");
            }
            if events[0].revents != 0 {
                // Bubblewrap's --new-session makes this the sandbox's group.
                // In isolated mode this also kills PID 1, so the kernel tears
                // down every process in the namespace. In inherited mode it
                // terminates the ordinary workload in this sandbox group.
                unsafe {
                    libc::killpg(libc::getpgrp(), libc::SIGKILL);
                    libc::kill(libc::getpid(), libc::SIGKILL);
                    libc::_exit(1);
                }
            }
            if events[1].revents != 0 {
                let mut signal: libc::signalfd_siginfo = unsafe { std::mem::zeroed() };
                // Consume the notification; the next waitpid iteration reaps
                // all exited children, not just the one described here.
                unsafe {
                    libc::read(
                        child_signal.as_raw_fd(),
                        (&mut signal as *mut libc::signalfd_siginfo).cast(),
                        std::mem::size_of_val(&signal),
                    );
                }
            }
        }
    }
}
