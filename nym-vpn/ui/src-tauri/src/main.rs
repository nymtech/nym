// Prevents additional console window on Windows in release, DO NOT REMOVE!!
//commented out for dev
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Result;
use tauri::api::path::{config_dir, data_dir};
use tokio::sync::Mutex;
use tracing::info;

use std::time::{Duration, Instant};
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};
use tauri::Manager;
use windows_service::service::{ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode, ServiceInfo, ServiceStartType, ServiceStatus};
use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
use std::sync::mpsc;
use hbb_common::{allow_err, anyhow::anyhow, bail, config::{Config}, log, ResultType, sleep, tokio};
use winapi::{
    um::{
        processthreadsapi::{
            GetCurrentProcessId,
        }
    },
};

use windows_service::{define_windows_service, service::{
    ServiceState,
    ServiceType,
}, service_control_handler};
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use tauri_plugin_autostart::MacosLauncher;
use windows_service::service_control_handler::ServiceControlHandlerResult;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};


mod commands;
mod error;
mod fs;
mod states;

use commands::*;
use states::app::AppState;

use crate::fs::config::AppConfig;
use crate::fs::data::AppData;
use crate::fs::storage::AppStorage;

const APP_DIR: &str = "nymvpn";
const APP_DATA_FILE: &str = "app-data.toml";
const APP_CONFIG_FILE: &str = "config.toml";
const VPN_SERVICE_NAME: &'static str = "nym_vpn_service";
const VPN_EXE: &'static str = "nymvpn.exe";
const SERVICE_DISPLAY_NAME: &'static str = "NymVPN Manager";


#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

fn main() -> Result<()> {
    dotenvy::dotenv()?;

    // uses RUST_LOG value for logging level
    // eg. RUST_LOG=tauri=debug,nymvpn_ui=trace
    tracing_subscriber::fmt::init();

    let mut app_data_path = data_dir().ok_or(anyhow!("Failed to retrieve data directory"))?;
    app_data_path.push(APP_DIR);
    let app_data_store = AppStorage::<AppData>::new(app_data_path, APP_DATA_FILE, None);

    let mut app_config_path = config_dir().ok_or(anyhow!("Failed to retrieve config directory"))?;
    app_config_path.push(APP_DIR);
    let app_config_store = AppStorage::<AppConfig>::new(app_config_path, APP_CONFIG_FILE, None);

    println!("Installing the service");
    if let Err(_err) = install_service() {
        eprintln!("Installing error: {_err}");
    }
    println!("Registering the service");
    start_os_service();
    println!("Starting service");
    if let Err(_err) = start_service() {
        eprintln!("Start error: {_err}");
    }
    println!("Service should be started");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let open = CustomMenuItem::new("open".to_string(), "Open");
    let tray_menu = SystemTrayMenu::new()
        .add_item(open)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    info!("Starting tauri app");

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--flag1", "--flag2"])))
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);
            app.emit_all("single-instance", Payload { args: argv, cwd }).unwrap();
        }))
        .manage(Arc::new(Mutex::new(AppState::default())))
        .manage(Arc::new(Mutex::new(app_data_store)))
        .manage(Arc::new(Mutex::new(app_config_store)))
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "quit" => {
                        let did_uninstall = uninstall_service();
                        if did_uninstall {
                            println!("Uninstalled service");
                        } else {
                            println!("Failed to uninstall service")
                        }
                    }
                    "open" => {
                        let window = app.get_window("main").unwrap();
                        window.show().unwrap();
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .setup(|_app| {
            info!("app setup");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            connection::get_connection_state,
            connection::connect,
            connection::disconnect,
            settings::save_user_settings,
            settings::set_user_settings,
        ])
        .build(tauri::generate_context!()).expect("error while building app")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            _ => {}
        });
    Ok(())
}

