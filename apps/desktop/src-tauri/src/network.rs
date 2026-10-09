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
    binding: String,
    address: String,
    active_config: Config,
    // The same Proxy instance that owns the listener; used only for controlled
    // rule tests so provenance comes from this exact channel.
    proxy: Arc<Proxy>,
    stop: Option<oneshot::Sender<()>>,
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
    /// Readonly gate for the coordinator's updater install decision. True when
    /// any environment still owns an unfinished proxy task (a live channel or a
    /// channel that has not yet been stopped/cleared). This never mutates state
    /// and never inspects a destination.
    pub(crate) async fn has_active_channels(&self) -> bool {
        self.channels
            .lock()
            .await
            .values()
            .any(|channel| !channel.task.is_finished())
    }

    /// New frozen launches retain the same owned-channel guarantee as the
    /// legacy network launcher. Hold the channel lock through the sole attempt
    /// so stop cannot change the reviewed address halfway through dispatch.
    pub(crate) async fn dispatch_core<F>(&self, mut payload: Value, core: F) -> Value
    where
        F: Fn(Value) -> Value + Send + 'static,
    {
        let command = payload["command"].as_str().unwrap_or("").to_string();
        if command == "plan_network_restore" || command == "plan_restore" {
            let original = core(json!({"command":"job","job_id":payload["job_id"]}));
            if original["ok"] == true && original["data"]["network_change"].is_object() {
                let plan =
                    core(json!({"command":"plan_show","plan_id":original["data"]["plan_id"]}));
                if plan["ok"] != true {
                    return plan;
                }
                let mut probe = payload
                    .get("probe")
                    .cloned()
                    .unwrap_or_else(|| plan["data"]["network"]["probe"].clone());
                // Restoration remains available after the original channel stops.
                // The new approved preview visibly freezes the available paths.
                let channels = self.channels.lock().await;
                let active = probe["proxy_url"].as_str().and_then(|proxy| {
                    channels.values().find(|channel| {
                        !channel.task.is_finished()
                            && format!("http://{}", channel.address) == proxy
                    })
                });
                if let Some(channel) = active {
                    probe["proxy_binding"] = json!(channel.binding);
                } else if let Some(probe) = probe.as_object_mut() {
                    probe.remove("proxy_url");
                    probe.remove("proxy_binding");
                }
                payload["command"] = json!("plan_network_restore");
                payload["probe"] = probe;
                let result = match tauri::async_runtime::spawn_blocking(move || core(payload)).await
                {
                    Ok(result) => result,
                    Err(_) => error(
                        "network_preview_failed",
                        "共享网络恢复预览未完成；没有据此执行",
                    ),
                };
                drop(channels);
                return result;
            }
        }
        let network_probe = if command == "network_probe" {
            Some(payload.clone())
        } else if command == "plan_network_ipv6" {
            Some(payload["probe"].clone())
        } else if command == "execute" {
            // The original accepted job survives stopped channels and App restart.
            let original = core(json!({"command":"job","job_id":payload["plan_id"]}));
            if original["ok"] == true {
                return original;
            }
            let plan = core(json!({"command":"plan_show","plan_id":payload["plan_id"]}));
            if plan["ok"] == true && plan["data"]["network"].is_object() {
                Some(plan["data"]["network"]["probe"].clone())
            } else {
                None
            }
        } else {
            None
        };
        if let Some(probe) = network_probe {
            if let Some(proxy) = probe["proxy_url"].as_str() {
                let channels = self.channels.lock().await;
                let Some(channel) = channels.values().find(|channel| {
                    format!("http://{}", channel.address) == proxy && !channel.task.is_finished()
                }) else {
                    return error(
                        "channel_changed",
                        "所选通道已停止或被替换；请用当前通道重新预览，原任务仍可查询",
                    );
                };
                if command == "execute" && probe["proxy_binding"] != channel.binding {
                    return error(
                        "channel_changed",
                        "批准后的通道实例已变化；相同端口也需要重新预览",
                    );
                }
                if command == "network_probe" {
                    payload["proxy_binding"] = json!(channel.binding);
                }
                if command == "plan_network_ipv6" {
                    payload["probe"]["proxy_binding"] = json!(channel.binding);
                }
                let result = match tauri::async_runtime::spawn_blocking(move || core(payload)).await
                {
                    Ok(result) => result,
                    Err(_) => error(
                        "network_request_uncertain",
                        "网络请求未完成；若已批准写入，请查询原任务",
                    ),
                };
                drop(channels);
                return result;
            }
        }
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
    async fn dispatch<F>(&self, payload: Value, core: F) -> Value
    where
        F: Fn(Value) -> Value + Send + 'static,
    {
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
                let proxy = Arc::new(proxy);
                let (stop, stopped) = oneshot::channel();
                let binding = format!(
                    "{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                );
                let serving = proxy.clone();
                let task = tokio::spawn(async move {
                    serving
                        .serve_until_shared(async move {
                            let _ = stopped.await;
                        })
                        .await
                });
                channels.insert(
                    id.to_string(),
                    Channel {
                        binding: binding.clone(),
                        address: address.clone(),
                        active_config: config.clone(),
                        proxy: proxy.clone(),
                        stop: Some(stop),
                        task,
                        events: event_buffer,
                    },
                );
                json!({"ok":true,"data":{
                    "running":true,"address":address,"channel_binding":binding,"active_config":config,"events":[],"coverage":COVERAGE,
                    "direct_connections_enforced":false,
                    "message":"通道已监听；只有明确配置为使用此通道的客户端才经过它。"
                }})
            }
            "stop" => {
                let Some(channel) = channels.get_mut(id) else {
                    return stopped_status();
                };
                if let Some(stop) = channel.stop.take() {
                    let _ = stop.send(());
                }
                // Keep the unfinished channel visible to the updater even if
                // this IPC future is cancelled while awaiting shutdown. A later
                // explicit stop can finish querying this same task.
                let stopped = (&mut channel.task).await;
                channels.remove(id);
                if !matches!(stopped, Ok(Ok(()))) {
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
                        "running":running,"address":channel.address,"channel_binding":channel.binding,
                        "active_config":if running { Some(&channel.active_config) } else { None },
                        "events":events(channel),"coverage":COVERAGE,
                        "direct_connections_enforced":false
                    }})
                } else {
                    stopped_status()
                }
            }
            // Native controlled rule test. Accepts only a catalog id and proves
            // the exact target is currently explicitly blocked by this channel's
            // canonical handle. It never connects the destination, never resolves
            // public DNS, and never touches an upstream. The returned kind/test ID
            // mark it as an owner-origin request; ordinary Claude/proxy traffic
            // never carries this marker.
            "rule_test" => {
                // Validate the finite fields and catalog id FIRST, before any
                // channel lookup, so a malformed request is never masked by a
                // missing/replaced channel.
                let extra = payload
                    .as_object()
                    .map(|object| {
                        object.keys().any(|key| {
                            !["op", "environment_id", "telemetry_id", "channel_binding"]
                                .contains(&key.as_str())
                        })
                    })
                    .unwrap_or(true);
                if extra {
                    return error("invalid_request", "rule_test 不接受其他字段");
                }
                let Some(telemetry_id) = payload["telemetry_id"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                else {
                    return error("invalid_request", "rule_test 需要 telemetry_id");
                };
                if lintel_egress::telemetry::host_for(telemetry_id).is_none() {
                    return error("rule_test_failed", "unknown_telemetry_destination");
                }
                let Some(binding) = payload["channel_binding"]
                    .as_str()
                    .filter(|value| !value.is_empty() && value.len() <= 128)
                else {
                    return error("invalid_request", "rule_test 需要原通道的 channel_binding");
                };
                let Some(channel) = channels.get(id) else {
                    return error("channel_missing", "请先启动当前环境的通道再进行受控测试");
                };
                if channel.task.is_finished() {
                    return error(
                        "channel_stopped",
                        "通道已停止或被替换；请用当前通道重新测试",
                    );
                }
                if binding != channel.binding {
                    return error("channel_changed", "通道实例已改变；请刷新后重新测试");
                }
                // The verified outcome comes only from this exact channel's own
                // serving Proxy; a failure is never reported as a pass.
                match channel.proxy.rule_test(telemetry_id).await {
                    Ok(outcome) => json!({"ok":true,"data":{
                        "kind":"rule_test","test_id":outcome.test_id,
                        "owner_origin":"lintel_app_proxy","telemetry_id":telemetry_id,
                        "environment_id":id,"channel_binding":channel.binding,
                        "outcome":outcome
                    }}),
                    Err(reason) => error("rule_test_failed", reason),
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

// The root app owns the single `network_request` IPC command (guarded by the
// ActivityGate). This module keeps the callable bridge function so the guard
// wrapper can invoke it after acquiring its permit.
pub async fn network_request(
    state: State<'_, NetworkState>,
    payload: Value,
    permit: tokio::sync::OwnedRwLockReadGuard<()>,
) -> Result<Value, String> {
    Ok(state
        .dispatch(payload, move |payload| {
            // A blocking core call retains admission after cancellation of its
            // parent IPC future, just like the core/browser/remote adapters.
            let _permit = &permit;
            lintel_core::handle_request(payload)
        })
        .await)
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
    async fn cancelled_stop_keeps_unfinished_channel_visible_to_updater() {
        let state = Arc::new(NetworkState::default());
        let proxy = Arc::new(
            Proxy::bind(Config::default(), Arc::new(|_| {}))
                .await
                .unwrap(),
        );
        let (stop, stopped) = oneshot::channel();
        let (finish, finished) = oneshot::channel();
        let stopping = Arc::new(tokio::sync::Notify::new());
        let observed = stopping.clone();
        let task = tokio::spawn(async move {
            let _ = stopped.await;
            observed.notify_one();
            let _ = finished.await;
            Ok(())
        });
        state.channels.lock().await.insert(
            ENVIRONMENT.into(),
            Channel {
                binding: "synthetic-cancelled-stop".into(),
                address: proxy.local_addr().unwrap().to_string(),
                active_config: Config::default(),
                proxy,
                stop: Some(stop),
                task,
                events: Arc::new(Mutex::new(VecDeque::new())),
            },
        );
        let owner = state.clone();
        let request = tokio::spawn(async move {
            owner
                .dispatch(
                    json!({"op":"stop","environment_id":ENVIRONMENT}),
                    synthetic_core,
                )
                .await
        });
        timeout(Duration::from_secs(5), stopping.notified())
            .await
            .unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(
            state.has_active_channels().await,
            "unfinished shutdown blocks installation"
        );
        let _ = finish.send(());
        let result = state
            .dispatch(
                json!({"op":"stop","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(result["ok"], true, "{result}");
        assert!(!state.has_active_channels().await);
    }

    #[tokio::test]
    async fn cancelled_network_inspection_retains_its_blocking_activity_permit() {
        let state = Arc::new(NetworkState::default());
        let gate = Arc::new(crate::activity::ActivityGate::default());
        let permit = gate.operation().unwrap();
        let entered = Arc::new(tokio::sync::Notify::new());
        let observed = entered.clone();
        let (finish, finished) = std::sync::mpsc::channel();
        let owner = state.clone();
        let request = tokio::spawn(async move {
            owner
                .dispatch(
                    json!({"op":"start","environment_id":ENVIRONMENT,"config":{"default_action":"allow"}}),
                    move |payload| {
                        let _permit = &permit;
                        observed.notify_one();
                        let _ = finished.recv_timeout(Duration::from_secs(5));
                        synthetic_core(payload)
                    },
                )
                .await
        });
        timeout(Duration::from_secs(5), entered.notified())
            .await
            .unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(
            gate.installation().is_err(),
            "cancelled IPC does not cancel a blocking call"
        );
        finish.send(()).unwrap();
        timeout(Duration::from_secs(5), async {
            while gate.installation().is_err() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(
            !state.has_active_channels().await,
            "cancelled start never publishes a channel"
        );
    }
    #[tokio::test]
    async fn network_probe_and_approval_bind_the_instance_and_keep_original_queries() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let state = NetworkState::default();
        state.dispatch(json!({"op":"start","environment_id":ENVIRONMENT,"config":{"default_action":"allow"}}), synthetic_core).await;
        let status = state
            .dispatch(
                json!({"op":"status","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        let proxy = format!("http://{}", status["data"]["address"].as_str().unwrap());
        let binding = status["data"]["channel_binding"].clone();
        let probe = json!({"proxy_url":proxy,"ipv4_url":"https://v4.example.invalid","ipv6_url":"https://v6.example.invalid"});
        let result = state
            .dispatch_core(
                json!({"command":"network_probe","proxy_url":proxy,"proxy_binding":"forged"}),
                |request| json!({"ok":true,"data":request}),
            )
            .await;
        assert_eq!(result["data"]["proxy_binding"], binding);
        let mut frozen = probe.clone();
        frozen["proxy_binding"] = binding.clone();
        let attempts = Arc::new(AtomicUsize::new(0));
        let callback = |frozen: Value, attempts: Arc<AtomicUsize>| {
            move |request: Value| match request["command"].as_str().unwrap() {
                "job" if request["job_id"] == "original" => {
                    json!({"ok":true,"data":{"status":"completed","id":"original"}})
                }
                "job" => json!({"ok":false,"error":{"code":"job_not_found"}}),
                "plan_show" => json!({"ok":true,"data":{"network":{"probe":frozen}}}),
                "execute" => {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    json!({"ok":true,"data":{"status":"completed"}})
                }
                _ => panic!("unexpected command"),
            }
        };
        // Same address and config, different owned instance: old approval fails.
        state
            .channels
            .lock()
            .await
            .get_mut(ENVIRONMENT)
            .unwrap()
            .binding = "replacement".into();
        let changed = state
            .dispatch_core(
                json!({"command":"execute","plan_id":"new"}),
                callback(frozen.clone(), attempts.clone()),
            )
            .await;
        assert_eq!(changed["error"]["code"], "channel_changed");
        assert_eq!(attempts.load(Ordering::SeqCst), 0);
        state
            .dispatch(
                json!({"op":"stop","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        let original = state
            .dispatch_core(
                json!({"command":"execute","plan_id":"original"}),
                callback(frozen, attempts.clone()),
            )
            .await;
        assert_eq!(original["data"]["id"], "original");
        assert_eq!(attempts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn network_restore_previews_available_paths_after_original_channel_stops() {
        let state = NetworkState::default();
        let result = state.dispatch_core(json!({"command":"plan_network_restore","job_id":"original"}), |request| match request["command"].as_str().unwrap() {
            "job" => json!({"ok":true,"data":{"plan_id":"original","network_change":{"scope":"host_shared"}}}),
            "plan_show" => json!({"ok":true,"data":{"network":{"probe":{"ipv4_url":"https://v4.example.invalid","ipv6_url":"https://v6.example.invalid","proxy_url":"http://127.0.0.1:7890","proxy_binding":"retired"}}}}),
            "plan_network_restore" => { assert!(request["probe"].get("proxy_url").is_none()); assert!(request["probe"].get("proxy_binding").is_none()); json!({"ok":true,"data":request}) },
            _ => panic!("unexpected command"),
        }).await;
        assert_eq!(result["ok"], true);
        assert_eq!(
            result["data"]["probe"]["ipv6_url"],
            "https://v6.example.invalid"
        );
    }
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

    // The native controlled rule test refuses without an active channel and
    // validates its finite fields before touching a Proxy. No loopback needed.
    #[tokio::test]
    async fn rule_test_requires_an_active_channel_and_catalog_id() {
        let state = NetworkState::default();
        let no_channel = state
            .dispatch(
                json!({"op":"rule_test","environment_id":ENVIRONMENT,"telemetry_id":"datadog_logs_intake","channel_binding":"synthetic-binding"}),
                synthetic_core,
            )
            .await;
        assert_eq!(no_channel["ok"], false);
        assert_eq!(no_channel["error"]["code"], "channel_missing");
        // An explicit extra field is rejected before any channel check.
        for bad in [
            json!({"op":"rule_test","environment_id":ENVIRONMENT,"telemetry_id":"datadog_logs_intake","host":"api.anthropic.com"}),
            json!({"op":"rule_test","environment_id":ENVIRONMENT,"telemetry_id":"datadog_logs_intake","approval":"x"}),
        ] {
            let result = state.dispatch(bad, synthetic_core).await;
            assert_eq!(result["ok"], false, "{result}");
            assert_eq!(result["error"]["code"], "invalid_request");
        }
    }

    #[tokio::test]
    async fn controlled_test_freezes_live_binding_and_reads_completed_proxy_event() {
        let state = NetworkState::default();
        let started = state.dispatch(json!({"op":"start","environment_id":ENVIRONMENT,"config":{
            "default_action":"allow","blocked":[{"host":"http-intake.logs.us5.datadoghq.com","ports":[443]}]
        }}), synthetic_core).await;
        assert_eq!(started["ok"], true, "{started}");
        assert!(state.has_active_channels().await);
        let binding = started["data"]["channel_binding"].clone();
        let stale = state.dispatch(json!({"op":"rule_test","environment_id":ENVIRONMENT,"telemetry_id":"datadog_logs_intake","channel_binding":"old-instance"}), synthetic_core).await;
        assert_eq!(stale["error"]["code"], "channel_changed");
        let tested = state.dispatch(json!({"op":"rule_test","environment_id":ENVIRONMENT,"telemetry_id":"datadog_logs_intake","channel_binding":binding}), synthetic_core).await;
        assert_eq!(tested["ok"], true, "{tested}");
        assert_eq!(tested["data"]["outcome"]["result"], "blocked_explicit");
        assert_eq!(
            tested["data"]["test_id"],
            tested["data"]["outcome"]["test_id"]
        );
        let status = state
            .dispatch(
                json!({"op":"status","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert_eq!(status["data"]["events"].as_array().unwrap().len(), 1);
        let event = &status["data"]["events"][0];
        assert_eq!(event["origin"], "rule_test");
        assert_eq!(event["test_id"], tested["data"]["test_id"]);
        assert_eq!(event["outcome"], "blocked");
        state
            .dispatch(
                json!({"op":"stop","environment_id":ENVIRONMENT}),
                synthetic_core,
            )
            .await;
        assert!(!state.has_active_channels().await);
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
                "address_family":"system",
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
