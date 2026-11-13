//! Authentication-source cleanup inside the existing plan/job authority.
//! No original credential bytes are written to plans, archives or receipts.
use crate::{auth_sources, err, now, storage::*, string, work, Engine, Result};
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

const SCHEMA: &str = "lintel.auth-cleanup/1";

pub(crate) fn settings_binding(path: &Path, extra: &Value) -> Result<Value> {
    let mut value = crate::storage::snapshot(path)?;
    if extra["authentication"]["schema"] == SCHEMA && !value.is_null() {
        let nonce = string(&extra["authentication"], "nonce")?;
        value["digest"] = json!(digest(&serde_json::to_vec(&json!([
            "lintel.auth-settings/1",
            nonce,
            value["digest"]
        ]))?));
    }
    Ok(value)
}

fn project(r: &Value) -> Result<Option<PathBuf>> {
    match r.get("project_cwd") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(path)) => {
            let path = PathBuf::from(path);
            guard(&path)?;
            if !fs::metadata(&path).is_ok_and(|m| m.is_dir()) {
                return Err(err("project_missing", "项目目录不可访问；没有创建或启动它"));
            }
            Ok(Some(path))
        }
        _ => Err(err(
            "invalid_request",
            "project_cwd 必须是目标主机上的绝对目录",
        )),
    }
}

fn flag(r: &Value, name: &str) -> Result<bool> {
    match r.get(name) {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        _ => Err(err("invalid_request", "认证操作的开关必须为 boolean")),
    }
}

impl Engine {
    pub(crate) fn auth_inventory(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let project = project(r)?;
        let mut inventory = auth_sources::inspect(
            &self.home,
            Path::new(string(&e, "root")?),
            project.as_deref(),
        )?;
        inventory["environment_id"] = e["id"].clone();
        inventory["checked_at"] = json!(now());
        Ok(inventory)
    }

