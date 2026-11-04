use leptos::*;
use crate::models::*;
use crate::frontend::app::RESTRICTED_TOOLS;

/// The scanners, grouped by what they need as a target. ZAP and SonarQube
/// also depend on a service, so their status comes from the health check.
#[component]
pub fn ToolsPage() -> impl IntoView {
    let tools = create_local_resource(|| (), |_| async { fetch_tools().await });
    let presets = create_local_resource(|| (), |_| async { fetch_presets().await });
    let health = create_local_resource(|| (), |_| async { fetch_health().await });
    let advanced_mode = use_context::<ReadSignal<bool>>().unwrap_or_else(|| create_signal(false).0);

    let visible = move || {
        let adv = advanced_mode.get();
        tools.get().and_then(|r| r.ok()).unwrap_or_default().into_iter()
            .filter(|t| adv || !RESTRICTED_TOOLS.contains(&t.name.as_str()))
            .collect::<Vec<_>>()
    };
    // None while the health check is in flight; Some(ready) once known.
    let ready = move |name: &str| -> Option<bool> {
        let h = health.get()?;
        Some(match name {
            "zap" => h.map(|h| h.zap).unwrap_or(false),
            "sonarqube" => h.map(|h| h.sonarqube).unwrap_or(false),
            _ => true,
        })
    };
    let in_presets = move |name: &str| -> String {
        let adv = advanced_mode.get();
        presets.get().and_then(|r| r.ok()).unwrap_or_default().into_iter()
            .filter(|p| p.tools.iter().any(|t| t == name))
            .filter(|p| adv || !crate::frontend::app::RESTRICTED_PRESETS.contains(&p.name.as_str()))
            .map(|p| p.display_name)
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let title = move || {
        let list = visible();
        if list.is_empty() {
            return "Tools".to_string();
        }
        let down = list.iter().filter(|t| ready(&t.name) == Some(false)).count();
        if down == 0 {
            format!("{} tools ready", list.len())
        } else {
            format!("{} tools, {} not reachable", list.len(), down)
        }
    };
    let group = move |web: bool| {
        visible().into_iter().filter(|t| t.web_only == web).collect::<Vec<_>>()
    };

    let row = move |tool: ToolInfo| {
        let name = store_value(tool.name.clone());
        let name_presets = tool.name.clone();
        let restricted = RESTRICTED_TOOLS.contains(&tool.name.as_str());
        let service = match tool.name.as_str() {
            "zap" => Some("ZAP daemon"),
            "sonarqube" => Some("SonarQube server"),
            _ => None,
        };
        let state = move || name.with_value(|n| ready(n));
        view! {
            <div class="tool-row row-enter">
                <span class=move || match state() { Some(true) => "dot on", Some(false) => "dot down", None => "dot" } aria-hidden="true"></span>
                <span class="tool-main">
                    <span class="tool-name">
                        {tool.display_name.clone()}
                        {restricted.then(|| view! { <span class="tag">"Advanced"</span> })}
                    </span>
                    <span class="tool-desc">{tool.description.clone()}</span>
                </span>
                <span class="tool-kind">{tool.category.clone()}</span>
                <span class="tool-presets">
                    {move || { let p = in_presets(&name_presets); if p.is_empty() { "No preset".to_string() } else { p } }}
                </span>
                <span class=move || if state() == Some(false) { "tool-state bad" } else { "tool-state" }>
                    {move || match (state(), service) {
                        (None, Some(_)) => "Checking".to_string(),
                        (Some(true), Some(s)) => format!("{} reachable", s),
                        (Some(false), Some(s)) => format!("{} not reachable", s),
                        _ => "Installed".to_string(),
                    }}
                </span>
            </div>
        }
    };

    view! {
        <section aria-label="Tools">
            <div class="eyebrow in" style="--d: 40ms">"Tools"</div>
            <h1 class="page-title in" style="--d: 80ms">{title}</h1>
            <p class="page-lede in" style="--d: 100ms">
                "A run uses the tools its preset names. Tools that need a URL are skipped on a folder or repository, and the code tools are skipped on a URL."
            </p>
        </section>

        <Suspense fallback=|| ()>
            {move || [(true, "Web and network", "Need a URL"), (false, "Source code", "Need a folder or repository")]
                .into_iter().enumerate().map(|(i, (web, heading, note))| view! {
                    <section class="tool-group in" style=format!("--d: {}ms", 140 + i * 60) aria-label=heading>
                        <div class="section-head"><h2>{heading}</h2><span class="section-note">{note}</span></div>
                        <div class="tool-cols" aria-hidden="true">
                            <span></span><span>"Tool"</span><span>"Kind"</span><span>"In presets"</span><span>"Status"</span>
                        </div>
                        <For each=move || group(web) key=|t| t.name.clone() children=row />
                    </section>
                }).collect_view()}
        </Suspense>
    }
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
