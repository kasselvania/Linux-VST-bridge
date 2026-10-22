//! Native Access browser sign-in, end to end on a real Wine runtime (#489).
//!
//! After the user signs in, the browser opens a `native-access:` URL. The
//! bridge's URL handler, `linux-vst-bridge native-access-callback URL`, finds
//! the one running managed Native Access session, checks its files and passes
//! the URL over a same-user socket to that session's supervisor. The
//! supervisor writes it to the Windows launch adapter, which starts a second
//! copy of the app with the URL in the first copy's prefix, Wine server and
//! container. That copy must hand the URL to the first copy and exit.
//!
//! This test runs the chain with the product's own pieces: the manager binary
//! as the handler, the shipped supervisor sources (`session.py`,
//! `ownership.py`) under the user's systemd manager, the launch adapter built
//! from `tools/is2` with its recorded recipe, and a runtime revision acquired
//! by the product's delivery code. Only Native Access is replaced, by the
//! first-party stand-in in `windows-fixtures/browser-return`, which behaves
//! like an Electron single-instance app and records a digest of what it
//! receives, never the URL. Its second copy can deliver, deliver slowly, hang,
//! refuse or crash. It never starts Chromium or opens a visible window, so
//! the runtime's X11 driver changes are not exercised here.
//!
//! Seams, all outside the product's code paths:
//! - the setup is test data (`setup.rs`): the stand-in sits in Native
//!   Access's install location under the names the bundled profile declares,
//!   with the receipts a completed installation leaves;
//! - the session is submitted as the window's Open submits it (`session.rs`),
//!   through the same `vendor_application::session_command`;
//! - the `runtime-revision-fixture` feature lets the test acquire retained
//!   revisions, not only the recommended one, through the same delivery path.
//!
//! Run it only in a disposable account: it writes that account's bridge state,
//! starts user units and runs Wine. The account needs a user systemd manager
//! with a session bus, an X server in that manager's environment (Xvfb is
//! enough), unprivileged user namespaces and host CPython 3.13 or 3.14 at
//! /usr/bin/python3. `.github/workflows/native-access-signin.yml` sets this up
//! on a GitHub runner. Locally, in a throwaway Linux VM user, from the
//! repository root:
//!
//! ```text
//! mkdir -p /var/tmp/lvb-archives
//! x86_64-w64-mingw32-gcc -std=c11 -O2 -municode -mwindows -o /tmp/stand-in.exe \
//!     windows-fixtures/browser-return/single_instance_app.c -lbcrypt -lws2_32 -lshell32
//! python3 -m pip install ziglang==0.14.1
//! python3 -c 'import pathlib,sys,ziglang; sys.path.insert(0,"tools/naui2"); import package; \
//!     package.build_adapter(pathlib.Path(ziglang.__file__).with_name("zig"), "tools/is2/launch.cpp", "/tmp/adapter.exe")'
//! LVB_DISPOSABLE_ACCOUNT=1 LVB_SIGNIN_RUNTIME=managed-ge-proton11-7-slr4-20260805-r7 \
//!     LVB_SIGNIN_ARCHIVES=/var/tmp/lvb-archives LVB_SIGNIN_ADAPTER=/tmp/adapter.exe \
//!     LVB_SIGNIN_STAND_IN=/tmp/stand-in.exe \
//!     cargo test --release --manifest-path bridge-manager/Cargo.toml \
//!     --features runtime-revision-fixture --test native_access_signin -- --nocapture
//! ```
//!
//! Optional: `LVB_SIGNIN_OTHER_USER` names another account for the same-user
//! socket check (run through `sudo -n -u`), and `LVB_SIGNIN_REPORT` receives
//! the timing table as Markdown.
#![cfg(target_os = "linux")]

mod privacy;
mod report;
mod session;
mod setup;

use linux_vst_bridge::hex;
use privacy::Secrets;
use report::Report;
use serde_json::Value;
use session::{listeners, prefix_processes, socket_entries, unit_present, unix_ms, Return, Session};
use setup::{Inputs, Setup};
use sha2::Digest;
use std::{path::{Path, PathBuf}, process::Command, time::Duration};

