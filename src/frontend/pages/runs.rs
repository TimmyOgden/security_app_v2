use leptos::*;
use leptos_router::*;
use crate::models::*;
use super::super::components::fmt;
use super::super::components::trace::RunTrace;

fn bucket(status: &str) -> &'static str {
    match status {
        "running" | "pending" => "live",
        "completed" => "done",
        "failed" => "failed",
        "stopped" => "stopped",
        _ => "done",
    }
}

/// Every run, newest first, each with its trace. Replaces Scan History and
/// the old Reports page (PDFs download from the row).
#[component]
pub fn RunsPage() -> impl IntoView {
    let (tick, set_tick) = create_signal(0u32);
    let resource = create_local_resource(move || tick.get(), |_| async move { fetch_runs().await });
    let (rows, set_rows) = create_signal(Vec::<RunRow>::new());
    let (loaded, set_loaded) = create_signal(false);
    let (filter, set_filter) = create_signal("all");

    create_effect(move |_| {
        if let Some(Ok(r)) = resource.get() {
            set_rows.set(r);
            set_loaded.set(true);
        }
    });

    #[cfg(feature = "hydrate")]
    {
        create_effect(move |_| {
            let refresh = gloo_timers::callback::Interval::new(5_000, move || set_tick.update(|t| *t += 1));
            on_cleanup(move || drop(refresh));
        });
    }
    let refetch = move || set_tick.update(|t| *t += 1);

    let total = create_memo(move |_| rows.with(|r| r.len()));
    let live_count = create_memo(move |_| rows.with(|r| r.iter().filter(|x| bucket(&x.status) == "live").count()));
    let count = move |k: &'static str| rows.with(|r| r.iter().filter(|x| k == "all" || bucket(&x.status) == k).count());
    let shown = move || {
        let f = filter.get();
        rows.get().into_iter().filter(|r| f == "all" || bucket(&r.status) == f).collect::<Vec<_>>()
    };

    let nothing_shown = move || loaded.get() && shown().is_empty();
    let still_loading = move || !loaded.get();
    let tabs: [(&'static str, &'static str); 5] = [("all", "All"), ("live", "Live"), ("done", "Done"), ("failed", "Failed"), ("stopped", "Stopped")];

    view! {
        <section aria-label="Runs">
            <div class="runs-head">
                <div class="in" style="--d: 40ms">
                    <div class="eyebrow">"Runs"</div>
                    <h1 class="runs-title">
                        {move || view! { <span class="roll num">{total.get()}</span> }}
                        {move || if total.get() == 1 { " run, " } else { " runs, " }}
                        {move || view! { <span class="roll num">{live_count.get()}</span> }}
                        " live"
                    </h1>
                </div>
                <A class="wt-btn wt-btn-solid in" href="/scan">"New run"</A>
            </div>

            <div class="tabs in" style="--d: 100ms" role="tablist" aria-label="Filter runs">
                {tabs.into_iter().map(|(key, label)| view! {
                    <button
                        type="button"
                        role="tab"
                        class="tab"
                        aria-selected=move || if filter.get() == key { "true" } else { "false" }
                        on:click=move |_| set_filter.set(key)
                    >
                        {label}" "<span class="num">{move || count(key)}</span>
                    </button>
                }).collect_view()}
            </div>
        </section>

        <section class="in" style="--d: 140ms" aria-label="Run list">
            <div class="runs-scroll">
                <div class="runs-grid">
                    <div class="run-head" aria-hidden="true">
                        <span>"Run"</span><span>"State"</span><span>"Target"</span><span>"Trace"</span>
                        <span>"Crit · High · Other"</span><span>"Score"</span><span>"Started"</span><span></span>
                    </div>
                    <For
                        each=shown
                        key=|r| r.id
                        children=move |r| {
                            let id = r.id;
                            let row = create_memo(move |_| rows.with(|v| v.iter().find(|x| x.id == id).cloned()));
                            run_row(r, row, refetch)
                        }
                    />
                    <Show when=nothing_shown>
                        <p class="empty-line">"No runs in this state."</p>
                    </Show>
                    <Show when=still_loading>
                        <p class="empty-line">"Loading runs..."</p>
                    </Show>
                </div>
            </div>
        </section>
    }
}

