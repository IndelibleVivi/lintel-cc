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
    active_config: Config,
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
        "running":false,"address":null,"active_config":null,"events":[],"coverage":COVERAGE,
        "direct_connections_enforced":false
    }})
}

// The production command always selects the shared core. The private function
// boundary lets synthetic tests observe launch payloads without starting Claude.
impl NetworkState {
    /// New frozen launches retain the same owned-channel guarantee as the
    /// legacy network launcher. Hold the channel lock through the sole attempt
    /// so stop cannot change the reviewed address halfway through dispatch.
    pub(crate) async fn dispatch_core<F>(&self, payload: Value, core: F) -> Value
    where
        F: Fn(Value) -> Value + Send + 'static,
    {
        let command = payload["command"].as_str().unwrap_or("");
        let target = if command == "plan_launch" {
            payload.clone()
        } else if command == "launch_request" {
            let query = core(json!({"command":"launch_query","request_id":payload["request_id"]}));
            if query["ok"] == true && query["data"]["observed"] == "record" {
                return match tauri::async_runtime::spawn_blocking(move || core(payload)).await {
                    Ok(result) => result,
                    Err(_) => error("launch_uncertain", "原启动请求核对未完成；保留原 ID"),
                };
            }
            let plan = core(json!({"command":"plan_show","plan_id":payload["request_id"]}));
            if plan["ok"] != true {
                return plan;
            }
            plan["data"]["launch_request"].clone()
        } else {
            Value::Null
        };
        let proxy = target["proxy_url"].as_str().filter(|s| !s.is_empty());
        if let Some(proxy) = proxy {
            let channels = self.channels.lock().await;
            let Some(channel) = target["environment_id"]
                .as_str()
                .and_then(|id| channels.get(id))
            else {
                return error(
                    "channel_missing",
                    "批准中的本机通道已不存在；请启动通道后重新核对目标",
                );
            };
            if channel.task.is_finished() || proxy != format!("http://{}", channel.address) {
                return error(
                    "channel_changed",
                    "通道已停止或地址已变化；请重新核对启动目标",
                );
            }
            let result = match tauri::async_runtime::spawn_blocking(move || core(payload)).await {
                Ok(result) => result,
                Err(_) => error(
                    "launch_uncertain",
                    "启动请求未完成；核对原启动 ID，不重新提交",
                ),
            };
            drop(channels);
            result
        } else {
            match tauri::async_runtime::spawn_blocking(move || core(payload)).await {
                Ok(result) => result,
                Err(_) => error("core_request_failed", "本地执行器未能完成请求"),
            }
        }
    }
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
                let mut config: Config = match serde_json::from_value(cfg) {
                    Ok(config) => config,
                    Err(_) => return error("invalid_config", "通道配置无效"),
                };
                // Keep the canonical, validated value actually passed to bind.
                // validate normalizes rule hosts but leaves the upstream URL intact.
                if let Err(reason) = config.validate() {
                    return error("invalid_config", reason);
                }
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
                let proxy = match Proxy::bind(config.clone(), observer).await {
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
                        active_config: config.clone(),
                        stop,
                        task,
                        events: event_buffer,
                    },
                );
                json!({"ok":true,"data":{
                    "running":true,"address":address,"active_config":config,"events":[],"coverage":COVERAGE,
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
                    let running = !channel.task.is_finished();
                    json!({"ok":true,"data":{
                        "running":running,"address":channel.address,
                        "active_config":if running { Some(&channel.active_config) } else { None },
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
    #[tokio::test]
    async fn frozen_proxy_requires_owned_live_address_but_original_query_survives_stop() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let state = NetworkState::default();
        let started=state.dispatch(json!({"op":"start","environment_id":ENVIRONMENT,"config":{"default_action":"deny"}}),synthetic_core).await;
        assert_eq!(started["ok"], true, "{started}");
        let address = started["data"]["address"].as_str().unwrap().to_string();
        let attempts = Arc::new(AtomicUsize::new(0));
        let core = |address: String, attempts: Arc<AtomicUsize>| {
            move |request: Value| match request["command"].as_str().unwrap() {
                "launch_query" if request["request_id"] == "recorded" => {
                    json!({"ok":true,"data":{"status":"launch_requested","observed":"record"}})
                }
                "launch_query" => {
                    json!({"ok":true,"data":{"status":"planned","observed":"plan","request_id":request["request_id"]}})
                }
                "plan_show" => {
                    json!({"ok":true,"data":{"launch_request":{"environment_id":ENVIRONMENT,"proxy_url":format!("http://{address}")}}})
                }
                "launch_request" => {
                    if request["request_id"] != "recorded" {
                        attempts.fetch_add(1, Ordering::SeqCst);
                    }
                    json!({"ok":true,"data":{"status":"launch_requested"}})
                }
                "plan_launch" => json!({"ok":true,"data":request}),
                _ => panic!("unexpected synthetic core command"),
            }
        };
        let preview = json!({"command":"plan_launch","environment_id":ENVIRONMENT,"proxy_url":format!("http://{address}")});
        assert_eq!(
            state
                .dispatch_core(preview, core(address.clone(), attempts.clone()))
                .await["ok"],
            true
        );
        let wrong = state
            .dispatch_core(
                json!({"command":"launch_request","request_id":"new"}),
                core("127.0.0.1:1".into(), attempts.clone()),
            )
            .await;
        assert_eq!(wrong["error"]["code"], "channel_changed");
        assert_eq!(attempts.load(Ordering::SeqCst), 0);
        assert_eq!(
            state
                .dispatch_core(
                    json!({"command":"launch_request","request_id":"new"}),
                    core(address.clone(), attempts.clone())
                )
                .await["ok"],
            true
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        state
            .dispatch(
                json!({"op":"stop","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(
            state
                .dispatch_core(
                    json!({"command":"launch_request","request_id":"new"}),
                    core(address.clone(), attempts.clone())
                )
                .await["error"]["code"],
            "channel_missing"
        );
        assert_eq!(
            state
                .dispatch_core(
                    json!({"command":"launch_request","request_id":"recorded"}),
                    core(address, attempts.clone())
                )
                .await["ok"],
            true
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
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
                    "environment_id":"caller-cannot-rebind-identity",
                    "blocked":[{"host":"DENIED.Synthetic.Invalid.","ports":[443]}],
                    "allowed":[{"host":"LOCALHOST.","ports":[8080,8443]}],
                    "upstream":"http://LOCALHOST.:8080",
                    "max_connections":3,"connect_timeout_seconds":2,
                    "connection_lifetime_seconds":30
                }}),
                synthetic_core,
            )
            .await;
        assert_eq!(started["ok"], true);
        assert_eq!(started["data"]["direct_connections_enforced"], false);
        let active_config = &started["data"]["active_config"];
        assert_eq!(
            active_config,
            &json!({
                "environment_id":ENVIRONMENT,"bind":"127.0.0.1:0",
                "default_action":"deny",
                "blocked":[{"host":"denied.synthetic.invalid","ports":[443]}],
                "allowed":[{"host":"localhost","ports":[8080,8443]}],
                "upstream":"http://LOCALHOST.:8080",
                "max_connections":3,"connect_timeout_seconds":2,
                "connection_lifetime_seconds":30
            })
        );
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
        assert_eq!(&status["data"]["active_config"], active_config);
        assert_eq!(status["data"]["events"][0]["outcome"], "blocked");
        assert_eq!(status["data"]["events"][0]["provenance"], "explicit_block");
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
        assert_eq!(stopped["data"]["active_config"], Value::Null);
        assert!(TcpStream::connect(address).await.is_err());
        let status = state
            .dispatch(
                json!({"op":"status","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(status["data"]["address"], Value::Null);
        assert_eq!(status["data"]["active_config"], Value::Null);
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
            json!({"upstream":"http://user:secret@localhost:8080"}),
            json!({"blocked":[{"host":"*.synthetic.invalid"}]}),
            json!({"allowed":[{"host":"localhost","ports":[0]}]}),
            json!({"default_action":"unknown"}),
            json!({"blocked":[{"host":"synthetic.invalid"}]}),
            json!({"max_connections":0}),
            json!({"unknown_rule":true}),
        ] {
            let result = state
                .dispatch(
                    json!({"op":"start","environment_id":ENVIRONMENT,"config":config}),
                    synthetic_core,
                )
                .await;
            assert_eq!(result["ok"], false);
            assert_eq!(result["error"]["code"], "invalid_config");
            assert!(state.channels.lock().await.is_empty());
            let status = state
                .dispatch(
                    json!({"op":"status","environment_id":ENVIRONMENT}),
                    synthetic_core,
                )
                .await;
            assert_eq!(status["data"]["active_config"], Value::Null);
        }
        let result = state
            .dispatch(
                json!({"op":"start","environment_id":ENVIRONMENT,"config":{"default_action":"deny"}}),
                unavailable_core,
            )
            .await;
        assert_eq!(result["error"]["code"], "environment_missing");
        assert!(state.channels.lock().await.is_empty());
    }

    #[tokio::test]
    async fn readback_and_shutdown_are_scoped_to_each_environment() {
        let state = NetworkState::default();
        let other = "00000000-0000-4000-8000-000000000002";
        for (id, action, host) in [
            (ENVIRONMENT, "deny", "first.invalid"),
            (other, "allow", "second.invalid"),
        ] {
            let response = state
                .dispatch(
                    json!({"op":"start","environment_id":id,"config":{
                        "default_action":action,"blocked":[{"host":host,"ports":[]}]
                    }}),
                    synthetic_core,
                )
                .await;
            assert_eq!(response["ok"], true);
            assert_eq!(response["data"]["active_config"]["environment_id"], id);
            assert_eq!(response["data"]["active_config"]["default_action"], action);
        }
        state
            .dispatch(
                json!({"op":"stop","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        let first = state
            .dispatch(
                json!({"op":"status","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        let second = state
            .dispatch(
                json!({"op":"status","environment_id":other}),
                synthetic_core,
            )
            .await;
        assert_eq!(first["data"]["active_config"], Value::Null);
        assert_eq!(second["data"]["running"], true);
        assert_eq!(
            second["data"]["active_config"]["blocked"][0]["host"],
            "second.invalid"
        );

        // A crashed/aborted task must not keep advertising its previous rules as active.
        state.channels.lock().await.get(other).unwrap().task.abort();
        let ended = timeout(Duration::from_secs(2), async {
            loop {
                let status = state
                    .dispatch(
                        json!({"op":"status","environment_id":other}),
                        synthetic_core,
                    )
                    .await;
                if status["data"]["running"] == false {
                    break status;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(ended["data"]["active_config"], Value::Null);
        state
            .dispatch(json!({"op":"stop","environment_id":other}), synthetic_core)
            .await;
    }
}