/// The supervisor's attempt cap, and the adapter's.
const ATTEMPTS: u64 = 4;
/// The handler waits this long for the supervisor's ready word, then gives up.
const HANDLER_READY: Duration = Duration::from_secs(25);
/// The adapter's wait for a second copy (60 s, then 5 s retirement) and the
/// handler's own 70 s wait for the result.
const SECONDARY_TIMEOUT: Duration = Duration::from_secs(60);
const HANDLER_RESULT: Duration = Duration::from_secs(70);
/// The longest a held second copy waits for its release, inside the adapter's 60 s.
const SLOW_SECONDS: u64 = 55;
/// The service ports the bundled profile declares.
const PORTS: [u16; 2] = [5146, 5563];

fn sign_in(code: &str) -> String {
    format!("native-access://auth?code={code}&state={}", linux_vst_bridge::random_id().unwrap())
}

#[test]
fn browser_return_reaches_the_running_app_and_nothing_else() {
    let inputs = Inputs::from_env();
    let mut report = Report::new(&inputs.revision, inputs.report.clone());
    let setup = Setup::create(&inputs, &mut report);
    let mut secrets = Secrets::default();

    // Without a running session there is nobody to deliver to.
    let url = secrets.url(sign_in);
    let refused = setup.browser_return(&url, None);
    assert_refused(&refused, &secrets, "no session is running");
    assert!(setup.stand_in_lines("secondary.log").is_empty());

    // First open of a fresh setup: the supervisor installs and starts the
    // bundled service, then launches the app's first copy.
    let mut session = Session::open(&setup);
    report.stage("first open until a return can be delivered", session.wait_ready());
    let (members, open) = session.core_limits();
    assert!(members > 3 && open.is_empty(), "core dumps allowed in {} of {members} session processes", open.len());

    // 1. The ordinary return: delivered exactly, in time.
    setup.stand_in_mode("deliver");
    let url = secrets.url(sign_in);
    let delivered = deliver(&session, &secrets, "deliver", &url, 1);
    assert!(delivered.elapsed < HANDLER_RESULT, "{:?}", delivered.elapsed);
    let record = session.wait_browser_return(1, 1, &["dispatched"]);
    report_return(&setup, &mut report, "deliver", "dispatched", &delivered);
    assert_eq!((record["authentication"].as_str(), record["secret_retained"].as_bool()),
        (Some("not_observed"), Some(false)), "{record}");

    // 2. Shape: one opaque URI argument, never a second argument, a quote or
    //    another scheme. The handler refuses before anything starts.
    let shapes: [fn(&str) -> String; 3] = [|code| format!("native-access://auth?code={code} --no-sandbox"),
        |code| format!("native-access://auth?code={code}\"--inject"),
        |code| format!("https://example.invalid/?code={code}")];
    for (index, shape) in shapes.into_iter().enumerate() {
        let url = secrets.url(shape);
        let refused = session.browser_return(&url);
        assert_refused(&refused, &secrets, "malformed URI");
        assert!(refused.connected_ms.is_none(), "a malformed URI reached the supervisor");
        if index == 0 { report_return(&setup, &mut report, "URI with an added argument", "refused by the handler", &refused); }
    }
    // The supervisor checks the shape again for any same-user program.
    let url = secrets.url(|code| format!("native-access://auth?code={code} --no-sandbox"));
    assert_eq!(session.raw_return(url.as_bytes()), Some(1), "the supervisor accepted a malformed URI");
    session.wait_browser_return(1, 1, &["invalid_return"]);

    // 3. Same user only: another account's connection is closed unanswered.
    match &inputs.other_user {
        Some(user) => {
            let answer = other_user_attempt(user, &session);
            assert_eq!(answer, "closed", "another account reached the return listener: {answer}");
            session.wait_browser_return(1, 1, &["invalid_return"]);
        }
        None => report.note("Not checked here: another account's connection (LVB_SIGNIN_OTHER_USER unset).".into()),
    }

    // 4. A second copy that exits without handing over: refused, not a timeout.
    setup.stand_in_mode("refuse");
    let url = secrets.url(sign_in);
    let refused = undelivered(&session, &secrets, "second copy refuses", &url, 1);
    assert!(refused.elapsed < Duration::from_secs(30), "{:?}", refused.elapsed);
    session.wait_browser_return(2, 1, &["adapter_refused"]);
    report_return(&setup, &mut report, "second copy refuses", "adapter_refused", &refused);

    // 5. Returns never interleave. While a slow second copy waits, a second
    //    return reaches the listener, is never served, and its handler gives
    //    up after its ready wait. The slow copy is held until then, so slower
    //    handler checks cannot change the outcome; it then still delivers.
    setup.release_slow_copy(false);
    setup.stand_in_mode(&format!("slow {SLOW_SECONDS}"));
    let url = secrets.url(sign_in);
    let competing = secrets.url(sign_in);
    let since = unix_ms();
    let (slow, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| session.browser_return(&url));
        wait_for(|| setup.secondary_events(since).iter().any(|(event, _)| event == "start"), "slow second copy");
        setup.stand_in_mode("deliver");
        let second = session.browser_return(&competing);
        setup.release_slow_copy(true);
        (first.join().unwrap(), second)
    });
    setup.release_slow_copy(false);
    let held_from = setup.secondary_events(since).iter().find(|(event, _)| event == "start").unwrap().1;
    let connected = second.connected_ms.expect("the second return never reached the listener");
    assert!(connected - held_from < (SLOW_SECONDS - HANDLER_READY.as_secs() - 5) * 1000,
        "the second return connected too late to show the wait: {} ms", connected - held_from);
    assert_refused(&second, &secrets, "a return sent while another was being delivered");
    assert!(second.elapsed >= HANDLER_READY, "the second return gave up early: {:?}", second.elapsed);
    assert!(slow.success, "slow return failed: {}", String::from_utf8_lossy(&slow.stderr));
    assert!(slow.elapsed < HANDLER_RESULT, "{:?}", slow.elapsed);
    let handler = slow.handler.as_ref().expect("the waiting handler was observed");
    assert!(handler.core_limit_zero && handler.environment_unreadable, "{handler:?}");
    assert_received(&setup, 2, &url);
    assert!(setup.received()[1].at_ms >= second.exited_ms, "the slow copy delivered while the second return waited");
    report_return(&setup, &mut report, "second copy held while another return waits", "dispatched", &slow);
    report_return(&setup, &mut report, "return sent during that wait", "refused: gave up after 25 s", &second);
    // The abandoned connection is accepted afterwards and may become the last result.
    session.wait_browser_return(3, 2, &["dispatched", "client_closed"]);

    // 6. A second copy that never hands over and never exits: the adapter
    //    ends it after its wait, and only the first copy remains.
    setup.stand_in_mode("hang");
    let url = secrets.url(sign_in);
    let hung = undelivered(&session, &secrets, "second copy hangs", &url, 2);
    assert!(hung.elapsed >= SECONDARY_TIMEOUT && hung.elapsed < HANDLER_RESULT, "{:?}", hung.elapsed);
    session.wait_browser_return(4, 2, &["secondary_timeout"]);
    report_return(&setup, &mut report, "second copy hangs", "secondary_timeout", &hung);
    wait_for(|| session.app_copies() == 1, "the hung second copy to be ended");

    // 7. The attempt cap: a fifth return is turned away before any launch.
    setup.stand_in_mode("deliver");
    let url = secrets.url(sign_in);
    let since = unix_ms();
    let capped = session.browser_return(&url);
    assert_refused(&capped, &secrets, "the fifth return in one session");
    assert!(setup.secondary_events(since).is_empty(), "a return beyond the cap launched a second copy");
    assert_eq!(setup.received().len(), 2);
    session.wait_browser_return(ATTEMPTS, 2, &["secondary_timeout"]);
    report_return(&setup, &mut report, "fifth return", "refused: attempt cap", &capped);

    // While the session still runs, nothing holds a code.
    search_for_codes(&setup, &inputs, &secrets, &mut report, "with the session running");

    // Stop: everything retires, the service included.
    let (result, stopped) = session.stop();
    report.stage("stop until retirement is recorded", stopped);
    assert_retired(&setup, &session, &result);
    assert_eq!(result["browser_return"]["attempts"], ATTEMPTS, "{}", result["browser_return"]);
    let after = setup.browser_return(&secrets.url(sign_in), None);
    assert_refused(&after, &secrets, "the session has ended");
    drop(session);

    // Second open: the service is already installed and only started.
    let mut session = Session::open(&setup);
    report.stage("second open until a return can be delivered", session.wait_ready());

    // 8. A second copy that crashes: refused, not a timeout.
    setup.stand_in_mode("crash");
    let url = secrets.url(sign_in);
    let crashed = undelivered(&session, &secrets, "second copy crashes", &url, 2);
    assert!(crashed.elapsed < Duration::from_secs(30), "{:?}", crashed.elapsed);
    session.wait_browser_return(1, 0, &["adapter_refused"]);
    report_return(&setup, &mut report, "second copy crashes", "adapter_refused", &crashed);

    // 9. The adapter starts the second copy only from the very file the first
    //    copy runs from. Here the app's Windows path is pointed at another
    //    build while every Linux path stays the same, so the handler and the
    //    supervisor pass the return on and only the adapter can refuse it.
    let redirect = setup.other_build_at_app_path();
    setup.stand_in_mode("deliver");
    let url = secrets.url(sign_in);
    let since = unix_ms();
    let redirected = undelivered(&session, &secrets, "another build at the app's path", &url, 2);
    assert!(redirected.connected_ms.is_some(), "the return did not reach the supervisor");
    session.wait_browser_return(2, 0, &["adapter_refused"]);
    assert!(setup.secondary_events(since).is_empty(), "the other build ran");
    drop(redirect);
    report_return(&setup, &mut report, "another build at the app's Windows path", "adapter_refused", &redirected);
    // The session is unharmed: the next return is delivered.
    let url = secrets.url(sign_in);
    deliver(&session, &secrets, "deliver after the refused build", &url, 3);
    session.wait_browser_return(3, 1, &["dispatched"]);

    // 10. A replaced app file: the handler refuses without contacting the
    //     session, which ends itself, and the replacement never starts.
    let replacement = setup.app_executable().with_file_name(".replacement.exe");
    let mut changed = setup.stand_in_bytes.clone();
    changed.extend_from_slice(b"different build");
    std::fs::write(&replacement, &changed).unwrap();
    std::fs::rename(&replacement, setup.app_executable()).unwrap();
    let url = secrets.url(sign_in);
    let since = unix_ms();
    let replaced = session.browser_return(&url);
    assert_refused(&replaced, &secrets, "the app file was replaced");
    assert!(replaced.connected_ms.is_none(), "a return for a replaced app file reached the supervisor");
    report_return(&setup, &mut report, "app file replaced", "refused by the handler", &replaced);
    assert!(setup.secondary_events(since).is_empty(), "the replaced file was started");
    assert_eq!(setup.received().len(), 3);
    let ended = session.wait_ended();
    report.note(format!("After the app file was replaced the session ended by itself: state {}, {}.",
        ended["state"], ended["error_code"]));
    assert_retired_processes(&setup, &session);
    drop(session);

    search_for_codes(&setup, &inputs, &secrets, &mut report, "after both sessions ended");
}

