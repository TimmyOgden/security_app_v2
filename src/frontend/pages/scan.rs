use leptos::*;
use leptos_router::*;
use crate::models::*;
use crate::frontend::app::{RESTRICTED_TOOLS, RESTRICTED_PRESETS};

#[component]
pub fn ScanPage() -> impl IntoView {
    let (scan_type, set_scan_type) = create_signal("full".to_string());
    let (target, set_target) = create_signal(String::new());
    let (target_source, set_target_source) = create_signal("url".to_string());
    let (selected_tools, set_selected_tools) = create_signal(Vec::<String>::new());
    let (is_scanning, set_is_scanning) = create_signal(false);
    let (browse_path, set_browse_path) = create_signal(None::<String>);
    let (folder_search, set_folder_search) = create_signal(String::new());

    // Read advanced mode from context — hides restricted tools when false
    let advanced_mode = use_context::<ReadSignal<bool>>().unwrap_or_else(|| create_signal(false).0);

    let drives = create_local_resource(|| (), |_| async { fetch_drives().await });
    let presets = create_local_resource(|| (), |_| async { fetch_presets().await });
    let tools = create_local_resource(|| (), |_| async { fetch_tools().await });

    let folders = create_local_resource(
        move || browse_path.get(),
        |path| async move { fetch_folders(path).await },
    );

    let start_scan = create_action(move |_: &()| {
        let st = scan_type.get();
        let tgt = target.get();
        let ts = target_source.get();
        let tls = selected_tools.get();
        async move {
            set_is_scanning.set(true);
            let req = StartScanRequest {
                scan_type: st,
                target: tgt,
                target_source: ts,
                tools: if tls.is_empty() { None } else { Some(tls) },
            };
            match do_start_scan(req).await {
                Ok(scan_id) => {
                    let navigate = leptos_router::use_navigate();
                    navigate(&format!("/runs/{}", scan_id), Default::default());
                }
                Err(e) => {
                    #[cfg(feature = "hydrate")]
                    web_sys::console::error_1(&format!("Scan failed: {}", e).into());
                    #[cfg(not(feature = "hydrate"))]
                    { let _ = e; }
                    set_is_scanning.set(false);
                }
            }
        }
    });

    let preset_items = move || {
        let adv = advanced_mode.get();
        presets.get().and_then(|r| r.ok()).unwrap_or_default().into_iter()
            .filter(|p| adv || !RESTRICTED_PRESETS.contains(&p.name.as_str()))
            .collect::<Vec<_>>()
    };
    let tool_items = move || {
        let adv = advanced_mode.get();
        tools.get().and_then(|r| r.ok()).unwrap_or_default().into_iter()
            .filter(|t| adv || !RESTRICTED_TOOLS.contains(&t.name.as_str()))
            .collect::<Vec<_>>()
    };
    let fits = move |web_only: bool| (target_source.get() == "url") == web_only;
    let summary = move || {
        let preset = presets.get().and_then(|r| r.ok()).unwrap_or_default().into_iter()
            .find(|p| p.name == scan_type.get());
        let chosen = selected_tools.get();
        let planned: Vec<String> = if chosen.is_empty() {
            preset.as_ref().map(|p| p.tools.clone()).unwrap_or_default()
        } else {
            chosen
        };
        let adv = advanced_mode.get();
        let planned: Vec<String> = planned.into_iter().filter(|t| adv || !RESTRICTED_TOOLS.contains(&t.as_str())).collect();
        let runnable = planned.iter().filter(|t| fits(tool_is_web_only(t))).count();
        let skipped = planned.len() - runnable;
        let name = preset.map(|p| p.display_name).unwrap_or_else(|| "Custom".into());
        let tgt = target.get();
        if tgt.is_empty() {
            "Choose a target to see what will run.".to_string()
        } else {
            format!("{} on {}: {} tool{} will run{}.", name, tgt, runnable, if runnable == 1 { "" } else { "s" },
                if skipped > 0 { format!(", {} skipped for this target", skipped) } else { String::new() })
        }
    };
    let sources: [(&'static str, &'static str); 3] = [("url", "URL"), ("local", "Local folder"), ("azure", "Azure DevOps")];

    view! {
        <section aria-label="New run">
            <div class="eyebrow in" style="--d: 40ms"><A class="eyebrow-link" href="/runs">"Runs"</A>" / New run"</div>
            <h1 class="page-title in" style="--d: 80ms">"Start a run"</h1>
            <p class="page-lede in" style="--d: 100ms">"Pick what to scan and how. Tools that cannot work on the chosen target are skipped."</p>
        </section>

        <ol class="steps">
            <li class="step in" style="--d: 140ms">
                <div class="step-head"><span class="step-n">"1"</span><h2>"Target"</h2></div>
                <div class="seg" role="radiogroup" aria-label="Target source">
                    {sources.into_iter().map(|(key, label)| view! {
                        <button type="button" role="radio" class="seg-btn"
                            aria-checked=move || if target_source.get() == key { "true" } else { "false" }
                            on:click=move |_| set_target_source.set(key.to_string())>{label}</button>
                    }).collect_view()}
                </div>
                <label class="field-label step-field">
                    {move || if target_source.get() == "url" { "URL" } else { "Path" }}
                    <input type="text" class="field"
                        placeholder=move || if target_source.get() == "url" { "https://example.com" } else { "/projects/my-app" }
                        prop:value=move || target.get()
                        on:input=move |ev| set_target.set(event_target_value(&ev))/>
                </label>
                <Show when=move || target_source.get() == "local">
                    <div class="fb-bar">
                        <button type="button" class="mini-btn" on:click=move |_| {
                            set_folder_search.set(String::new());
                            set_browse_path.set(Some("/projects".into()));
                        }>"Browse projects"</button>
                        {move || drives.get().and_then(|r| r.ok()).filter(|d| !d.is_empty()).map(|drive_list| view! {
                            <select class="field-sm" aria-label="Browse a drive" on:change=move |ev| {
                                let val = event_target_value(&ev);
                                if !val.is_empty() {
                                    set_folder_search.set(String::new());
                                    set_browse_path.set(Some(val));
                                }
                            }>
                                <option value="">"Browse a drive"</option>
                                {drive_list.into_iter().map(|d| view! { <option value={d.path.clone()}>{format!("{}:", d.letter)}</option> }).collect_view()}
                            </select>
                        })}
                    </div>
                    <div class=move || if browse_path.get().is_some() { "expand open" } else { "expand" }>
                        <div>
                            <div class="fb">
                                {move || folders.get().map(|data| match data {
                                    Ok(resp) => {
                                        let current = resp.current_path.clone();
                                        let current_for_select = resp.current_path.clone();
                                        let parent = resp.parent.clone();
                                        let search = folder_search.get().to_lowercase();
                                        let entries: Vec<_> = resp.entries.iter()
                                            .filter(|e| search.is_empty() || e.name.to_lowercase().contains(&search))
                                            .cloned().collect();
                                        let empty = entries.is_empty();
                                        view! {
                                            <div class="fb-head">
                                                <span class="fb-path">{current}</span>
                                                {parent.map(|p| view! {
                                                    <button type="button" class="mini-btn" on:click=move |_| {
                                                        set_folder_search.set(String::new());
                                                        set_browse_path.set(Some(p.clone()));
                                                    }>"Up"</button>
                                                })}
                                                <button type="button" class="mini-btn" on:click=move |_| {
                                                    set_target.set(current_for_select.clone());
                                                    set_browse_path.set(None);
                                                }>"Use this folder"</button>
                                                <button type="button" class="mini-btn" aria-label="Close the folder browser" on:click=move |_| set_browse_path.set(None)>"Close"</button>
                                            </div>
                                            <input type="search" class="field-sm fb-search" placeholder="Filter this folder" aria-label="Filter this folder"
                                                prop:value=move || folder_search.get()
                                                on:input=move |ev| set_folder_search.set(event_target_value(&ev))/>
                                            <div class="fb-list">
                                                {empty.then(|| view! { <p class="fb-empty">"Nothing matches."</p> })}
                                                {entries.into_iter().map(|entry| {
                                                    let path = entry.path.clone();
                                                    let is_dir = entry.is_dir;
                                                    view! {
                                                        <button type="button" class="fb-row" on:click=move |_| {
                                                            if is_dir {
                                                                set_folder_search.set(String::new());
                                                                set_browse_path.set(Some(path.clone()));
                                                            } else {
                                                                set_target.set(path.clone());
                                                                set_browse_path.set(None);
                                                            }
                                                        }>
                                                            {if is_dir {
                                                                view! { <svg class="fb-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 6.5h6l2 2h10v10H3z"></path></svg> }
                                                            } else {
                                                                view! { <svg class="fb-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3h8l4 4v14H6z"></path><path d="M14 3v4h4"></path></svg> }
                                                            }}
                                                            <span class="fb-name">{entry.name.clone()}</span>
                                                            <span class="fb-kind">{if is_dir { "Open" } else { "Use file" }}</span>
                                                        </button>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        }.into_view()
                                    }
                                    Err(_) => view! { <p class="fb-empty">"This folder could not be read."</p> }.into_view(),
                                })}
                            </div>
                        </div>
                    </div>
                </Show>
            </li>

            <li class="step in" style="--d: 180ms">
                <div class="step-head"><span class="step-n">"2"</span><h2>"Preset"</h2></div>
                <div class="preset-tiles">
                    <For
                        each=preset_items
                        key=|p| p.name.clone()
                        children=move |preset| {
                            let name = preset.name.clone();
                            let name_click = preset.name.clone();
                            let tool_tags = preset.tools.clone();
                            view! {
                                <button type="button" class="tile"
                                    aria-pressed=move || if scan_type.get() == name { "true" } else { "false" }
                                    on:click=move |_| set_scan_type.set(name_click.clone())>
                                    <span class="tile-name">{preset.display_name.clone()}</span>
                                    <span class="tile-desc">{preset.description.clone()}</span>
                                    <span class="tile-tags">
                                        {tool_tags.into_iter()
                                            .filter(move |t| advanced_mode.get_untracked() || !RESTRICTED_TOOLS.contains(&t.as_str()))
                                            .map(|t| {
                                                let web = tool_is_web_only(&t);
                                                view! { <span class=move || if fits(web) { "ttag" } else { "ttag skipped" }>{t.clone()}</span> }
                                            }).collect_view()}
                                    </span>
                                </button>
                            }
                        }
                    />
                </div>
            </li>

            <li class="step in" style="--d: 220ms">
                <div class="step-head"><span class="step-n">"3"</span><h2>"Tools"</h2><span class="section-note">"Optional. Leave all unticked to use the preset's tools."</span></div>
                <p class="notice">
                    {move || if target_source.get() == "url" {
                        "A URL target skips the code tools (Bandit, SonarQube, Trivy, Dependency-Check): they need source code."
                    } else {
                        "A folder target skips the web and network tools (ZAP, Nikto, SQLMap, Nmap, OpenVAS): they need a URL."
                    }}
                </p>
                <div class="tool-checks">
                    <For
                        each=tool_items
                        key=|t| t.name.clone()
                        children=move |tool| {
                            let name = tool.name.clone();
                            let name_change = tool.name.clone();
                            let web_only = tool.web_only;
                            let restricted = RESTRICTED_TOOLS.contains(&tool.name.as_str());
                            view! {
                                <label class=move || if fits(web_only) { "tool-check" } else { "tool-check off" }>
                                    <input type="checkbox"
                                        prop:checked=move || selected_tools.get().contains(&name)
                                        disabled=move || !fits(web_only)
                                        on:change=move |ev| {
                                            let checked = event_target_checked(&ev);
                                            set_selected_tools.update(|v| {
                                                if checked { v.push(name_change.clone()); } else { v.retain(|x| x != &name_change); }
                                            });
                                        }/>
                                    <span class="tool-check-name">{tool.display_name.clone()}</span>
                                    <span class="tool-check-meta">
                                        {tool.category.clone()}
                                        {restricted.then(|| " · advanced")}
                                        {move || (!fits(web_only)).then(|| if web_only { " · needs a URL" } else { " · needs source code" })}
                                    </span>
                                </label>
                            }
                        }
                    />
                </div>
            </li>

            <li class="step in" style="--d: 260ms">
                <div class="step-head"><span class="step-n">"4"</span><h2>"Start"</h2></div>
                <p class="step-summary" aria-live="polite">{summary}</p>
                <button type="button" class="wt-btn wt-btn-solid"
                    disabled=move || is_scanning.get() || target.get().is_empty()
                    on:click=move |_| start_scan.dispatch(())>
                    {move || if is_scanning.get() { "Starting..." } else if advanced_mode.get() { "Execute Order 66" } else { "Start run" }}
                </button>
            </li>
        </ol>
    }
}

fn tool_is_web_only(name: &str) -> bool {
    matches!(name, "zap" | "nmap" | "nikto" | "sqlmap" | "openvas")
}

fn event_target_checked(ev: &leptos::ev::Event) -> bool {
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|e: web_sys::HtmlInputElement| e.checked())
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = ev;
        false
    }
}

async fn fetch_presets() -> Result<Vec<ScanPreset>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/presets")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ScanPreset>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn fetch_tools() -> Result<Vec<ToolInfo>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/tools")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ToolInfo>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn do_start_scan(req: StartScanRequest) -> Result<i64, String> {
    #[cfg(feature = "hydrate")]
    {
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/scans/start")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        api.data
            .and_then(|d| d["scan_id"].as_i64())
            .ok_or("No scan_id returned".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = req; Err("SSR".into()) }
}

async fn fetch_folders(path: Option<String>) -> Result<BrowseFoldersResponse, String> {
    #[cfg(not(feature = "hydrate"))]
    { let _ = path; }
    #[cfg(feature = "hydrate")]
    {
        let req = BrowseFoldersRequest { path };
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/browse-folders")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        resp.json::<BrowseFoldersResponse>().await.map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn fetch_drives() -> Result<Vec<DriveInfo>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/drives")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<DriveInfo>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}