    pub(crate) fn plan_auth_cleanup(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        self.block_managed(&e)?;
        let recipe = string(r, "recipe")?;
        if !["repair_login", "reset_client", "retire"].contains(&recipe) {
            return Err(err("unsupported_recipe", "未知清理配方"));
        }
        if flag(r, "official_logout")? {
            return Err(err("official_logout_scope_unsafe", "当前官方注销可能同时移除已保存的 MCP 登录和插件凭据，无法证明只处理 Claude 登录；未调用注销。请使用精确本地来源清理，并按官方说明单独核对 Keychain 与共享登录。"));
        }
        if r["writers_confirmed_stopped"] != true {
            return Err(err(
                "writers_confirmation_required",
                "先关闭目标终端、IDE 与自动重启来源，再预览清理",
            ));
        }
        if !self.known_writers(&e)?.is_empty() {
            return Err(err(
                "writers_active",
                "仍检测到 Claude 写入进程；请核对并关闭目标写入者，Lintel 不会全局杀进程",
            ));
        }
        let services = self.check_cleanup_services(&e)?;
        let selected_project = project(r)?;
        let inventory = self.auth_inventory(r)?;
        let ids = match r.get("auth_source_ids") {
            None => inventory["sources"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|s| s["default_selected"] == true && s["removable"] == true)
                .filter_map(|s| s["id"].as_str().map(str::to_owned))
                .collect::<Vec<_>>(),
            Some(Value::Array(values)) if values.len() <= 128 => values
                .iter()
                .map(|v| {
                    v.as_str()
                        .filter(|s| !s.is_empty() && s.len() <= 256)
                        .map(str::to_owned)
                        .ok_or_else(|| {
                            err("invalid_request", "认证来源选择必须为有限的来源 ID 列表")
                        })
                })
                .collect::<Result<Vec<_>>>()?,
            _ => {
                return Err(err(
                    "invalid_request",
                    "认证来源选择必须为最多 128 项的 ID 列表",
                ))
            }
        };
        let nonce = crate::id();
        let mut authentication = auth_sources::freeze(
            &self.home,
            Path::new(string(&e, "root")?),
            selected_project.as_deref(),
            &ids,
            &nonce,
        )?;
        authentication["schema"] = json!(SCHEMA);
        authentication["nonce"] = json!(nonce);
        authentication["project_cwd"] = json!(selected_project);
        authentication["context"] = inventory["context"].clone();
        authentication["runtime_verification_requested"] = json!(flag(r, "verify_auth_status")?);
        authentication["executable"] = e["executable"].clone();
        // The optional status command is a separately disclosed process action.
        // Reuse the launch executable identity so approval cannot silently follow an update.
        authentication["executable_snapshot"] =
            if authentication["runtime_verification_requested"] == true {
                match e["executable"].as_str() {
                    Some(exe) => crate::launch::exec_identity(Path::new(exe))?,
                    None => Value::Null,
                }
            } else {
                Value::Null
            };
        let categories = if recipe == "repair_login" {
            vec![]
        } else {
            work::categories(r)?
        };
        let manifest = work::manifest(Path::new(string(&e, "root")?), &categories)?;
        let activation = work::activation(r, &categories);
        let frozen_target = if recipe == "reset_client" {
            work::freeze_new_target(&self.state, &manifest, activation["instructions"] == true)?
        } else {
            Value::Null
        };
        let mut actions = vec![
            json!({"id":"quiescence","label":"复查目标写入者与冻结的认证来源","reversible":false}),
        ];
        if recipe != "repair_login" {
            actions.push(json!({"id":"archive","label":"先加密归档所选工作；不复制认证存储","reversible":false}));
        }
        if recipe == "reset_client" {
            actions.push(json!({"id":"rebuild","label":"建立新环境并迁入选中工作（在清理之前）","reversible":false}));
        }
        for (index, group) in authentication["groups"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            actions.push(json!({"id":format!("auth_edit_{index}"),"label":format!("精确移除所选认证声明：{}",group["path"].as_str().or_else(||group["service"].as_str()).unwrap_or("")),"reversible":false,"source_ids":group["source_ids"],"locators":group["locators"],"credential_backup":false}));
        }
        if recipe == "retire" {
            actions.push(json!({"id":"retire","label":"停用此环境的启动入口；保留原工作目录","reversible":true}));
        }
        if authentication["runtime_verification_requested"] == true {
            actions.push(json!({"id":"auth_status","label":"显式运行当前执行器上下文的官方认证状态命令","reversible":false}));
        }
        actions.push(json!({"id":"auth_verify","label":"独立复查所选来源；其他上下文与服务端状态分别报告","reversible":false}));
        self.plan(&e, "cleanup", match recipe { "repair_login" => "清理所选本地认证来源", "reset_client" => "保全工作并重建环境", _ => "清理所选认证并退役环境" }, json!([]),
            vec!["所有原始会话、指令、项目文件与工作内容", "未选择的配置、MCP、插件与未知凭据字段", "其他运行上下文、其他机器和服务端状态不自动改变"],
            json!(actions), json!({"recipe":recipe,"services":services,"official_logout":false,"authentication":authentication,"categories":categories,"manifest":manifest,"archive_passphrase_required":recipe!="repair_login","frozen_target":frozen_target,"work_purpose":work::purposes(&categories),"activate":activation}))
    }

    pub(crate) fn check_auth_cleanup(&self, e: &Value, p: &Value, r: &Value) -> Result<()> {
        let a = &p["extra"]["authentication"];
        if a["schema"] != SCHEMA || p["extra"]["official_logout"] == true {
            return Err(err(
                "stale_auth_plan",
                "旧清理预览缺少精确来源保全约束；请重新预览。已接受的原任务仍只查询，不重放。",
            ));
        }
        if !self.known_writers(e)?.is_empty() {
            return Err(err(
                "writers_active",
                "预览后出现 Claude 写入进程；未继续清理",
            ));
        }
        if json!(self.check_cleanup_services(e)?) != p["extra"]["services"] {
            return Err(err(
                "stale_service_plan",
                "预览后服务停写证据变化；未继续清理",
            ));
        }
        auth_sources::check(&a["groups"], string(a, "nonce")?)?;
        if a["runtime_verification_requested"] == true {
            self.check_auth_executable(e, a)?;
        }
        if p["extra"]["archive_passphrase_required"] == true {
            work::check_passphrase(r)?;
        }
        Ok(())
    }

