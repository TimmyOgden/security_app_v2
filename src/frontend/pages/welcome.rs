use leptos::*;
use crate::models::*;

fn mark_welcome_seen() {
    #[cfg(feature = "hydrate")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("watchtower-welcome-seen", "1");
        }
    }
}

#[component]
pub fn WelcomePage() -> impl IntoView {
    let (step, set_step) = create_signal(1u8);
    let (email_from, set_email_from) = create_signal(String::new());
    let (email_to, set_email_to) = create_signal(String::new());
    let (smtp_server, set_smtp_server) = create_signal(String::new());
    let (smtp_port, set_smtp_port) = create_signal("587".to_string());
    let (smtp_username, set_smtp_username) = create_signal(String::new());
    let (smtp_password, set_smtp_password) = create_signal(String::new());
    let (save_msg, set_save_msg) = create_signal(Option::<String>::None);

    let save_email = create_action(move |_: &()| {
        let pairs = vec![
            SettingPair { key: "smtp_server".into(), value: smtp_server.get() },
            SettingPair { key: "smtp_port".into(), value: smtp_port.get() },
            SettingPair { key: "smtp_username".into(), value: smtp_username.get() },
            SettingPair { key: "smtp_password".into(), value: smtp_password.get() },
            SettingPair { key: "email_from".into(), value: email_from.get() },
            SettingPair { key: "email_to".into(), value: email_to.get() },
        ];
        async move {
            match do_save_settings(pairs).await {
                Ok(_) => { set_save_msg.set(Some("Saved".into())); set_step.set(3); }
                Err(e) => set_save_msg.set(Some(format!("Error: {}", e))),
            }
        }
    });

    let finish = move |dest: &'static str| {
        move |_| {
            mark_welcome_seen();
            leptos_router::use_navigate()(dest, Default::default());
        }
    };

    view! {
        <div class="page-header">
            <h1>"🗼 Welcome to Watchtower"</h1>
            <p class="page-subtitle">"A quick 30-second setup, or skip straight to scanning"</p>
        </div>

        <div class="card" style="max-width:640px">
            {move || match step.get() {
                1 => view! {
                    <div>
                        <h3>"What this does"</h3>
                        <p style="margin-top:10px; color:var(--text-secondary); line-height:1.6;">
                            "Watchtower runs nine security scanners — "
                            <b>"ZAP, Nikto, SQLMap, Nmap"</b>" for websites, "
                            <b>"Bandit, Trivy, SonarQube, Dependency-Check"</b>" for source code, and "
                            "OpenVAS for network scans — against a target you choose, then gives you one "
                            "unified findings list with severity scoring, triage, and scan-to-scan diffing."
                        </p>
                        <p style="margin-top:14px; color:var(--text-secondary); line-height:1.6;">
                            "SonarQube sets itself up automatically on first use. Everything else works "
                            "with zero configuration — the next step is optional."
                        </p>
                        <div style="display:flex; gap:10px; margin-top:20px;">
                            <button class="btn btn-primary" on:click=move |_| set_step.set(2)>"Continue →"</button>
                            <button class="btn btn-secondary" on:click=finish("/scan")>"Skip, take me to New Scan"</button>
                        </div>
                    </div>
                }.into_view(),
                2 => view! {
                    <div>
                        <h3>"Get notified on critical findings (optional)"</h3>
                        <p style="margin:8px 0 16px; color:var(--text-secondary);">
                            "If a scan finds anything critical or high severity, Watchtower can email you a summary. "
                            "Leave this blank and skip it — you can always set it up later in Settings."
                        </p>
                        <div class="form-row">
                            <div class="form-group">
                                <label>"SMTP Server"</label>
                                <input type="text" class="form-control" prop:value=move || smtp_server.get()
                                    on:input=move |ev| set_smtp_server.set(event_target_value(&ev))/>
                            </div>
                            <div class="form-group">
                                <label>"Port"</label>
                                <input type="text" class="form-control" prop:value=move || smtp_port.get()
                                    on:input=move |ev| set_smtp_port.set(event_target_value(&ev))/>
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>"Username"</label>
                                <input type="text" class="form-control" prop:value=move || smtp_username.get()
                                    on:input=move |ev| set_smtp_username.set(event_target_value(&ev))/>
                            </div>
                            <div class="form-group">
                                <label>"Password"</label>
                                <input type="password" class="form-control" prop:value=move || smtp_password.get()
                                    on:input=move |ev| set_smtp_password.set(event_target_value(&ev))/>
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>"From"</label>
                                <input type="email" class="form-control" prop:value=move || email_from.get()
                                    on:input=move |ev| set_email_from.set(event_target_value(&ev))/>
                            </div>
                            <div class="form-group">
                                <label>"To"</label>
                                <input type="email" class="form-control" prop:value=move || email_to.get()
                                    on:input=move |ev| set_email_to.set(event_target_value(&ev))/>
                            </div>
                        </div>
                        {move || save_msg.get().map(|msg| view! { <div class="alert alert-info">{msg}</div> })}
                        <div style="display:flex; gap:10px; margin-top:10px;">
                            <button class="btn btn-primary"
                                disabled=move || smtp_server.get().is_empty()
                                on:click=move |_| save_email.dispatch(())>"Save & Continue →"</button>
                            <button class="btn btn-secondary" on:click=move |_| set_step.set(3)>"Skip"</button>
                        </div>
                    </div>
                }.into_view(),
                _ => view! {
                    <div>
                        <h3>"You're set"</h3>
                        <p style="margin:8px 0 20px; color:var(--text-secondary);">
                            "Head to New Scan to run your first scan, or Schedules to set up a recurring one."
                        </p>
                        <div style="display:flex; gap:10px;">
                            <button class="btn btn-primary" on:click=finish("/scan")>"🔍 New Scan"</button>
                            <button class="btn btn-secondary" on:click=finish("/schedules")>"⏱ Schedules"</button>
                            <button class="btn btn-secondary" on:click=finish("/")>"Dashboard"</button>
                        </div>
                    </div>
                }.into_view(),
            }}
        </div>
    }
}

async fn do_save_settings(pairs: Vec<SettingPair>) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let req = SaveSettingsRequest { settings: pairs };
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/settings")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(()) } else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = pairs; Err("SSR".into()) }
}
