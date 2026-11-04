use leptos::*;
use crate::models::*;
use crate::frontend::components::trace::RunTrace;

fn mark_welcome_seen() {
    #[cfg(feature = "hydrate")]
    {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("watchtower-welcome-seen", "1");
        }
    }
}

/// A made-up run for the welcome trace: a few findings at fixed points.
fn sample_marks() -> Vec<TraceMark> {
    [(0.08, "low"), (0.17, "medium"), (0.29, "info"), (0.41, "high"), (0.47, "medium"),
     (0.58, "low"), (0.66, "critical"), (0.74, "medium"), (0.83, "low"), (0.91, "high")]
        .into_iter().enumerate()
        .map(|(i, (pos, sev))| TraceMark { pos, severity: sev.into(), id: i as i64, tool: String::new() })
        .collect()
}

const STEPS: [&str; 3] = ["What it does", "Email", "Ready"];

#[component]
pub fn WelcomePage() -> impl IntoView {
    let (step, set_step) = create_signal(1u8);
    let (email_from, set_email_from) = create_signal(String::new());
    let (email_to, set_email_to) = create_signal(String::new());
    let (smtp_server, set_smtp_server) = create_signal(String::new());
    let (smtp_port, set_smtp_port) = create_signal("587".to_string());
    let (smtp_username, set_smtp_username) = create_signal(String::new());
    let (smtp_password, set_smtp_password) = create_signal(String::new());
    let (save_err, set_save_err) = create_signal(Option::<String>::None);

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
                Ok(_) => { set_save_err.set(None); set_step.set(3); }
                Err(e) => set_save_err.set(Some(e)),
            }
        }
    });
    let saving = save_email.pending();

    let finish = move |dest: &'static str| {
        move |_| {
            mark_welcome_seen();
            leptos_router::use_navigate()(dest, Default::default());
        }
    };
    let input = move |label: &'static str, kind: &'static str, value: ReadSignal<String>, set: WriteSignal<String>, placeholder: &'static str| view! {
        <label class="field-label">{label}
            <input type=kind class="field" placeholder=placeholder spellcheck="false"
                autocomplete=if kind == "password" { "new-password" } else { "off" }
                prop:value=move || value.get()
                on:input=move |ev| set.set(event_target_value(&ev))/>
        </label>
    };
    let (marks, _) = create_signal(sample_marks());

    view! {
        <section class="wel" aria-label="Welcome">
            <div class="wel-trace in" style="--d: 40ms">
                <RunTrace
                    marks=marks
                    progress=Signal::derive(|| 1.0)
                    live=Signal::derive(|| false)
                    pending=Signal::derive(|| false)
                    seed=7
                    width=1000.0
                    height=72.0
                    amp=3.0
                />
            </div>
            <div class="wel-head in" style="--d: 80ms">
                <span class="eyebrow">"Welcome to Watchtower"</span>
                <ol class="wel-steps" aria-label="Setup steps">
                    {STEPS.into_iter().enumerate().map(|(i, name)| {
                        let n = i as u8 + 1;
                        view! {
                            <li class=move || match step.get() { s if s == n => "wel-step now", s if s > n => "wel-step done", _ => "wel-step" }
                                aria-current=move || (step.get() == n).then_some("step")>
                                <span class="num">{format!("{:02}", n)}</span>" "{name}
                            </li>
                        }
                    }).collect_view()}
                </ol>
            </div>

            {move || match step.get() {
                1 => view! {
                    <div class="wel-body">
                        <h1 class="page-title in" style="--d: 120ms">"One findings list from nine scanners"</h1>
                        <p class="page-lede in" style="--d: 160ms">
                            "Watchtower runs ZAP, Nikto, SQLMap and Nmap against websites, Bandit, Trivy, SonarQube and Dependency-Check against source code, and OpenVAS against networks. "
                            "Each run gives you one list of findings with severity scores, triage, and a comparison with the run before it."
                        </p>
                        <p class="page-lede in" style="--d: 200ms">
                            "SonarQube sets itself up on first use. Everything else works without configuration, so the next step is optional."
                        </p>
                        <div class="wel-actions in" style="--d: 240ms">
                            <button type="button" class="wt-btn wt-btn-solid" on:click=move |_| set_step.set(2)>"Continue"</button>
                            <button type="button" class="wt-btn wt-btn-ghost" on:click=finish("/scan")>"Skip, start a run"</button>
                        </div>
                    </div>
                }.into_view(),
                2 => view! {
                    <div class="wel-body">
                        <h1 class="page-title in" style="--d: 40ms">"Email on critical findings"</h1>
                        <p class="page-lede in" style="--d: 80ms">
                            "When a run finds anything critical or high, Watchtower can email you a summary. This is optional, and you can change it later in Settings."
                        </p>
                        <div class="wel-fields in" style="--d: 120ms">
                            {input("SMTP server", "text", smtp_server, set_smtp_server, "smtp.example.com")}
                            {input("Port", "text", smtp_port, set_smtp_port, "587")}
                            {input("Username", "text", smtp_username, set_smtp_username, "")}
                            {input("Password", "password", smtp_password, set_smtp_password, "")}
                            {input("From", "email", email_from, set_email_from, "watchtower@example.com")}
                            {input("To", "email", email_to, set_email_to, "security@example.com")}
                        </div>
                        {move || save_err.get().map(|e| view! { <p class="notice error" role="status">{format!("Not saved: {}", e)}</p> })}
                        <div class="wel-actions in" style="--d: 160ms">
                            <button type="button" class="wt-btn wt-btn-solid"
                                disabled=move || smtp_server.get().is_empty() || saving.get()
                                on:click=move |_| save_email.dispatch(())>
                                {move || if saving.get() { "Saving" } else { "Save and continue" }}
                            </button>
                            <button type="button" class="wt-btn wt-btn-ghost" on:click=move |_| set_step.set(3)>"Skip"</button>
                            <button type="button" class="mini-btn wel-back" on:click=move |_| set_step.set(1)>"Back"</button>
                        </div>
                    </div>
                }.into_view(),
                _ => view! {
                    <div class="wel-body">
                        <h1 class="page-title in" style="--d: 40ms">"Ready"</h1>
                        <p class="page-lede in" style="--d: 80ms">"Start a run now, or set one to repeat on a schedule."</p>
                        <div class="wel-actions in" style="--d: 120ms">
                            <button type="button" class="wt-btn wt-btn-solid" on:click=finish("/scan")>"Start a run"</button>
                            <button type="button" class="wt-btn wt-btn-ghost" on:click=finish("/schedules")>"Schedules"</button>
                            <button type="button" class="wt-btn wt-btn-ghost" on:click=finish("/")>"Overview"</button>
                        </div>
                    </div>
                }.into_view(),
            }}
        </section>
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
