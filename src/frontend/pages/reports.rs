use leptos::*;
use leptos_router::*;
use crate::models::*;
use crate::frontend::components::fmt;

/// Every generated report, newest first, each linked to its run.
#[component]
pub fn ReportsPage() -> impl IntoView {
    let reports = create_local_resource(|| (), |_| async { fetch_reports().await });
    let list = move || {
        let mut l = reports.get().and_then(|r| r.ok()).unwrap_or_default();
        l.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        l
    };
    let loaded = move || reports.get().is_some();
    let empty = move || loaded() && list().is_empty();
    let title = move || match list().len() {
        0 => "Reports".to_string(),
        1 => "1 report".to_string(),
        n => format!("{} reports", n),
    };

    view! {
        <section aria-label="Reports">
            <div class="eyebrow in" style="--d: 40ms"><A class="eyebrow-link" href="/runs">"Runs"</A>" / Reports"</div>
            <h1 class="page-title in" style="--d: 80ms">{title}</h1>
            <p class="page-lede in" style="--d: 100ms">"Reports are made from a run's page with Export PDF."</p>
        </section>

        <section class="in" style="--d: 140ms" aria-label="Report list">
            <div class="rep-cols" aria-hidden="true">
                <span>"Run"</span><span>"Format"</span><span>"Made"</span><span>"Emailed"</span><span></span>
            </div>
            <For
                each=list
                key=|r| r.id
                children=|r| {
                    let run = r.scan_job_id;
                    let pdf = r.format == "pdf";
                    view! {
                        <div class="rep-row row-enter">
                            <a class="rep-run" href=format!("/runs/{}", run)>
                                <span class="num">{format!("Run {}", run)}</span>
                                <span class="rep-file">{r.file_path.rsplit(['/', '\\']).next().unwrap_or_default().to_string()}</span>
                            </a>
                            <span class="tag">{r.format.to_uppercase()}</span>
                            <span class="num c-dim">{fmt::short_date(&r.created_at)}</span>
                            <span class="c-dim">{if r.emailed { "Emailed" } else { "No" }}</span>
                            <span class="rep-act">
                                {pdf.then(|| view! {
                                    <a class="mini-btn" href=format!("/api/reports/{}/download-pdf", run) target="_blank" rel="noopener"
                                        aria-label=format!("Download the PDF for run {}", run)>"Download"</a>
                                })}
                            </span>
                        </div>
                    }
                }
            />
            <Show when=empty>
                <p class="empty-line">"No reports yet. Open a finished run and use Export PDF."</p>
            </Show>
        </section>
    }
}

async fn fetch_reports() -> Result<Vec<Report>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/reports")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<Report>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}
