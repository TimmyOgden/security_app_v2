use leptos::*;
use leptos_router::*;
#[cfg(feature = "hydrate")]
use crate::models::{ApiResponse, ScanJob};
use crate::models::HealthStatus;

#[allow(dead_code)]
async fn fetch_health() -> Option<HealthStatus> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/health").send().await.ok()?;
        let api: ApiResponse<HealthStatus> = resp.json().await.ok()?;
        api.data
    }
    #[cfg(not(feature = "hydrate"))]
    { None }
}

/// Ids of runs that are running or queued.
#[allow(dead_code)]
async fn fetch_live() -> Option<Vec<i64>> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/scans").send().await.ok()?;
        let api: ApiResponse<Vec<ScanJob>> = resp.json().await.ok()?;
        Some(api.data?.into_iter().filter(|s| s.status == "running" || s.status == "pending").map(|s| s.id).collect())
    }
    #[cfg(not(feature = "hydrate"))]
    { None }
}

#[allow(dead_code)]
fn read_stored_theme() -> bool {
    #[cfg(feature = "hydrate")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("watchtower-theme").ok().flatten())
            .map(|v| v == "light")
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    { false }
}

fn apply_theme(light: bool) {
    #[cfg(feature = "hydrate")]
    {
        if let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) {
            use wasm_bindgen::JsCast;
            let class_list = body.unchecked_ref::<web_sys::Element>().class_list();
            if light {
                let _ = class_list.add_1("light-mode");
            } else {
                let _ = class_list.remove_1("light-mode");
            }
        }
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("watchtower-theme", if light { "light" } else { "dark" });
        }
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = light; }
}

/// The app's only chrome: wordmark, primary nav, live runs, service health
/// and the theme toggle. Replaces the old sidebar and bottom status bar.
#[component]
pub fn TopBar() -> impl IntoView {
    #[allow(unused_variables)]
    let (health, set_health) = create_signal(Option::<HealthStatus>::None);
    #[allow(unused_variables)]
    let (live, set_live) = create_signal(Vec::<i64>::new());
    let (light_mode, set_light_mode) = create_signal(false);

    #[cfg(feature = "hydrate")]
    {
        create_effect(move |_| {
            let stored = read_stored_theme();
            set_light_mode.set(stored);
            apply_theme(stored);
        });
        create_effect(move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                set_health.set(fetch_health().await);
            });
            let health_poll = gloo_timers::callback::Interval::new(30_000, move || {
                wasm_bindgen_futures::spawn_local(async move {
                    set_health.set(fetch_health().await);
                });
            });
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(ids) = fetch_live().await { set_live.set(ids); }
            });
            let live_poll = gloo_timers::callback::Interval::new(5_000, move || {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(ids) = fetch_live().await { set_live.set(ids); }
                });
            });
            on_cleanup(move || {
                drop(health_poll);
                drop(live_poll);
            });
        });
    }

    view! {
        <header class="topbar">
            <div class="topbar-inner">
                <a class="brand" href="/">
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M9 21V9l3-5 3 5v12"></path>
                        <path d="M6.5 21h11"></path>
                        <path d="M17.5 7.5a5 5 0 0 1 0 5"></path>
                        <path d="M20 5a9 9 0 0 1 0 10"></path>
                    </svg>
                    <span>"Watchtower"</span>
                </a>
                <nav class="topnav" aria-label="Primary">
                    <A href="/" exact=true>"Overview"</A>
                    <A href="/runs">"Runs"</A>
                    <A href="/schedules">"Schedules"</A>
                    <A href="/tools">"Tools"</A>
                    <A href="/settings">"Settings"</A>
                </nav>
                <div class="topbar-status">
                    {move || {
                        let ids = live.get();
                        let n = ids.len();
                        let href = if n == 1 { format!("/runs/{}", ids[0]) } else { "/runs".to_string() };
                        view! {
                            <a class="status-item" href=href>
                                <span class={if n > 0 { "dot live" } else { "dot" }}></span>
                                {format!("{} live", n)}
                            </a>
                        }
                    }}
                    {move || {
                        let h = health.get();
                        let zap = h.as_ref().map(|h| h.zap).unwrap_or(false);
                        let sonar = h.as_ref().map(|h| h.sonarqube).unwrap_or(false);
                        view! {
                            <span class="status-item" title=if zap { "ZAP reachable" } else { "ZAP unreachable" }>
                                <span class=if zap { "dot on" } else { "dot" }></span>
                                {if zap { "ZAP" } else { "ZAP down" }}
                            </span>
                            <span class="status-item" title=if sonar { "SonarQube reachable" } else { "SonarQube unreachable" }>
                                <span class=if sonar { "dot on" } else { "dot" }></span>
                                {if sonar { "SonarQube" } else { "SonarQube down" }}
                            </span>
                        }
                    }}
                    <button
                        class="theme-btn"
                        title="Toggle light and dark theme"
                        aria-label="Toggle light and dark theme"
                        on:click=move |_| {
                            let next = !light_mode.get();
                            set_light_mode.set(next);
                            apply_theme(next);
                        }
                    >
                        {move || if light_mode.get() {
                            view! {
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                                    <circle cx="12" cy="12" r="4"></circle>
                                    <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"></path>
                                </svg>
                            }
                        } else {
                            view! {
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                                    <path d="M21 12.5A8.5 8.5 0 1 1 11.5 3 7 7 0 0 0 21 12.5Z"></path>
                                </svg>
                            }
                        }}
                    </button>
                </div>
            </div>
        </header>
    }
}