    fn check_auth_executable(&self, e: &Value, a: &Value) -> Result<()> {
        // Absence is part of the frozen approval too. A later installation or
        // discovery must not turn a missing status adapter into permission to
        // run a program that was never identified in the original preview.
        let exe = a["executable"].as_str();
        if e["executable"] != a["executable"]
            || self.executable().as_deref() != exe
            || match exe {
                Some(exe) => {
                    crate::launch::exec_identity(Path::new(exe))? != a["executable_snapshot"]
                }
                None => !a["executable_snapshot"].is_null(),
            }
        {
            return Err(err(
                "executable_changed",
                "认证检查程序在预览后变化；请重新预览",
            ));
        }
        Ok(())
    }

    pub(crate) fn execute_auth_cleanup(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let recipe = string(&p["extra"], "recipe")?;
        let a = &p["extra"]["authentication"];
        let result = (|| -> Result<()> {
            if recipe != "repair_login" {
                self.archive_work(e, p, r, j, journal)?;
            }
            if recipe == "reset_client" {
                self.migrate_to_new_root(e, p, r, j, journal)?;
            }
            j["steps"].as_array_mut().unwrap().push(json!({"id":"quiescence","label":"清理前复查","status":"executing","message":"核对写入者、服务与每份冻结来源；保全产物已完成时继续保留。"}));
            save(journal, j)?;
            self.check_auth_cleanup(e, p, r)?;
            j["steps"].as_array_mut().unwrap().last_mut().unwrap()["status"] = json!("completed");
            save(journal, j)?;
            for (index, group) in a["groups"]
                .as_array()
                .ok_or_else(|| err("invalid_plan", "缺少认证来源组"))?
                .iter()
                .enumerate()
            {
                // Recheck before every shared-file boundary as well as acceptance.
                if !self.known_writers(e)?.is_empty() {
                    return Err(err(
                        "writers_active",
                        "清理期间出现写入进程，剩余来源保持不变",
                    ));
                }
                edit_source(group, string(a, "nonce")?, index, j, journal)?;
            }
            if recipe == "retire" {
                let mut inventory = self.inventory()?;
                for entry in &mut inventory {
                    if entry["id"] == e["id"] {
                        entry["status"] = json!("retired");
                    }
                }
                save(&self.state.join("inventory.json"), &json!(inventory))?;
                j["steps"].as_array_mut().unwrap().push(json!({"id":"retire","label":"环境已退役","status":"completed","message":"启动入口已停用，原目录和工作保留。"}));
            }
            Ok(())
        })();
        // Reinspection is independent of mutation success; it never repeats edits.
        let report = self.auth_verification(
            e,
            Some(p),
            Some(j),
            a["runtime_verification_requested"] == true,
        )?;
        let local_done = report["local_sources"]
            .as_array()
            .is_some_and(|sources| sources.iter().all(|s| s["state"] == "removed"))
            && report["retained_sensitive_material"]
                .as_array()
                .is_some_and(Vec::is_empty);
        j["auth_verification"] = report;
        j["remote_revocation"] = json!("unverified");
        j["local_cleanup"] = json!(if result.is_ok() && local_done {
            "completed"
        } else {
            "unverified"
        });
        j["steps"].as_array_mut().unwrap().push(json!({"id":"auth_verify","label":"认证来源复查","status":if local_done{"completed"}else{"unverified"},"message":"只核对原选择；其他进程、未选择的 Keychain 来源、共享 profile 和服务端撤销保留各自覆盖状态。"}));
        save(journal, j)?;
        result?;
        if !local_done {
            return Err(err(
                "auth_cleanup_unverified",
                "所选来源或中断保留材料尚未确认移除；查看原任务与逐来源结果",
            ));
        }
        // Preserve the established local-only cleanup status. The task result
        // and coverage explain completion of the explicitly selected scope.
        j["status"] = json!("partially_completed");
        j["warnings"].as_array_mut().unwrap().push(json!(
            "所选本地声明已处理；现有进程环境不会随文件编辑改变，服务端 token 撤销未验证。"
        ));
        Ok(())
    }