fn start_service() -> windows_service::Result<()> {
    let service_manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START;
    let service = service_manager.open_service(VPN_SERVICE_NAME, service_access)?;
    println!("Opened connection to service");
    if service.query_status()?.current_state == ServiceState::Stopped {
        println!("Service is stopped, starting.");
        service.start(&["Starting service"])?;
    }
    drop(service);
    Ok(())
}

fn stop_service() -> windows_service::Result<()> {
    let service_manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP;
    let service = service_manager.open_service(VPN_SERVICE_NAME, service_access)?;
    println!("Opened connection to service");
    if service.query_status()?.current_state == ServiceState::Running {
        println!("Service is running, stopping.");
        let status = service.stop()?;
        if status.current_state == ServiceState::Running {
            println!("Failed to stop service")
        }
    }
    drop(service);
    Ok(())
}

#[cfg(windows)]
fn install_service() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = std::env::current_exe()
        .unwrap()
        .with_file_name(VPN_EXE);

    let service_info = ServiceInfo {
        name: OsString::from(VPN_SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START)?;

    service.set_description("The service to manage NymVPN")?;
    drop(service);
    println!("Set description");
    Ok(())
}

//this doesn't work that great, uninstall_service works better
#[cfg(windows)]
async fn remove_service() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(VPN_SERVICE_NAME, service_access)?;

    // The service will be marked for deletion as long as this function call succeeds.
    // However, it will not be deleted from the database until it is stopped and all open handles to it are closed.
    service.delete()?;
    // Our handle to it is not closed yet. So we can still query it.
    if service.query_status()?.current_state != ServiceState::Stopped {
        // If the service cannot be stopped, it will be deleted when the system restarts.
        service.stop()?;
    }
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    // Win32 API does not give us a way to wait for service deletion.
    // To check if the service is deleted from the database, we have to poll it ourselves.
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(VPN_SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                println!("service is deleted.");
                return Ok(());
            }
        }
        sleep(1f32).await;
    }
    println!("service is marked for deletion.");
    Ok(())
}

fn get_current_pid() -> u32 {
    unsafe { GetCurrentProcessId() }
}

pub fn uninstall_service() -> bool {
    log::info!("Uninstalling service...");
    let filter = format!(" /FI \"PID ne {}\"", get_current_pid());
    Config::set_option("stop-service".into(), "Y".into());
    let cmds = format!(
        "
    chcp 65001
    sc stop {app_name}
    sc delete {app_name}
    if exist \"%PROGRAMDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\{app_name} Tray.lnk\" del /f /q \"%PROGRAMDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\{app_name} Tray.lnk\"
    taskkill /F /IM {broker_exe}
    taskkill /F /IM {app_name}.exe{filter}
    ",
        app_name = VPN_SERVICE_NAME,
        broker_exe = VPN_EXE
    );
    if let Err(err) = run_cmds(cmds, false, "uninstall") {
        Config::set_option("stop-service".into(), "".into());
        log::debug!("{err}");
        return true;
    }
    std::process::exit(0);
}

fn run_cmds(cmds: String, show: bool, tip: &str) -> ResultType<()> {
    let tmp = write_cmds(cmds, "bat", tip)?;
    let tmp2 = get_undone_file(&tmp)?;
    let tmp_fn = tmp.to_str().unwrap_or("");
    let res = runas::Command::new("cmd")
        .args(&["/C", &tmp_fn])
        .show(show)
        .force_prompt(true)
        .status();
    if !show {
        allow_err!(std::fs::remove_file(tmp));
    }
    let _ = res?;
    if tmp2.exists() {
        allow_err!(std::fs::remove_file(tmp2));
        bail!("{} failed", tip);
    }
    Ok(())
}

