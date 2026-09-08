use leptos::*;
use crate::models::*;

#[component]
pub fn SchedulesPage() -> impl IntoView {
    let (refresh_counter, set_refresh_counter) = create_signal(0u32);
    let schedules = create_local_resource(move || refresh_counter.get(), |_| async { fetch_schedules().await });
    let presets = create_local_resource(|| (), |_| async { fetch_presets().await });

    let (scan_type, set_scan_type) = create_signal("full".to_string());
    let (target, set_target) = create_signal(String::new());
    let (target_source, set_target_source) = create_signal("url".to_string());
    let (interval_hours, set_interval_hours) = create_signal(24i64);
    let (create_msg, set_create_msg) = create_signal(Option::<String>::None);

    let create_action = create_action(move |_: &()| {
        let req = CreateScheduleRequest {
            scan_type: scan_type.get(),
            target: target.get(),
            target_source: target_source.get(),
            tools: None,
            interval_hours: interval_hours.get(),
        };
        async move {
            match do_create_schedule(req).await {
                Ok(_) => {
                    set_create_msg.set(Some("Schedule created".into()));
                    set_target.set(String::new());
                    set_refresh_counter.update(|v| *v += 1);
                }
                Err(e) => set_create_msg.set(Some(format!("Error: {}", e))),
            }
        }
    });

    view! {
        <div class="page-header">
            <h1>"⏱ Schedules"</h1>
            <p class="page-subtitle">"Recurring scans, run automatically on an interval"</p>
        </div>

        <div class="card mb-3">
            <h3>"New Schedule"</h3>

            <div class="form-group">
                <label>"Target Source"</label>
                <div class="radio-group">
                    <label class="radio-label">
                        <input type="radio" name="sched-source" value="url"
                            checked=move || target_source.get() == "url"
                            on:change=move |_| set_target_source.set("url".into())/>
                        " URL"
                    </label>
                    <label class="radio-label">
                        <input type="radio" name="sched-source" value="local"
                            checked=move || target_source.get() == "local"
                            on:change=move |_| set_target_source.set("local".into())/>
                        " Local Folder"
                    </label>
                </div>
            </div>

            <div class="form-row">
                <div class="form-group">
                    <label>"Target"</label>
                    <input type="text" class="form-control"
                        placeholder=move || if target_source.get() == "url" { "https://example.com" } else { "/projects/my-app" }
                        prop:value=move || target.get()
                        on:input=move |ev| set_target.set(event_target_value(&ev))/>
                </div>
                <div class="form-group">
                    <label>"Repeat every (hours)"</label>
                    <input type="number" class="form-control" min="1"
                        prop:value=move || interval_hours.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<i64>() {
                                set_interval_hours.set(v.max(1));
                            }
                        }/>
                </div>
            </div>

            <div class="form-group">
                <label>"Scan Preset"</label>
                <Suspense fallback=move || view! { <p>"Loading presets..."</p> }>
                    {move || presets.get().map(|data| match data {
                        Ok(preset_list) => view! {
                            <select class="form-control"
                                on:change=move |ev| set_scan_type.set(event_target_value(&ev))>
                                {preset_list.into_iter().map(|p| view! {
                                    <option value={p.name.clone()} selected=move || scan_type.get() == p.name>{p.display_name.clone()}</option>
                                }).collect_view()}
                            </select>
                        }.into_view(),
                        Err(_) => view! { <p>"Loading..."</p> }.into_view(),
                    })}
                </Suspense>
            </div>

            {move || create_msg.get().map(|msg| view! { <div class="alert alert-info">{msg}</div> })}

            <button class="btn btn-primary"
                disabled=move || target.get().is_empty()
                on:click=move |_| create_action.dispatch(())>
                "＋ Create Schedule"
            </button>
        </div>

        <div class="card">
            <h3>"Active Schedules"</h3>
            <Suspense fallback=move || view! { <div class="loading">"Loading schedules..."</div> }>
                {move || schedules.get().map(|data| match data {
                    Ok(list) if list.is_empty() => view! {
                        <div class="empty-state"><p>"No schedules yet. Create one above to run scans automatically."</p></div>
                    }.into_view(),
                    Ok(list) => view! {
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"Target"</th>
                                    <th>"Preset"</th>
                                    <th>"Every"</th>
                                    <th>"Last Run"</th>
                                    <th>"Next Run"</th>
                                    <th>"Status"</th>
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || list.clone()
                                    key=|s| s.id
                                    children=move |s: ScheduledScan| {
                                        #[allow(unused_variables)]
                                        let sid = s.id;
                                        view! {
                                            <tr>
                                                <td class="target-cell">{s.target.clone()}</td>
                                                <td><span class="badge badge-type">{s.scan_type.clone()}</span></td>
                                                <td>{format!("{}h", s.interval_hours)}</td>
                                                <td>{s.last_run_at.clone().unwrap_or("never".into())}</td>
                                                <td>{s.next_run_at.clone()}</td>
                                                <td>
                                                    <span class=if s.enabled { "badge badge-status-completed" } else { "badge badge-category" }>
                                                        {if s.enabled { "enabled" } else { "paused" }}
                                                    </span>
                                                </td>
                                                <td class="scan-actions-cell">
                                                    <button class="scan-action-btn scan-resume-btn"
                                                        on:click=move |_| {
                                                            #[cfg(feature = "hydrate")]
                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                let _ = do_toggle_schedule(sid).await;
                                                                set_refresh_counter.update(|v| *v += 1);
                                                            });
                                                        }>
                                                        {if s.enabled { "⏸ Pause" } else { "▶ Resume" }}
                                                    </button>
                                                    <button class="scan-action-btn scan-cancel-btn"
                                                        on:click=move |_| {
                                                            #[cfg(feature = "hydrate")]
                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                let _ = do_delete_schedule(sid).await;
                                                                set_refresh_counter.update(|v| *v += 1);
                                                            });
                                                        }>"✕ Delete"</button>
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </table>
                    }.into_view(),
                    Err(_) => view! { <div class="loading">"Loading..."</div> }.into_view(),
                })}
            </Suspense>
        </div>
    }
}

async fn fetch_schedules() -> Result<Vec<ScheduledScan>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/schedules")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ScheduledScan>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
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

async fn do_create_schedule(req: CreateScheduleRequest) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/schedules")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(()) } else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = req; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn do_toggle_schedule(id: i64) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        gloo_net::http::Request::post(&format!("/api/schedules/{}/toggle", id))
            .send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn do_delete_schedule(id: i64) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        gloo_net::http::Request::delete(&format!("/api/schedules/{}", id))
            .send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}