/// Nothing the product wrote or logged holds a code. Controls planted in the
/// bridge's records, the prefix, /tmp and the journal prove the search finds
/// what it looks for; inside the bridge's own files nothing may be unreadable.
fn search_for_codes(setup: &Setup, inputs: &Inputs, secrets: &Secrets, report: &mut Report, when: &str) {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let uid = unsafe { libc::getuid() };
    let roots = [home.clone(), PathBuf::from("/tmp"), PathBuf::from("/var/tmp"),
        PathBuf::from(format!("/run/user/{uid}")), PathBuf::from("/dev/shm")];
    // Sealed runtimes and downloads, the checkout, its build output and the
    // toolchains are not written by the product.
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().to_path_buf();
    let mut skip = vec![setup.m.root.join("runners"), inputs.archives.clone(), repository, target];
    for name in ["work", ".cargo", ".rustup"] { skip.push(home.join(name)); }
    if let Ok(workspace) = std::env::var("GITHUB_WORKSPACE") { skip.push(workspace.into()); }
    let marker = format!("lvb-signin-control-{}", linux_vst_bridge::random_id().unwrap());
    let controls = [PathBuf::from("/tmp"), setup.directory.clone(), setup.drive()].map(|place| place.join(&marker));
    for control in &controls { std::fs::write(control, format!("control {}\n", secrets.control())).unwrap(); }
    let logged = Command::new("systemd-cat").args(["-t", "lvb-signin-control", "echo", secrets.control()]).status().unwrap();
    assert!(logged.success());
    wait_for(|| Command::new("journalctl").args(["--user", "-t", "lvb-signin-control", "-o", "cat", "--no-pager"])
        .output().is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains(secrets.control())), "the journal control");
    let search = privacy::search(secrets, &roots, &skip, &setup.m.root);
    for control in &controls { std::fs::remove_file(control).unwrap(); }
    report.note(format!("Searched {when}: {} files ({} MB) and the user and system journals; {} unreadable \
        outside the bridge's files.", search.files, search.bytes / 1_000_000, search.unreadable));
    for control in &controls {
        assert!(search.controls.contains(&control.display().to_string()), "the search missed its control in {}", control.display());
    }
    assert!(search.controls.iter().any(|place| place.ends_with("journal")), "the search missed its journal control");
    assert!(search.unreadable_strict.is_empty(), "unreadable bridge files: {:?}", search.unreadable_strict);
    assert!(search.found.is_empty(), "a code was written to: {:?}", search.found);
}