    pub(crate) fn auth_verify(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let observe_runtime = flag(r, "observe_runtime")?;
        match r.get("job_id") {
            None => self.auth_verification(&e, None, None, observe_runtime),
            Some(_) => {
                let id = crate::safe_id(r, "job_id")?;
                let j = self.load_job(&self.path("jobs", &id), &id)?;
                let p = load(&self.path("plans", &id))?;
                if j["environment_id"] != e["id"]
                    || p["environment_id"] != e["id"]
                    || p["root"] != e["root"]
                {
                    return Err(err("auth_verification_scope", "原任务与所选环境不匹配"));
                }
                let mut unhashed = p.clone();
                unhashed
                    .as_object_mut()
                    .ok_or_else(|| err("invalid_plan", "原计划损坏"))?
                    .remove("hash");
                if p["hash"] != digest(&serde_json::to_vec(&unhashed)?) {
                    return Err(err("plan_changed", "原计划已变化；不能据此复查来源"));
                }
                if p["extra"]["authentication"]["schema"] != SCHEMA {
                    return Err(err(
                        "legacy_auth_coverage",
                        "此历史任务没有逐来源范围；可不带 job_id 检查当前来源，原记录保留",
                    ));
                }
                self.auth_verification(&e, Some(&p), Some(&j), observe_runtime)
            }
        }
    }

    fn auth_verification(
        &self,
        e: &Value,
        p: Option<&Value>,
        j: Option<&Value>,
        observe_runtime: bool,
    ) -> Result<Value> {
        let a = p.map(|p| &p["extra"]["authentication"]);
        // Even a status command may run a helper or refresh client state. Run
        // the explicitly requested process first; the final local readback
        // must observe any credentials it recreates, rather than report an
        // earlier absence as the state after verification.
        let runtime = if !observe_runtime {
            json!({"state":"not_requested"})
        } else {
            let observation = (|| {
                if let Some(a) = a.filter(|a| a["runtime_verification_requested"] == true) {
                    self.check_auth_executable(e, a)?;
                }
                self.auth_probe(&json!({"environment_id":e["id"]}))
            })();
            match observation {
                Ok(observation) => json!({"state":"observed","observation":observation}),
                Err(error) => json!({"state":"unavailable","error_code":error.code}),
            }
        };
        let req = json!({"environment_id":e["id"],"project_cwd":a.map(|a|&a["project_cwd"]).unwrap_or(&Value::Null)});
        let inventory = self.auth_inventory(&req).unwrap_or_else(|error| json!({"schema":"lintel.auth-sources/1","sources":[],"error_code":error.code,"coverage":"unknown"}));
        let local_sources: Vec<Value> = a.into_iter().flat_map(|a|a["sources"].as_array().into_iter().flatten()).map(|source| {
            let group = a.and_then(|a|a["groups"].as_array()).and_then(|groups|groups.iter().find(|group|group["source_ids"].as_array().is_some_and(|ids|ids.contains(&source["id"]))));
            let state = if group.is_none_or(|group|auth_sources::check_parent(group).is_err()) {
                "unknown".into()
            } else { auth_sources::source_state_bound(source, group.unwrap()).unwrap_or_else(|_| "unknown".into()) };
            json!({"id":source["id"],"kind":source["kind"],"path":source["path"],"service":source["service"],"locator":source["locator"],"state":state})
        }).collect();
        let mut retained = vec![];
        for step in j
            .into_iter()
            .flat_map(|j| j["steps"].as_array().into_iter().flatten())
        {
            // A missing old pathname is not proof that a held original was
            // destroyed. A writer may have moved the entire directory. Keep
            // the last known locator and the lost-ownership uncertainty; do
            // not search for or delete a renamed directory.
            if step["auth_edit_phase"].is_string()
                && step["auth_edit_phase"] != "completed"
                && step["auth_quarantine_identity"].is_object()
            {
                if let Some(path) = step["quarantine_path"].as_str() {
                    if check_edit_directory(Path::new(path), &step["auth_quarantine_identity"])
                        .is_err()
                    {
                        retained.push(json!({"path":path,"state":"location_unknown","may_contain_credentials":true}));
                        continue;
                    }
                }
            }
            let initial_count = retained.len();
            for key in ["auth_original_path", "auth_staged_path"] {
                if let Some(path) = step[key].as_str() {
                    let state =
                        match guard(Path::new(path)).and_then(|_| {
                            match fs::symlink_metadata(path) {
                                Ok(_) => Ok(true),
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                                Err(e) => Err(e.into()),
                            }
                        }) {
                            Ok(false) => continue,
                            Ok(true) => "retained",
                            Err(_) => "unknown",
                        };
                    retained
                        .push(json!({"path":path,"state":state,"may_contain_credentials":true}));
                }
            }
            // An interrupted atomic writer may leave its owned random staging
            // name inside this exact directory. Absence of the two final names
            // cannot establish absence of all temporary credential material.
            if retained.len() == initial_count && step["auth_edit_phase"].is_string() {
                if let Some(path) = step["quarantine_path"].as_str() {
                    match guard(Path::new(path)).and_then(|_|match fs::symlink_metadata(path) {
                        Ok(_) => Ok(true), Err(e) if e.kind()==std::io::ErrorKind::NotFound => Ok(false), Err(e)=>Err(e.into()),
                    }) {
                        Ok(false) => {},
                        result => retained.push(json!({"path":path,"state":if result.is_ok(){"retained_container"}else{"unknown"},"may_contain_credentials":true})),
                    }
                }
            }
        }
        Ok(
            json!({"schema":"lintel.auth-verification/1","environment_id":e["id"],"original_job_id":j.map(|j| &j["id"]),"checked_at":now(),"local_sources":local_sources,"inventory":inventory,"runtime":runtime,"other_contexts":"unverified","server_revocation":"unverified","retained_sensitive_material":retained,"coverage":"selected_sources_only"}),
        )
    }
}

