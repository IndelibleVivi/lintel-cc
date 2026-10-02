use lintel_egress::{Config, Event, Proxy};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};
use tauri::State;
use tokio::sync::{oneshot, Mutex as AsyncMutex};

const EVENT_LIMIT: usize = 200;
const COVERAGE: &str = "proxy_connections_only";

struct Channel {
    address: String,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
    events: Arc<Mutex<VecDeque<Event>>>,
}

#[derive(Default)]
pub struct NetworkState {
    channels: AsyncMutex<HashMap<String, Channel>>,
}

fn error(code: &str, message: &str) -> Value {
    json!({"ok":false,"error":{"code":code,"message":message}})
}

fn events(channel: &Channel) -> Vec<Event> {
    channel
        .events
        .lock()
        .map(|events| events.iter().cloned().collect())
        .unwrap_or_default()
}

fn stopped_status() -> Value {
    json!({"ok":true,"data":{
        "running":false,"address":null,"events":[],"coverage":COVERAGE,
        "direct_connections_enforced":false
    }})
}

// The production command always selects the shared core. The private function
// boundary lets synthetic tests observe launch payloads without starting Claude.
impl NetworkState {
    async fn dispatch(&self, payload: Value, core: fn(Value) -> Value) -> Value {
        let Some(id) = payload["environment_id"]
            .as_str()
            .filter(|id| !id.is_empty())
        else {
            return error("environment_required", "请先选择环境");
        };
        // Serializes start/stop/launch so a launch cannot race a stopped channel.
        let mut channels = self.channels.lock().await;
        match payload["op"].as_str().unwrap_or("") {
            "start" => {
                if channels.contains_key(id) {
                    return error("channel_exists", "请先停止此环境的原通道，再应用新规则");
                }
                let mut cfg = match payload.get("config") {
                    None => json!({}),
                    Some(value) if value.is_object() => value.clone(),
                    _ => return error("invalid_config", "通道配置必须是 JSON object"),
                };
                // The selected environment and OS-assigned loopback port are the
                // bridge's authority; callers cannot request a public listener.
                cfg["environment_id"] = json!(id);
                cfg["bind"] = json!("127.0.0.1:0");
                let config: Config = match serde_json::from_value(cfg) {
                    Ok(config) => config,
                    Err(_) => return error("invalid_config", "通道配置无效"),
                };
                let eid = id.to_string();
                match tauri::async_runtime::spawn_blocking(move || {
                    core(json!({"command":"inspect","environment_id":eid}))
                })
                .await
                {
                    Ok(result) if result["ok"] == true => {}
                    Ok(result) => return result,
                    Err(_) => return error("environment_unavailable", "无法检查目标环境"),
                }
                let event_buffer = Arc::new(Mutex::new(VecDeque::new()));
                let observer_events = event_buffer.clone();
                let observer = Arc::new(move |event: Event| {
                    if let Ok(mut events) = observer_events.lock() {
                        if events.len() == EVENT_LIMIT {
                            events.pop_front();
                        }
                        events.push_back(event);
                    }
                });
                let proxy = match Proxy::bind(config, observer).await {
                    Ok(proxy) => proxy,
                    Err(reason) => return error("channel_start_failed", &reason),
                };
                let address = match proxy.local_addr() {
                    Ok(address) => address.to_string(),
                    Err(_) => return error("address_unavailable", "无法取得监听地址"),
                };
                let (stop, stopped) = oneshot::channel();
                let task = tokio::spawn(proxy.serve_until(async move {
                    let _ = stopped.await;
                }));
                channels.insert(
                    id.to_string(),
                    Channel {
                        address: address.clone(),
                        stop,
                        task,
                        events: event_buffer,
                    },
                );
                json!({"ok":true,"data":{
                    "running":true,"address":address,"events":[],"coverage":COVERAGE,
                    "direct_connections_enforced":false,
                    "message":"通道已监听；只有明确配置为使用此通道的客户端才经过它。"
                }})
            }
            "stop" => {
                let Some(channel) = channels.remove(id) else {
                    return stopped_status();
                };
                let _ = channel.stop.send(());
                if !matches!(channel.task.await, Ok(Ok(()))) {
                    return error(
                        "channel_stop_failed",
                        "通道任务已结束，但停止返回异常，请检查结果",
                    );
                }
                let mut response = stopped_status();
                response["data"]["message"] =
                    json!("监听及现有连接已关闭；没有修改系统代理。已有客户端不会由此自动退出。");
                response
            }
            "status" => {
                if let Some(channel) = channels.get(id) {
                    json!({"ok":true,"data":{
                        "running":!channel.task.is_finished(),"address":channel.address,
                        "events":events(channel),"coverage":COVERAGE,
                        "direct_connections_enforced":false
                    }})
                } else {
                    stopped_status()
                }
            }
            "launch" => {
                let Some(channel) = channels.get(id) else {
                    return error("channel_missing", "请先启动当前环境的通道");
                };
                if channel.task.is_finished() {
                    return error("channel_stopped", "通道已停止，请重新启动");
                }
                let proxy_url = format!("http://{}", channel.address);
                let eid = id.to_string();
                match tauri::async_runtime::spawn_blocking(move || {
                    core(json!({"command":"launch","environment_id":eid,"proxy_url":proxy_url}))
                })
                .await
                {
                    Ok(result) => result,
                    Err(_) => error("launch_failed", "启动请求未完成"),
                }
            }
            _ => error("unknown_network_operation", "不支持此通道操作"),
        }
    }
}