fn run_row(first: RunRow, row: Memo<Option<RunRow>>, refetch: impl Fn() + Copy + 'static) -> impl IntoView {
    let id = first.id;
    let status = move || row.with(|r| r.as_ref().map(|r| r.status.clone()).unwrap_or_default());
    let state_class = move || format!("run-state {}", bucket(&status()));
    let state_label = move || match status().as_str() {
        "running" => "Live",
        "pending" => "Queued",
        "completed" => "Done",
        "failed" => "Failed",
        "stopped" => "Stopped",
        _ => "Unknown",
    };
    let marks = Signal::derive(move || row.with(|r| r.as_ref().map(|r| r.marks.clone()).unwrap_or_default()));
    let progress = Signal::derive(move || row.with(|r| r.as_ref().map(|r| r.progress).unwrap_or(0.0)));
    let live = Signal::derive(move || status() == "running");
    let pending = Signal::derive(move || status() == "pending");
    let num = move |f: fn(&RunRow) -> i64| row.with(|r| r.as_ref().map(f).unwrap_or(0));
    let when = move || row.with(|r| r.as_ref().map(|r| match r.status.as_str() {
        "running" | "pending" => format!("{} so far", fmt::clock(r.elapsed_seconds)),
        "stopped" => format!("stopped at {}", fmt::clock(r.elapsed_seconds)),
        "failed" => format!("failed at {}", fmt::clock(r.elapsed_seconds)),
        _ => fmt::clock(r.duration_seconds.unwrap_or(r.elapsed_seconds)),
    }).unwrap_or_default());

    view! {
        <div class="run-row row-enter">
            <A class="run-link" href=format!("/runs/{}", id)>{format!("#{}", id)}</A>
            <span class=state_class>
                {move || if status() == "running" { view! { <span class="dot live"></span> }.into_view() } else { view! { <span class="sq"></span> }.into_view() }}
                {state_label}
            </span>
            <span class="run-target">
                <b>{first.target.clone()}</b>
                <small>{first.scan_type.clone()}</small>
            </span>
            <RunTrace
                marks=marks
                progress=progress
                live=live
                pending=pending
                seed=id
                width=160.0
                height=32.0
            />
            <span class="run-counts">
                {move || { let n = num(|r| r.critical); view! { <span class={if n > 0 { "c-critical" } else { "c-dim" }}>{n}</span> } }}
                {move || { let n = num(|r| r.high); view! { <span class={if n > 0 { "c-high" } else { "c-dim" }}>{n}</span> } }}
                <span>{move || num(|r| r.other)}</span>
            </span>
            <span class="num">
                {move || row.with(|r| r.as_ref().and_then(|r| r.grade.clone()).unwrap_or_else(|| "\u{2014}".into()))}
                <small class="c-dim">{move || row.with(|r| r.as_ref().and_then(|r| r.score).map(|s| format!(" {}", s)).unwrap_or_default())}</small>
            </span>
            <span class="run-when">
                {fmt::short_date(&first.started_at)}
                <small>{when}</small>
            </span>
            <span class="run-actions">
                {move || match status().as_str() {
                    "running" => view! {
                        <button type="button" class="mini-btn" title="Stop this run" on:click=move |_| act(id, "stop", refetch)>"Stop"</button>
                    }.into_view(),
                    "stopped" => view! {
                        <button type="button" class="mini-btn" title="Resume this run" on:click=move |_| act(id, "resume", refetch)>"Resume"</button>
                    }.into_view(),
                    "completed" => view! {
                        <a class="mini-btn" href=format!("/api/reports/{}/download-pdf", id) target="_blank" rel="noopener">"PDF"</a>
                    }.into_view(),
                    _ => ().into_view(),
                }}
                <button type="button" class="mini-btn" title="Delete this run and its findings" aria-label=format!("Delete run {}", id) on:click=move |_| delete(id, refetch)>"Delete"</button>
            </span>
        </div>
    }
}

#[allow(unused_variables)]
fn act(id: i64, action: &'static str, refetch: impl Fn() + 'static) {
    #[cfg(feature = "hydrate")]
    wasm_bindgen_futures::spawn_local(async move {
        let _ = gloo_net::http::Request::post(&format!("/api/scans/{}/{}", id, action)).send().await;
        refetch();
    });
}

#[allow(unused_variables)]
fn delete(id: i64, refetch: impl Fn() + 'static) {
    #[cfg(feature = "hydrate")]
    {
        let ok = web_sys::window()
            .and_then(|w| w.confirm_with_message(&format!("Delete run #{} and all its findings?", id)).ok())
            .unwrap_or(false);
        if ok {
            act(id, "cancel", refetch);
        }
    }
}

async fn fetch_runs() -> Result<Vec<RunRow>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/runs")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<RunRow>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Err("SSR".into())
    }
}
