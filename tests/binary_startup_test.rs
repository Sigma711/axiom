use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

fn unused_loopback_port() -> u16 {
    TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .expect("reserve a loopback port")
        .local_addr()
        .expect("read loopback address")
        .port()
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn released_server_boots_and_serves_the_strategy_api() {
    let port = unused_loopback_port();
    let data_dir =
        std::env::temp_dir().join(format!("axiom-startup-{}-{port}", std::process::id()));
    // The coverage runner puts an instrumented executable beside this harness.
    // Ordinary cargo test still falls back to Cargo's binary path.
    let instrumented = std::env::current_exe()
        .expect("locate test executable")
        .parent()
        .and_then(|deps| deps.parent())
        .map(|target| target.join("axiom"));
    let binary = instrumented
        .filter(|path| path.is_file())
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_axiom").into());
    let mut child = Command::new(binary)
        .env("AXIOM_HOST", "127.0.0.1")
        .env("AXIOM_PORT", port.to_string())
        .env("AXIOM_OFFLINE", "1")
        .env("AXIOM_DATA_DIR", &data_dir)
        .spawn()
        .expect("start AXIOM binary");

    let deadline = Instant::now() + Duration::from_secs(10);
    let response = loop {
        match TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(200),
        ) {
            Ok(mut stream) => {
                stream
                    .write_all(b"GET /api/strategies HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .expect("write HTTP request");
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .expect("read HTTP response");
                break response;
            }
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                terminate(&mut child);
                panic!("AXIOM did not accept connections: {error}");
            }
        }
    };

    terminate(&mut child);
    let _ = std::fs::remove_dir_all(data_dir);
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected response: {response}"
    );
    assert!(
        response.contains("sma_cross"),
        "strategy catalogue missing: {response}"
    );
}