fn write_cmds(cmds: String, ext: &str, tip: &str) -> ResultType<PathBuf> {
    let mut cmds = cmds;
    let mut tmp = std::env::temp_dir();
    // When dir contains these characters, the bat file will not execute in elevated mode.
    if vec!["&", "@", "^"]
        .drain(..)
        .any(|s| tmp.to_string_lossy().to_string().contains(s))
    {
        if let Ok(dir) = user_accessible_folder() {
            tmp = dir;
        }
    }
    tmp.push(format!("{}_{}.{}", VPN_SERVICE_NAME, tip, ext));
    let mut file = std::fs::File::create(&tmp)?;
    if ext == "bat" {
        let tmp2 = get_undone_file(&tmp)?;
        std::fs::File::create(&tmp2).ok();
        cmds = format!(
            "
{cmds}
if exist \"{path}\" del /f /q \"{path}\"
",
            path = tmp2.to_string_lossy()
        );
    }
    // in case cmds mixed with \r\n and \n, make sure all ending with \r\n
    // in some windows, \r\n required for cmd file to run
    cmds = cmds.replace("\r\n", "\n").replace("\n", "\r\n");
    if ext == "vbs" {
        let mut v: Vec<u16> = cmds.encode_utf16().collect();
        // utf8 -> utf16le which vbs support it only
        file.write_all(to_le(&mut v))?;
    } else {
        file.write_all(cmds.as_bytes())?;
    }
    file.sync_all()?;
    return Ok(tmp);
}

fn to_le(v: &mut [u16]) -> &[u8] {
    for b in v.iter_mut() {
        *b = b.to_le()
    }
    unsafe { v.align_to().1 }
}

pub fn user_accessible_folder() -> ResultType<PathBuf> {
    let disk = std::env::var("SystemDrive").unwrap_or("C:".to_string());
    let dir1 = PathBuf::from(format!("{}\\ProgramData", disk));
    // NOTICE: "C:\Windows\Temp" requires permanent authorization.
    let dir2 = PathBuf::from(format!("{}\\Windows\\Temp", disk));
    let dir;
    if dir1.exists() {
        dir = dir1;
    } else if dir2.exists() {
        dir = dir2;
    } else {
        bail!("no vaild user accessible folder");
    }
    Ok(dir)
}

fn get_undone_file(tmp: &PathBuf) -> ResultType<PathBuf> {
    let mut tmp1 = tmp.clone();
    tmp1.set_file_name(format!(
        "{}.undone",
        tmp.file_name()
            .ok_or(anyhow!("Failed to get filename of {:?}", tmp))?
            .to_string_lossy()
    ));
    Ok(tmp1)
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(arguments: Vec<OsString>) {
    if let Err(e) = run_service(arguments) {
        log::error!("run_service failed: {}", e);
    }
}

pub fn start_os_service() {
    if let Err(e) =
        windows_service::service_dispatcher::start(VPN_SERVICE_NAME, ffi_service_main)
    {
        log::error!("start_service failed: {}", e);
    }
}

const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

#[tokio::main(flavor = "current_thread")]
async fn run_service(_arguments: Vec<OsString>) -> ResultType<()> {
    let (send, recv) = mpsc::channel();
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        log::info!("Got service control event: {:?}", control_event);
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                //I can never get this to work
                //for now, just uninstall the service if we need to stop it
                println!("Received stop command");
                if let Err(_err) = send.send(()) {
                    eprintln!("Failed to send stop to sleep: {_err}");
                }
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler
    let status_handle = service_control_handler::register(VPN_SERVICE_NAME, event_handler)?;

    let next_status = ServiceStatus {
        // Should match the one from system service registry
        service_type: SERVICE_TYPE,
        // The new state
        current_state: ServiceState::Running,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint: 0,
        // Only used for pending states, otherwise must be zero
        wait_hint: Duration::default(),
        process_id: None,
    };

    // Tell the system that the service is running now
    status_handle.set_service_status(next_status)?;

    //TODO do work here, loop for now
    loop {
        //request to local server for testing
        let resp = reqwest::blocking::get("http://localhost:8080/")?.text()?;
        println!("{:#?}", resp);
        if let Ok(_) = recv.recv_timeout(Duration::from_secs(2)) {
            // Sleep was interrupted
            break;
        }
    }

    println!("This should only happen after work");

    //after work, update status of service
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    Ok(())
}