/// No-replace publication around an exact owned original. The original is
/// moved, never copied to a backup. Its private locator is durable before the
/// move; an uncertain original job is query-only. Noncooperating writers can
/// still exist, so each boundary verifies rather than promising filesystem CAS.
fn edit_source(
    group: &Value,
    nonce: &str,
    index: usize,
    j: &mut Value,
    journal: &Path,
) -> Result<()> {
    if group["storage"] == "keychain" {
        j["steps"].as_array_mut().unwrap().push(json!({"id":format!("auth_edit_{index}"),"label":group["service"],"status":"executing","message":"精确移除 Keychain 的 Claude 登录字段；其余字段保留。","source_ids":group["source_ids"],"auth_edit_phase":"intent"}));
        save(journal, j)?;
        let original = auth_sources::keychain_original(group, nonce)?;
        let replacement = auth_sources::keychain_replacement(group, &original)?;
        let updated = crate::keychain::replace(
            Path::new(string(group, "home")?),
            string(group, "service")?,
            &original,
            &replacement,
        )?;
        if updated.bytes != replacement {
            return Err(err(
                "keychain_write_uncertain",
                "Keychain 更新读回不一致；保留原任务，不重试",
            ));
        }
        let step = j["steps"].as_array_mut().unwrap().last_mut().unwrap();
        step["auth_edit_phase"] = json!("completed");
        step["status"] = json!("completed");
        step["message"] = json!("已读回精确修改；未导出凭据或创建备份。");
        return save(journal, j);
    }
    let path = Path::new(string(group, "path")?);
    let replacement = auth_sources::replacement(group, nonce)?;
    let directory = path
        .parent()
        .ok_or_else(|| err("invalid_path", "认证来源缺少父目录"))?
        .join(format!(".lintel-auth-edit-{}-{index}", string(j, "id")?));
    let held = directory.join("original");
    let staged = directory.join("replacement");
    j["steps"].as_array_mut().unwrap().push(json!({"id":format!("auth_edit_{index}"),"label":path,"status":"executing","message":"精确移除所选声明；原凭据不创建备份。","source_ids":group["source_ids"],"auth_original_path":held,"auth_staged_path":staged,"quarantine_path":directory,"auth_edit_phase":"intent"}));
    save(journal, j)?;
    guard(&directory)?;
    fs::DirBuilder::new().mode(0o700).create(&directory)?;
    let directory_identity = crate::launch::dir_identity(&directory)?;
    j["steps"].as_array_mut().unwrap().last_mut().unwrap()["auth_quarantine_identity"] =
        directory_identity.clone();
    save(journal, j)?;
    let mode = group["snapshot"]["mode"].as_u64().unwrap_or(0o600) as u32 & 0o777 & !0o022;
    atomic_new(&staged, &replacement, mode)?;
    let after = auth_sources::snapshot(&staged, nonce)?;
    let _staged_owner = AuthStaging {
        path: staged.clone(),
        directory: directory.clone(),
        directory_identity: directory_identity.clone(),
        snapshot: after.clone(),
        nonce: nonce.to_owned(),
    };
    auth_edit_hook(path, &directory, "prepared");
    // Re-read the exact source immediately before moving it out of the way.
    auth_sources::replacement(group, nonce)?;
    check_edit_directory(&directory, &directory_identity)?;
    guard(&held)?;
    // Do not replace a file that appeared at the held name after mkdir.
    publish_staged_new(path, &held)?;
    fs::File::open(&directory)?.sync_all()?;
    fs::File::open(path.parent().unwrap())?.sync_all()?;
    j["steps"].as_array_mut().unwrap().last_mut().unwrap()["auth_edit_phase"] = json!("held");
    save(journal, j)?;
    auth_edit_hook(path, &directory, "held");
    auth_sources::check_parent(group)?;
    check_edit_directory(&directory, &directory_identity)?;
    if !auth_sources::matches_snapshot(&held, &group["snapshot"], nonce)? {
        // Preserve an outside edit. Never replace a newly created live path.
        if !path.exists() {
            let _ = publish_staged_new(&held, path);
        }
        return Err(err(
            "auth_source_conflict",
            "来源在清理窗口内改变；保留原件及外部编辑，按原任务核对",
        ));
    }
    publish_staged_new(&staged, path)?;
    j["steps"].as_array_mut().unwrap().last_mut().unwrap()["auth_edit_phase"] = json!("published");
    save(journal, j)?;
    auth_edit_hook(path, &directory, "published");
    auth_sources::check_parent(group)?;
    check_edit_directory(&directory, &directory_identity)?;
    if !auth_sources::matches_snapshot(path, &after, nonce)?
        || !auth_sources::matches_snapshot(&held, &group["snapshot"], nonce)?
    {
        return Err(err(
            "auth_source_conflict",
            "发布后发现写入变化；未丢弃隔离原件，按原任务核对",
        ));
    }
    // A successful exact edit intentionally has no credential undo material.
    fs::remove_file(&held)?;
    fs::File::open(&directory)?.sync_all()?;
    fs::remove_dir(&directory)?;
    fs::File::open(path.parent().unwrap())?.sync_all()?;
    let step = j["steps"].as_array_mut().unwrap().last_mut().unwrap();
    step["auth_edit_phase"] = json!("completed");
    step["status"] = json!("completed");
    step["message"] = json!("已读回精确修改并移除隔离原件；其余字段保持。");
    save(journal, j)
}