fn deliver(session: &Session, secrets: &Secrets, case: &str, url: &str, received: usize) -> Return {
    let r = session.browser_return(url);
    assert!(!secrets.found_in(&r.stdout) && !secrets.found_in(&r.stderr), "the handler printed a code");
    assert!(r.success, "{case}: the handler failed: {}", String::from_utf8_lossy(&r.stderr));
    assert_received(session.setup(), received, url);
    r
}
fn undelivered(session: &Session, secrets: &Secrets, case: &str, url: &str, received: usize) -> Return {
    let r = session.browser_return(url);
    assert_refused(&r, secrets, case);
    assert_eq!(session.setup().received().len(), received, "{case}: the stand-in received a return");
    r
}

fn report_return(setup: &Setup, report: &mut Report, case: &str, outcome: &str, r: &Return) {
    let events = setup.secondary_events(r.started_ms);
    let start = events.iter().find(|(event, _)| event == "start").map(|(_, at)| *at);
    let handed = setup.received().last().map(|line| line.at_ms).filter(|at| *at >= r.started_ms)
        .or_else(|| events.iter().find(|(event, _)| event.starts_with("end")).map(|(_, at)| *at));
    report.browser_return(case, outcome, r, start, handed);
}

fn assert_refused(r: &Return, secrets: &Secrets, why: &str) {
    assert!(!secrets.found_in(&r.stdout) && !secrets.found_in(&r.stderr), "the handler printed a code");
    assert!(!r.success, "the handler reported delivery although {why}");
    assert!(String::from_utf8_lossy(&r.stderr).contains("Native Access login return could not be delivered"),
        "unexpected handler output when {why}: {}", String::from_utf8_lossy(&r.stderr));
}

