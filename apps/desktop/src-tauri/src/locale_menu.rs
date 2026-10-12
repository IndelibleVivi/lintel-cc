//! The ordinary Tauri menu, with finite App language labels and native actions.

fn chinese(language: &str) -> Result<bool, String> {
    match language {
        "zh-CN" => Ok(true),
        "en" => Ok(false),
        _ => Err("unsupported_app_language".into()),
    }
}

pub fn initial_language() -> &'static str {
    for language in super::locale::preferred_languages() {
        let language = language.to_ascii_lowercase().replace('_', "-");
        if language == "zh" || language.starts_with("zh-") {
            return "zh-CN";
        }
        if language == "en" || language.starts_with("en-") {
            return "en";
        }
    }
    "en"
}

pub fn set_language(app: &tauri::AppHandle, language: &str) -> Result<(), String> {
    chinese(language)?;
    #[cfg(target_os = "macos")]
    app.set_menu(build(app, language).map_err(|_| "app_menu_unavailable")?)
        .map_err(|_| "app_menu_unavailable")?;
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    Ok(())
}

pub fn build(
    app: &tauri::AppHandle,
    language: &str,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = language;
        tauri::menu::Menu::default(app)
    }
    #[cfg(target_os = "macos")]
    {
        use tauri::menu::{
            AboutMetadata, Menu, PredefinedMenuItem as Item, Submenu, HELP_SUBMENU_ID,
            WINDOW_SUBMENU_ID,
        };
        let zh = language == "zh-CN";
        let text = |chinese, english| if zh { chinese } else { english };
        let about = AboutMetadata {
            name: Some("Lintel".into()),
            version: Some(app.package_info().version.to_string()),
            copyright: app.config().bundle.copyright.clone(),
            authors: app
                .config()
                .bundle
                .publisher
                .clone()
                .map(|publisher| vec![publisher]),
            ..Default::default()
        };
        Menu::with_items(
            app,
            &[
                &Submenu::with_items(
                    app,
                    "Lintel",
                    true,
                    &[
                        &Item::about(app, Some(text("关于 Lintel", "About Lintel")), Some(about))?,
                        &Item::separator(app)?,
                        &Item::services(app, Some(text("服务", "Services")))?,
                        &Item::separator(app)?,
                        &Item::hide(app, Some(text("隐藏 Lintel", "Hide Lintel")))?,
                        &Item::hide_others(app, Some(text("隐藏其他", "Hide Others")))?,
                        &Item::separator(app)?,
                        &Item::quit(app, Some(text("退出 Lintel", "Quit Lintel")))?,
                    ],
                )?,
                &Submenu::with_items(
                    app,
                    text("文件", "File"),
                    true,
                    &[&Item::close_window(
                        app,
                        Some(text("关闭窗口", "Close Window")),
                    )?],
                )?,
                &Submenu::with_items(
                    app,
                    text("编辑", "Edit"),
                    true,
                    &[
                        &Item::undo(app, Some(text("撤销", "Undo")))?,
                        &Item::redo(app, Some(text("重做", "Redo")))?,
                        &Item::separator(app)?,
                        &Item::cut(app, Some(text("剪切", "Cut")))?,
                        &Item::copy(app, Some(text("复制", "Copy")))?,
                        &Item::paste(app, Some(text("粘贴", "Paste")))?,
                        &Item::select_all(app, Some(text("全选", "Select All")))?,
                    ],
                )?,
                &Submenu::with_items(
                    app,
                    text("视图", "View"),
                    true,
                    &[&Item::fullscreen(
                        app,
                        Some(text("切换全屏", "Toggle Full Screen")),
                    )?],
                )?,
                &Submenu::with_id_and_items(
                    app,
                    WINDOW_SUBMENU_ID,
                    text("窗口", "Window"),
                    true,
                    &[
                        &Item::minimize(app, Some(text("最小化", "Minimize")))?,
                        &Item::maximize(app, Some(text("缩放", "Zoom")))?,
                        &Item::separator(app)?,
                        &Item::close_window(app, Some(text("关闭窗口", "Close Window")))?,
                    ],
                )?,
                &Submenu::with_id_and_items(app, HELP_SUBMENU_ID, text("帮助", "Help"), true, &[])?,
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn menu_accepts_only_supported_app_languages() {
        assert_eq!(super::chinese("zh-CN"), Ok(true));
        assert_eq!(super::chinese("en"), Ok(false));
        for invalid in ["", "en-US", "zh-TW", "fr", "../en"] {
            assert_eq!(
                super::chinese(invalid),
                Err("unsupported_app_language".into())
            );
        }
    }
}