fn check_edit_directory(path: &Path, expected: &Value) -> Result<()> {
    guard(path)?;
    if crate::launch::dir_identity(path)? != *expected {
        return Err(err(
            "auth_quarantine_changed",
            "原任务的私有暂存目录已变化；保留现有对象，不继续写入或删除",
        ));
    }
    Ok(())
}

struct AuthStaging {
    path: PathBuf,
    directory: PathBuf,
    directory_identity: Value,
    snapshot: Value,
    nonce: String,
}
impl Drop for AuthStaging {
    fn drop(&mut self) {
        // A reused pathname is not ownership. A replaced directory or staging
        // object remains visible for original-task reconciliation.
        if check_edit_directory(&self.directory, &self.directory_identity).is_ok()
            && auth_sources::matches_snapshot(&self.path, &self.snapshot, &self.nonce)
                .unwrap_or(false)
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
thread_local! { static EDIT_HOOK: std::cell::Cell<Option<fn(&Path, &Path, &str)>> = const { std::cell::Cell::new(None) }; }
#[cfg(test)]
pub(crate) fn set_edit_hook(hook: Option<fn(&Path, &Path, &str)>) {
    EDIT_HOOK.with(|slot| slot.set(hook));
}
fn auth_edit_hook(_path: &Path, _directory: &Path, _phase: &str) {
    #[cfg(test)]
    EDIT_HOOK.with(|slot| {
        if let Some(hook) = slot.get() {
            hook(_path, _directory, _phase);
        }
    });
}
