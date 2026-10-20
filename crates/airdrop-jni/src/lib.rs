use airdrop_core::{
    discover_airdrop_peers, run_airdrop_server, AirDropBlePayload, AirDropClient,
    AirDropServerConfig,
};
use jni::objects::{JClass, JObjectArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use mdns_sd::ServiceDaemon;
use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

static RECEIVER_SHUTDOWN: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

#[derive(Serialize)]
struct PeerJson {
    instance_name: String,
    host_name: String,
    port: u16,
    target_url: String,
    ip_addresses: Vec<String>,
}

#[derive(Serialize)]
struct SendResultJson {
    success: bool,
    error: Option<String>,
}

#[no_mangle]
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_init(
    _env: JNIEnv,
    _class: JClass,
) {
    let _ = tracing_subscriber::fmt::try_init();
    airdrop_core::ensure_crypto_provider();
}

#[no_mangle]
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_generateBlePayload(
    env: JNIEnv,
    _class: JClass,
    everyone: jboolean,
) -> jbyteArray {
    let payload = if everyone == JNI_TRUE {
        AirDropBlePayload::new_everyone()
    } else {
        AirDropBlePayload::new_contacts(vec![])
    };
    let encoded = payload.encode();

    match env.byte_array_from_slice(&encoded) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_startReceiver(
    mut env: JNIEnv,
    _class: JClass,
    port: jint,
    download_dir: JString,
    name: JString,
    auto_accept: jboolean,
) -> jboolean {
    let dir_str: String = match env.get_string(&download_dir) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };
    let name_str: String = match env.get_string(&name) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };

    let p = port as u16;
    let bind_addr: SocketAddr = match format!("0.0.0.0:{}", p).parse() {
        Ok(a) => a,
        Err(_) => return JNI_FALSE,
    };

    let dir_path = PathBuf::from(&dir_str);
    if let Err(e) = std::fs::create_dir_all(&dir_path) {
        error!("Failed to create download dir: {}", e);
        return JNI_FALSE;
    }

    let config = AirDropServerConfig {
        bind_addr,
        computer_name: name_str,
        download_dir: dir_path,
        auto_accept: auto_accept == JNI_TRUE,
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_signal = shutdown.clone();

    {
        let mut guard = RECEIVER_SHUTDOWN.lock().unwrap();
        *guard = Some(shutdown);
    }

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to create tokio runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            info!("AirDrop receiver thread started on port {}", p);
            if let Err(e) = run_airdrop_server(config, shutdown_signal).await {
                error!("AirDrop server error: {}", e);
            }
        });
    });

    JNI_TRUE
}

#[no_mangle]
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_stopReceiver(
    _env: JNIEnv,
    _class: JClass,
) {
    let mut guard = RECEIVER_SHUTDOWN.lock().unwrap();
    if let Some(shutdown) = guard.take() {
        shutdown.store(true, Ordering::Relaxed);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_sendFiles(
    mut env: JNIEnv,
    _class: JClass,
    target_url: JString,
    files_array: JObjectArray,
    sender_name: JString,
    sender_model: JString,
) -> jstring {
    let target: String = match env.get_string(&target_url) {
        Ok(s) => s.into(),
        Err(e) => return return_json_error(&mut env, format!("Invalid target_url: {}", e)),
    };
    let name: String = match env.get_string(&sender_name) {
        Ok(s) => s.into(),
        Err(_) => "Android Phone".to_string(),
    };
    let model: String = match env.get_string(&sender_model) {
        Ok(s) => s.into(),
        Err(_) => "Pixel".to_string(),
    };

    let len = match env.get_array_length(&files_array) {
        Ok(l) => l,
        Err(e) => return return_json_error(&mut env, format!("Invalid files_array: {}", e)),
    };

    let mut files = Vec::new();
    for i in 0..len {
        let elem = match env.get_object_array_element(&files_array, i) {
            Ok(obj) => obj,
            Err(e) => return return_json_error(&mut env, format!("Array access error: {}", e)),
        };
        let jstr: JString = elem.into();
        let path_str: String = match env.get_string(&jstr) {
            Ok(s) => s.into(),
            Err(e) => return return_json_error(&mut env, format!("String parse error: {}", e)),
        };
        files.push(PathBuf::from(path_str));
    }

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => return return_json_error(&mut env, format!("Tokio runtime error: {}", e)),
    };

    let result = rt.block_on(async {
        let client = AirDropClient::new(name, model);
        client.send_files(&target, &files).await
    });

    let res_obj = match result {
        Ok(_) => SendResultJson {
            success: true,
            error: None,
        },
        Err(e) => SendResultJson {
            success: false,
            error: Some(e.to_string()),
        },
    };

    let json_str = serde_json::to_string(&res_obj).unwrap_or_default();
    match env.new_string(json_str) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_scanPeers(
    mut env: JNIEnv,
    _class: JClass,
    timeout_secs: jint,
) -> jstring {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => return return_json_error(&mut env, format!("Tokio runtime error: {}", e)),
    };

    let peers = rt.block_on(async {
        let mdns = match ServiceDaemon::new() {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };
        discover_airdrop_peers(&mdns, Duration::from_secs(timeout_secs.max(1) as u64))
            .await
            .unwrap_or_default()
    });

    let peer_list: Vec<PeerJson> = peers
        .into_iter()
        .map(|p| {
            let target_url = if let Some(ip) = p.ip_addresses.first() {
                if ip.is_ipv6() {
                    format!("https://[{}]:{}", ip, p.port)
                } else {
                    format!("https://{}:{}", ip, p.port)
                }
            } else {
                format!("https://{}:{}", p.host_name, p.port)
            };

            PeerJson {
                instance_name: p.instance_name,
                host_name: p.host_name,
                port: p.port,
                target_url,
                ip_addresses: p.ip_addresses.into_iter().map(|ip| ip.to_string()).collect(),
            }
        })
        .collect();

    let json_str = serde_json::to_string(&peer_list).unwrap_or_else(|_| "[]".to_string());
    match env.new_string(json_str) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn return_json_error(env: &mut JNIEnv, msg: String) -> jstring {
    let res = SendResultJson {
        success: false,
        error: Some(msg),
    };
    let json_str = serde_json::to_string(&res).unwrap_or_default();
    match env.new_string(json_str) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
