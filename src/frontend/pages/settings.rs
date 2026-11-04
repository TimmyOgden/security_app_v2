use leptos::*;
use crate::models::*;

/// Result of a save or a connection test: (ok, message).
type Outcome = Option<(bool, String)>;

/// A labelled text input that marks the page as changed on edit.
fn text_input(
    label: &'static str,
    kind: &'static str,
    value: ReadSignal<String>,
    set: WriteSignal<String>,
    placeholder: &'static str,
    dirty: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <label class="field-label">{label}
            <input type=kind class="field" placeholder=placeholder spellcheck="false"
                autocomplete=if kind == "password" { "new-password" } else { "off" }
                prop:value=move || value.get()
                on:input=move |ev| { set.set(event_target_value(&ev)); dirty.set(true); }/>
        </label>
    }
}

/// The result line next to a Test button.
fn outcome_line(pending: Signal<bool>, outcome: ReadSignal<Outcome>) -> impl IntoView {
    move || {
        if pending.get() {
            view! { <span class="set-result" role="status">"Testing"</span> }.into_view()
        } else {
            outcome.get().map(|(ok, msg)| view! {
                <span class=if ok { "set-result ok" } else { "set-result bad" } role="status">{msg}</span>
            }).into_view()
        }
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let settings = create_local_resource(|| (), |_| async { fetch_settings().await });

    let (azure_org, set_azure_org) = create_signal(String::new());
    let (azure_pat, set_azure_pat) = create_signal(String::new());
    let (azure_project, set_azure_project) = create_signal(String::new());
    let (smtp_server, set_smtp_server) = create_signal(String::new());
    let (smtp_port, set_smtp_port) = create_signal(String::new());
    let (smtp_username, set_smtp_username) = create_signal(String::new());
    let (smtp_password, set_smtp_password) = create_signal(String::new());
    let (email_from, set_email_from) = create_signal(String::new());
    let (email_to, set_email_to) = create_signal(String::new());
    let (sonar_url, set_sonar_url) = create_signal(String::new());
    let (sonar_token, set_sonar_token) = create_signal(String::new());
    let (sonar_project, set_sonar_project) = create_signal(String::new());
    let (sonar_exclusions, set_sonar_exclusions) = create_signal(String::new());
    let (sonar_qp, set_sonar_qp) = create_signal(String::new());
    let (qp_list, set_qp_list) = create_signal(Vec::<QualityProfile>::new());
    let (qp_error, set_qp_error) = create_signal(Option::<String>::None);
    let (openvas_url, set_openvas_url) = create_signal(String::new());
    let (openvas_username, set_openvas_username) = create_signal(String::new());
    let (openvas_password, set_openvas_password) = create_signal(String::new());

    let (dirty, set_dirty) = create_signal(false);
    let (save_out, set_save_out) = create_signal(Outcome::None);
    let (email_out, set_email_out) = create_signal(Outcome::None);
    let (sonar_out, set_sonar_out) = create_signal(Outcome::None);
    let (openvas_out, set_openvas_out) = create_signal(Outcome::None);

    create_effect(move |_| {
        if let Some(Ok(s)) = settings.get() {
            set_azure_org.set(s.azure_devops_org);
            set_azure_pat.set(s.azure_devops_pat);
            set_azure_project.set(s.azure_devops_project);
            set_smtp_server.set(s.smtp_server);
            set_smtp_port.set(s.smtp_port);
            set_smtp_username.set(s.smtp_username);
            set_smtp_password.set(s.smtp_password);
            set_email_from.set(s.email_from);
            set_email_to.set(s.email_to);
            set_sonar_url.set(s.sonarqube_url);
            set_sonar_token.set(s.sonarqube_token);
            set_sonar_project.set(s.sonarqube_project_key);
            set_sonar_exclusions.set(s.sonarqube_exclusions);
            set_sonar_qp.set(s.sonarqube_quality_profile);
            set_openvas_url.set(s.openvas_url);
            set_openvas_username.set(s.openvas_username);
            set_openvas_password.set(s.openvas_password);
            set_dirty.set(false);
        }
    });

    let save_action = create_action(move |_: &()| {
        let pairs = vec![
            SettingPair { key: "azure_devops_org".into(), value: azure_org.get() },
            SettingPair { key: "azure_devops_pat".into(), value: azure_pat.get() },
            SettingPair { key: "azure_devops_project".into(), value: azure_project.get() },
            SettingPair { key: "smtp_server".into(), value: smtp_server.get() },
            SettingPair { key: "smtp_port".into(), value: smtp_port.get() },
            SettingPair { key: "smtp_username".into(), value: smtp_username.get() },
            SettingPair { key: "smtp_password".into(), value: smtp_password.get() },
            SettingPair { key: "email_from".into(), value: email_from.get() },
            SettingPair { key: "email_to".into(), value: email_to.get() },
            SettingPair { key: "sonarqube_url".into(), value: sonar_url.get() },
            SettingPair { key: "sonarqube_token".into(), value: sonar_token.get() },
            SettingPair { key: "sonarqube_project_key".into(), value: sonar_project.get() },
            SettingPair { key: "sonarqube_exclusions".into(), value: sonar_exclusions.get() },
            SettingPair { key: "sonarqube_quality_profile".into(), value: sonar_qp.get() },
            SettingPair { key: "openvas_url".into(), value: openvas_url.get() },
            SettingPair { key: "openvas_username".into(), value: openvas_username.get() },
            SettingPair { key: "openvas_password".into(), value: openvas_password.get() },
        ];
        async move {
            match do_save_settings(pairs).await {
                Ok(_) => {
                    set_dirty.set(false);
                    set_save_out.set(Some((true, "Saved".into())));
                }
                Err(e) => set_save_out.set(Some((false, format!("Not saved: {}", e)))),
            }
        }
    });
    let test_email_action = create_action(move |_: &()| async move {
        set_email_out.set(Some(match do_test_email().await { Ok(m) => (true, m), Err(e) => (false, e) }));
    });
    let test_sonar_action = create_action(move |_: &()| async move {
        set_sonar_out.set(Some(match do_test_sonarqube().await { Ok(m) => (true, m), Err(e) => (false, e) }));
    });
    let test_openvas_action = create_action(move |_: &()| async move {
        set_openvas_out.set(Some(match do_test_openvas().await { Ok(m) => (true, m), Err(e) => (false, e) }));
    });
    let load_profiles_action = create_action(move |_: &()| async move {
        set_qp_error.set(None);
        match fetch_quality_profiles().await {
            Ok(profiles) => set_qp_list.set(profiles),
            Err(e) => set_qp_error.set(Some(e)),
        }
    });

    let saving = save_action.pending();
    let qp_loading = load_profiles_action.pending();
    let save_state = move || {
        if saving.get() {
            ("set-state", "Saving".to_string())
        } else if dirty.get() {
            ("set-state changed", "Unsaved changes".to_string())
        } else {
            match save_out.get() {
                Some((true, m)) => ("set-state ok", m),
                Some((false, m)) => ("set-state bad", m),
                None => ("set-state", String::new()),
            }
        }
    };
    let save_bar = move || view! {
        <span class=move || save_state().0 role="status">{move || save_state().1}</span>
        <button type="button" class="wt-btn wt-btn-solid" disabled=move || saving.get()
            on:click=move |_| save_action.dispatch(())>"Save settings"</button>
    };
    // Tests read the stored values, so say so while edits are unsaved.
    let test_hint = move || dirty.get().then(|| view! { <span class="set-hint">"Tests use the saved values."</span> });
    let profile_groups = move || {
        let profiles = qp_list.get();
        let mut langs: Vec<String> = profiles.iter().map(|p| p.language_name.clone()).collect();
        langs.sort();
        langs.dedup();
        langs.into_iter().map(|lang| {
            let in_lang: Vec<QualityProfile> = profiles.iter().filter(|p| p.language_name == lang).cloned().collect();
            (lang, in_lang)
        }).collect::<Vec<_>>()
    };

    view! {
        <section class="page-head" aria-label="Settings">
            <div class="in" style="--d: 40ms">
                <div class="eyebrow">"Settings"</div>
                <h1 class="page-title">"Integrations"</h1>
            </div>
            <div class="set-bar in" style="--d: 80ms">{save_bar}</div>
        </section>

        <section class="set-sec in" style="--d: 120ms" aria-labelledby="set-sonar">
            <div class="set-side">
                <h2 id="set-sonar">"SonarQube"</h2>
                <p>"Code quality and security analysis for folder and repository targets. It sets itself up on first use if left blank."</p>
                <div class="set-test">
                    <button type="button" class="mini-btn" on:click=move |_| test_sonar_action.dispatch(())>"Test connection"</button>
                    {outcome_line(test_sonar_action.pending().into(), sonar_out)}
                    {test_hint}
                </div>
            </div>
            <div class="set-fields">
                {text_input("Server URL", "text", sonar_url, set_sonar_url, "http://sonarqube:9000", set_dirty)}
                {text_input("Token", "password", sonar_token, set_sonar_token, "", set_dirty)}
                {text_input("Project key", "text", sonar_project, set_sonar_project, "watchtower-scan", set_dirty)}
                {text_input("Exclusions", "text", sonar_exclusions, set_sonar_exclusions, "**/node_modules/**,**/venv/**", set_dirty)}
                <div class="set-wide prof">
                    <div class="prof-head">
                        <span class="field-label">"Quality profile"</span>
                        <span class="prof-current">{move || { let q = sonar_qp.get(); if q.is_empty() { "Server default".to_string() } else { q } }}</span>
                        <button type="button" class="mini-btn" disabled=move || qp_loading.get()
                            on:click=move |_| load_profiles_action.dispatch(())>
                            {move || if qp_loading.get() { "Loading" } else if qp_list.with(|l| l.is_empty()) { "Load profiles" } else { "Reload" }}
                        </button>
                        <Show when=move || !sonar_qp.get().is_empty()>
                            <button type="button" class="mini-btn" on:click=move |_| { set_sonar_qp.set(String::new()); set_dirty.set(true); }>"Use server default"</button>
                        </Show>
                    </div>
                    {move || qp_error.get().map(|e| view! { <p class="notice error" role="status">{e}</p> })}
                    <div class="prof-list" role="radiogroup" aria-label="Quality profile">
                        {move || profile_groups().into_iter().map(|(lang, list)| view! {
                            <div class="prof-lang row-enter">
                                <div class="prof-lang-name">{lang}</div>
                                {list.into_iter().map(|p| {
                                    let name = p.name.clone();
                                    let name_click = p.name.clone();
                                    view! {
                                        <button type="button" role="radio" class="prof-row"
                                            aria-checked=move || if sonar_qp.get() == name { "true" } else { "false" }
                                            on:click=move |_| { set_sonar_qp.set(name_click.clone()); set_dirty.set(true); }>
                                            <span class="prof-mark" aria-hidden="true"></span>
                                            <span class="prof-name">{p.name.clone()}</span>
                                            {p.is_default.then(|| view! { <span class="tag">"Default"</span> })}
                                            {p.is_built_in.then(|| view! { <span class="tag">"Built-in"</span> })}
                                            <span class="prof-rules num">{format!("{} rules", p.active_rule_count)}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        }).collect_view()}
                    </div>
                </div>
            </div>
        </section>

        <section class="set-sec in" style="--d: 160ms" aria-labelledby="set-email">
            <div class="set-side">
                <h2 id="set-email">"Email"</h2>
                <p>"Sends a summary when a run finds anything critical or high. Leave the server blank to turn it off."</p>
                <div class="set-test">
                    <button type="button" class="mini-btn" on:click=move |_| test_email_action.dispatch(())>"Send test email"</button>
                    {outcome_line(test_email_action.pending().into(), email_out)}
                    {test_hint}
                </div>
            </div>
            <div class="set-fields">
                {text_input("SMTP server", "text", smtp_server, set_smtp_server, "smtp.example.com", set_dirty)}
                {text_input("Port", "text", smtp_port, set_smtp_port, "587", set_dirty)}
                {text_input("Username", "text", smtp_username, set_smtp_username, "", set_dirty)}
                {text_input("Password", "password", smtp_password, set_smtp_password, "", set_dirty)}
                {text_input("From", "email", email_from, set_email_from, "watchtower@example.com", set_dirty)}
                {text_input("To", "email", email_to, set_email_to, "security@example.com", set_dirty)}
            </div>
        </section>

        <section class="set-sec in" style="--d: 200ms" aria-labelledby="set-azure">
            <div class="set-side">
                <h2 id="set-azure">"Azure DevOps"</h2>
                <p>"Lets a run list and clone repositories from your organization."</p>
            </div>
            <div class="set-fields">
                {text_input("Organization", "text", azure_org, set_azure_org, "my-org", set_dirty)}
                {text_input("Default project", "text", azure_project, set_azure_project, "", set_dirty)}
                <div class="set-wide">{text_input("Personal access token", "password", azure_pat, set_azure_pat, "", set_dirty)}</div>
            </div>
        </section>

        <section class="set-sec in" style="--d: 240ms" aria-labelledby="set-openvas">
            <div class="set-side">
                <h2 id="set-openvas">"OpenVAS"</h2>
                <p>"Network vulnerability scanning through the Greenbone Community Edition stack. Its first start takes 10 to 20 minutes while feeds download."</p>
                <div class="set-test">
                    <button type="button" class="mini-btn" on:click=move |_| test_openvas_action.dispatch(())>"Test connection"</button>
                    {outcome_line(test_openvas_action.pending().into(), openvas_out)}
                    {test_hint}
                </div>
            </div>
            <div class="set-fields">
                <div class="set-wide">{text_input("gsad URL", "text", openvas_url, set_openvas_url, "http://gsad:80", set_dirty)}</div>
                {text_input("Username", "text", openvas_username, set_openvas_username, "admin", set_dirty)}
                {text_input("Password", "password", openvas_password, set_openvas_password, "", set_dirty)}
            </div>
        </section>

        <div class="set-foot">{save_bar}</div>
    }
}

async fn fetch_settings() -> Result<AllSettings, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/settings")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<AllSettings> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn do_save_settings(pairs: Vec<SettingPair>) -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let req = SaveSettingsRequest { settings: pairs };
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/settings")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(api.message.unwrap_or("Saved".into())) } else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = pairs; Err("SSR".into()) }
}

async fn do_test_email() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/settings/test-email")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(api.message.unwrap_or("OK".into())) }
        else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn do_test_sonarqube() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/settings/test-sonarqube")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(api.message.unwrap_or("OK".into())) }
        else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn do_test_openvas() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/settings/test-openvas")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(api.message.unwrap_or("OK".into())) }
        else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn fetch_quality_profiles() -> Result<Vec<QualityProfile>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/sonarqube/profiles")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<QualityProfile>> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success {
            api.data.ok_or("No data".into())
        } else {
            Err(api.message.unwrap_or("Failed".into()))
        }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}
