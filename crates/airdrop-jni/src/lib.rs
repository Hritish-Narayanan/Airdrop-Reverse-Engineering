use airdrop_core::{
    discover_airdrop_peers, run_airdrop_server, AirDropBlePayload, AirDropClient,
    AirDropServerConfig,
};
use jni::objects::{GlobalRef, JClass, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::{JNIEnv, JavaVM};
use mdns_sd::ServiceDaemon;
use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

static JVM: Mutex<Option<JavaVM>> = Mutex::new(None);
static BRIDGE_CLASS: Mutex<Option<GlobalRef>> = Mutex::new(None);
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
    mut env: JNIEnv,
    class: JClass,
) {
    let _ = tracing_subscriber::fmt::try_init();
    airdrop_core::ensure_crypto_provider();

    if let Ok(vm) = env.get_java_vm() {
        let mut guard = JVM.lock().unwrap();
        *guard = Some(vm);
    }

    let actual_class: JObject = if let Ok(is_class) = env.is_instance_of(&class, "java/lang/Class") {
        if is_class {
            class.into()
        } else if let Ok(c) = env.get_object_class(&class) {
            c.into()
        } else {
            class.into()
        }
    } else {
        class.into()
    };

    if let Ok(global_cls) = env.new_global_ref(actual_class) {
        let mut guard = BRIDGE_CLASS.lock().unwrap();
        *guard = Some(global_cls);
    }

    airdrop_core::set_consent_listener(|details| {
        let jvm_guard = JVM.lock().unwrap();
        let cls_guard = BRIDGE_CLASS.lock().unwrap();
        if let (Some(jvm), Some(global_cls)) = (jvm_guard.as_ref(), cls_guard.as_ref()) {
            if let Ok(mut env) = jvm.attach_current_thread() {
                let cls = unsafe { JClass::from_raw(global_cls.as_raw()) };
                if let (Ok(id), Ok(sender), Ok(model)) = (
                    env.new_string(&details.transfer_id),
                    env.new_string(&details.sender_name),
                    env.new_string(&details.sender_model),
                ) {
                    let icon_obj: JObject = match details.icon_data.as_ref() {
                        Some(s) => env.new_string(s).map(JObject::from).unwrap_or_else(|_| JObject::null()),
                        None => JObject::null(),
                    };
                    let file_count = details.file_count as jint;
                    let total_bytes = details.total_bytes as jlong;
                    let _ = env.call_static_method(
                        cls,
                        "onIncomingConsent",
                        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;IJLjava/lang/String;)V",
                        &[
                            (&id).into(),
                            (&sender).into(),
                            (&model).into(),
                            file_count.into(),
                            total_bytes.into(),
                            (&icon_obj).into(),
                        ],
                    );
                }
            }
        }
    });
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
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_generateNearbyActionPayload(
    env: JNIEnv,
    _class: JClass,
    flags: jint,
) -> jbyteArray {
    let payload = AirDropBlePayload::new_nearby_action(flags as u8);
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
        if let Some(old) = guard.take() {
            old.store(true, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(350));
        }
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
pub extern "system" fn Java_com_airdrop_android_AirDropBridge_respondToConsent(
    mut env: JNIEnv,
    _class: JClass,
    transfer_id: JString,
    accept: jboolean,
) -> jboolean {
    let id_str: String = match env.get_string(&transfer_id) {
        Ok(s) => s.into(),
        Err(_) => return JNI_FALSE,
    };

    if airdrop_core::respond_to_consent(&id_str, accept == JNI_TRUE) {
        JNI_TRUE
    } else {
        JNI_FALSE
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
            let target_url = if let Some(ip) = p.ip_addresses.iter().find(|ip| ip.is_ipv4()).or_else(|| p.ip_addresses.first()) {
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