/// The stand-in's first copy saw exactly this URL, once, beside the
/// adapter's own rendering argument and nothing else.
fn assert_received(setup: &Setup, count: usize, url: &str) {
    let received = setup.received();
    assert_eq!(received.len(), count, "returns received by the first copy");
    let last = received.last().unwrap();
    assert_eq!((last.args, last.gpu_flag, last.urls, last.other), (3, 1, 1, 0), "{last:?}");
    assert_eq!((last.url_sha256.as_str(), last.url_bytes), (hex(&sha2::Sha256::digest(url.as_bytes())).as_str(), url.len()),
        "the first copy received a different URL");
}

fn assert_retired(setup: &Setup, session: &Session, result: &Value) {
    assert_eq!(result["cleanup_confirmed"], true, "{result:#}");
    assert!(matches!(result["state"].as_str(), Some("completed" | "cancelled")), "{}", result["state"]);
    assert_eq!(result["owned_live"], 0);
    assert_eq!(result["dependency"]["dependency_cleanup_disposition"], "graceful_service_retirement",
        "{}", result["dependency"]);
    assert_retired_processes(setup, session);
}
fn assert_retired_processes(setup: &Setup, session: &Session) {
    assert!(!unit_present(&session.unit), "the session unit is still present");
    wait_for(|| prefix_processes(&setup.prefix).is_empty(), "every process in the prefix to end");
    assert_eq!(listeners(&PORTS), 0, "the service's ports are still held");
    assert_eq!(socket_entries(&session.socket_name()), 0, "the return listener is still open");
}

/// Connect as another account; the supervisor must close at once, without
/// its ready word. Silence until the timeout is a different answer.
fn other_user_attempt(user: &str, session: &Session) -> String {
    let script = "import socket,sys\n\
        s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);s.settimeout(10)\n\
        s.connect('\\0'+sys.argv[1]);s.sendall(sys.argv[2].encode())\n\
        try:data=s.recv(4)\n\
        except TimeoutError:data=None\n\
        except OSError:data=b''\n\
        print('timeout' if data is None else 'closed' if data==b'' else 'answered')\n";
    let output = Command::new("sudo").args(["-n", "-u", user, "/usr/bin/python3", "-I", "-c", script])
        .arg(session.socket_name()).arg(&session.operation).output().unwrap();
    assert!(output.status.success(), "the other account could not run: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn wait_for(mut condition: impl FnMut() -> bool, what: &str) {
    let started = std::time::Instant::now();
    while !condition() {
        assert!(started.elapsed() < Duration::from_secs(30), "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(100));
    }
}