#[tauri::command]
pub async fn network_request(
    state: State<'_, NetworkState>,
    payload: Value,
) -> Result<Value, String> {
    Ok(state.dispatch(payload, lintel_core::handle_request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        time::{timeout, Duration},
    };

    const ENVIRONMENT: &str = "00000000-0000-4000-8000-000000000001";
    fn synthetic_core(request: Value) -> Value {
        match request["command"].as_str() {
            Some("inspect") => json!({"ok":true,"data":{"environment":{"id":ENVIRONMENT}}}),
            Some("launch") => json!({"ok":true,"data":request}),
            _ => panic!("unexpected synthetic core call"),
        }
    }
    fn unavailable_core(_: Value) -> Value {
        error("environment_missing", "synthetic missing environment")
    }

    #[tokio::test]
    async fn channel_start_deny_status_launch_payload_and_stop() {
        let state = NetworkState::default();
        let started = state
            .dispatch(
                json!({"op":"start","environment_id":ENVIRONMENT,"config":{
                    "default_action":"deny","bind":"0.0.0.0:9000",
                    "environment_id":"caller-cannot-rebind-identity"
                }}),
                synthetic_core,
            )
            .await;
        assert_eq!(started["ok"], true);
        assert_eq!(started["data"]["direct_connections_enforced"], false);
        let address = started["data"]["address"].as_str().unwrap();
        assert!(address.starts_with("127.0.0.1:"));
        let mut client = TcpStream::connect(address).await.unwrap();
        client.write_all(b"CONNECT denied.synthetic.invalid:443 HTTP/1.1\r\nHost: denied.synthetic.invalid:443\r\nAuthorization: SYNTHETIC_SECRET\r\n\r\n").await.unwrap();
        let mut reply = Vec::new();
        timeout(Duration::from_secs(2), client.read_to_end(&mut reply))
            .await
            .unwrap()
            .unwrap();
        assert!(reply.starts_with(b"HTTP/1.1 403"));
        // Await the event emitted when the completed synthetic connection closes.
        let status = timeout(Duration::from_secs(2), async {
            loop {
                let status = state
                    .dispatch(
                        json!({"op":"status","environment_id":ENVIRONMENT}),
                        synthetic_core,
                    )
                    .await;
                if !status["data"]["events"].as_array().unwrap().is_empty() {
                    break status;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(status["data"]["running"], true);
        assert_eq!(status["data"]["events"][0]["outcome"], "blocked");
        assert_eq!(status["data"]["events"][0]["environment_id"], ENVIRONMENT);
        assert!(!status.to_string().contains("SYNTHETIC_SECRET"));
        let launched = state
            .dispatch(
                json!({"op":"launch","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(launched["data"]["command"], "launch");
        assert_eq!(launched["data"]["environment_id"], ENVIRONMENT);
        assert_eq!(launched["data"]["proxy_url"], format!("http://{address}"));
        let duplicate = state
            .dispatch(
                json!({"op":"start","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(duplicate["error"]["code"], "channel_exists");
        let stopped = state
            .dispatch(
                json!({"op":"stop","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(stopped["ok"], true);
        assert_eq!(stopped["data"]["running"], false);
        assert!(TcpStream::connect(address).await.is_err());
        let status = state
            .dispatch(
                json!({"op":"status","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(status["data"]["address"], Value::Null);
        let unavailable = state
            .dispatch(
                json!({"op":"launch","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(unavailable["error"]["code"], "channel_missing");
    }

    #[tokio::test]
    async fn invalid_configuration_or_missing_environment_never_opens_channel() {
        let state = NetworkState::default();
        for config in [
            json!("invalid"),
            json!({"upstream":"socks5://localhost:1080"}),
        ] {
            let result = state
                .dispatch(
                    json!({"op":"start","environment_id":ENVIRONMENT,"config":config}),
                    synthetic_core,
                )
                .await;
            assert_eq!(result["ok"], false);
            assert!(state.channels.lock().await.is_empty());
        }
        let result = state
            .dispatch(
                json!({"op":"start","environment_id":ENVIRONMENT}),
                unavailable_core,
            )
            .await;
        assert_eq!(result["error"]["code"], "environment_missing");
        assert!(state.channels.lock().await.is_empty());
    }
}
