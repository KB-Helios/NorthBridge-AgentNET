use serde::Serialize;
use std::fs;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};

const GATEWAY_ADMIN_PORT: u16 = 15000;
const GATEWAY_LLM_PORT: u16 = 14000;

#[derive(Default)]
struct HostState {
    gateway: Mutex<Option<Child>>,
    lifecycle: Mutex<()>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GatewayStatus {
    state: String,
    admin_url: String,
    llm_base_url: String,
    config_path: String,
    detail: String,
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn gateway_binary(app: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("agentgateway")
            .join("agentgateway.exe"));
    }
    app.path()
        .resource_dir()
        .map_err(|error| error.to_string())
        .map(|dir| {
            dir.join("resources")
                .join("agentgateway")
                .join("agentgateway.exe")
        })
}

fn gateway_config(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("agentgateway");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join("northbridge-agentgateway.yaml");
    let temporary = dir.join("northbridge-agentgateway.yaml.tmp");
    let content = format!("config:\n  adminAddr: 127.0.0.1:{GATEWAY_ADMIN_PORT}\n  logging:\n    format: json\nllm:\n  port: {GATEWAY_LLM_PORT}\n  models: []\n");
    fs::write(&temporary, content).map_err(|error| error.to_string())?;
    fs::rename(&temporary, &path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(120));
    }
    false
}

fn empty_status(app: &AppHandle, state: &str, detail: String) -> GatewayStatus {
    let config_path = app_data_dir(app)
        .map(|dir| {
            dir.join("agentgateway")
                .join("northbridge-agentgateway.yaml")
                .display()
                .to_string()
        })
        .unwrap_or_default();
    GatewayStatus {
        state: state.to_string(),
        admin_url: format!("http://127.0.0.1:{GATEWAY_ADMIN_PORT}"),
        llm_base_url: format!("http://127.0.0.1:{GATEWAY_LLM_PORT}/v1"),
        config_path,
        detail,
    }
}

fn refresh_status(app: &AppHandle, state: &HostState) -> GatewayStatus {
    let mut gateway = match state.gateway.lock() {
        Ok(gateway) => gateway,
        Err(_) => {
            return empty_status(
                app,
                "error",
                "The gateway supervisor lock is unavailable.".to_string(),
            )
        }
    };
    match gateway.as_mut() {
        Some(child) => match child.try_wait() {
            Ok(None) => {
                if TcpStream::connect(("127.0.0.1", GATEWAY_ADMIN_PORT)).is_ok() {
                    empty_status(
                        app,
                        "running",
                        "AgentGateway is bound to local loopback endpoints.".to_string(),
                    )
                } else {
                    empty_status(
                        app,
                        "starting",
                        "AgentGateway process is alive but administrative endpoint is not ready.".to_string(),
                    )
                }
            }
            Ok(Some(exit)) => {
                let detail = format!("AgentGateway exited with {exit}.");
                *gateway = None;
                empty_status(app, "stopped", detail)
            }
            Err(error) => empty_status(app, "error", error.to_string()),
        },
        None => empty_status(app, "stopped", "AgentGateway is not running.".to_string()),
    }
}

fn start_gateway_internal(app: &AppHandle, state: &HostState) -> Result<GatewayStatus, String> {
    let _lifecycle_guard = state
        .lifecycle
        .lock()
        .map_err(|_| "The gateway lifecycle lock is unavailable.".to_string())?;
    let current = refresh_status(app, state);
    if current.state == "running" {
        return Ok(current);
    }
    let binary = gateway_binary(app)?;
    if !binary.is_file() {
        return Err(format!(
            "AgentGateway binary is missing at {}.",
            binary.display()
        ));
    }
    let config = gateway_config(app)?;
    let child = Command::new(binary)
        .args(["-f", config.to_string_lossy().as_ref()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut gateway = state
        .gateway
        .lock()
        .map_err(|_| "The gateway supervisor lock is unavailable.".to_string())?;
    *gateway = Some(child);
    drop(gateway);
    if !wait_for_port(GATEWAY_ADMIN_PORT, Duration::from_secs(8)) {
        let mut gateway = state
            .gateway
            .lock()
            .map_err(|_| "The gateway supervisor lock is unavailable.".to_string())?;
        if let Some(mut child) = gateway.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        return Err(
            "AgentGateway did not become ready on its administrative loopback port.".to_string(),
        );
    }
    Ok(refresh_status(app, state))
}

fn graceful_gateway_stop() {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, GATEWAY_ADMIN_PORT));
    if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
        let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
        let _ = stream.write_all(
            b"GET /quitquitquit HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );
    }
}

fn stop_gateway_internal(app: &AppHandle, state: &HostState) -> GatewayStatus {
    let _lifecycle_guard = match state.lifecycle.lock() {
        Ok(guard) => guard,
        Err(_) => return empty_status(app, "error", "The gateway lifecycle lock is unavailable.".to_string()),
    };
    graceful_gateway_stop();
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let status = refresh_status(app, state);
        if status.state != "running" || Instant::now() >= deadline {
            if status.state == "running" {
                if let Ok(mut gateway) = state.gateway.lock() {
                    if let Some(child) = gateway.as_mut() {
                        let _ = child.kill();
                    }
                    *gateway = None;
                }
            }
            return refresh_status(app, state);
        }
        thread::sleep(Duration::from_millis(120));
    }
}

#[tauri::command]
fn gateway_status(app: AppHandle, state: State<HostState>) -> GatewayStatus {
    refresh_status(&app, &state)
}

#[tauri::command]
fn start_gateway(app: AppHandle, state: State<HostState>) -> Result<GatewayStatus, String> {
    start_gateway_internal(&app, &state)
}

#[tauri::command]
fn stop_gateway(app: AppHandle, state: State<HostState>) -> GatewayStatus {
    stop_gateway_internal(&app, &state)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(HostState::default())
        .setup(|app| {
            let state = app.state::<HostState>();
            let _ = start_gateway_internal(&app.handle(), &state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            gateway_status,
            start_gateway,
            stop_gateway
        ])
        .build(tauri::generate_context!())
        .expect("NorthBridge desktop host failed to build")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app_handle.state::<HostState>();
                stop_gateway_internal(app_handle, &state);
            }
        });
}
