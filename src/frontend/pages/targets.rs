use leptos::*;
use leptos_router::*;
use crate::models::*;
use super::super::components::fmt;
use super::super::components::trace::RunTrace;

/// What is being scanned, rather than the scans: every target with its trend,
/// and for the selected one its score over time, every run as a trace, what
/// is open now and what was fixed in the last 30 days.
#[component]
pub fn TargetsPage() -> impl IntoView {
    let query = use_query_map();
    let navigate = use_navigate();
    let (tick, set_tick) = create_signal(0u32);
    let list_res = create_local_resource(move || tick.get(), |_| async move { fetch_targets().await });
    let (targets, set_targets) = create_signal(Vec::<TargetSummary>::new());
    let (loaded, set_loaded) = create_signal(false);
    #[allow(unused_variables)]
    let (now, set_now) = create_signal(0f64);
    create_effect(move |_| {
        if let Some(Ok(t)) = list_res.get() {
            set_targets.set(t);
            set_loaded.set(true);
        }
    });

    #[cfg(feature = "hydrate")]
    create_effect(move |_| {
        set_now.set(fmt::now_ms());
        let refresh = gloo_timers::callback::Interval::new(10_000, move || set_tick.update(|t| *t += 1));
        on_cleanup(move || drop(refresh));
    });
    let _ = set_tick;

    // The selected target comes from ?t=, else the first in the list.
    let selected = create_memo(move |_| {
        query.with(|q| q.get("t").cloned())
            .filter(|t| targets.with(|ts| ts.iter().any(|x| &x.target == t)))
            .or_else(|| targets.with(|ts| ts.first().map(|t| t.target.clone())))
    });
    let detail_res = create_local_resource(move || (selected.get(), tick.get()), |(t, _)| async move {
        match t {
            Some(t) => fetch_detail(&t).await.map(Some),
            None => Ok(None),
        }
    });
    let (detail, set_detail) = create_signal(Option::<TargetDetail>::None);
    create_effect(move |_| {
        if let Some(Ok(d)) = detail_res.get() {
            set_detail.set(d);
        }
    });

    let trend = |t: &TargetSummary| -> Option<i64> { Some(t.score? - t.previous_score?) };
    let headline = move || targets.with(|ts| {
        let better = ts.iter().filter(|t| trend(t).map(|d| d > 0).unwrap_or(false)).count();
        let worse = ts.iter().filter(|t| trend(t).map(|d| d < 0).unwrap_or(false)).count();
        let mut s = format!("{} target{}.", ts.len(), if ts.len() == 1 { "" } else { "s" });
        match (better, worse) {
            (0, 0) => {}
            (b, 0) => s.push_str(&format!(" {} getting better.", b)),
            (0, w) => s.push_str(&format!(" {} getting worse.", w)),
            (b, w) => s.push_str(&format!(" {} getting better, {} getting worse.", b, w)),
        }
        s
    });
    let pick = move |target: String| {
        navigate(&format!("/targets?t={}", fmt::encode(&target)), NavigateOptions { replace: true, scroll: false, ..Default::default() });
    };
    let list_items = move || targets.get();
    // Rebuild the detail only when something a person would notice changed,
    // so the 10 s refresh does not replay its entrance.
    let shape = create_memo(move |_| detail.with(|d| d.as_ref().map(|d| (
        d.summary.target.clone(), d.runs.len(), d.open.len(), d.fixed.len(), d.summary.latest_status.clone(),
    ))));
    let empty = move || loaded.get() && targets.with(|t| t.is_empty());

    view! {
        <section aria-label="Targets">
            <div class="eyebrow in" style="--d: 40ms">"Targets · what you scan, not the scans"</div>
            <h1 class="page-title in" style="--d: 80ms">{headline}</h1>
        </section>

        <Show when=empty>
            <p class="empty-line">"No targets yet. A target appears here after its first run."</p>
        </Show>

        <div class="tg-cols">
            <nav class="tg-list in" style="--d: 120ms" aria-label="Targets">
                <For
                    each=list_items
                    key=|t| (t.target.clone(), t.score, t.latest_status.clone())
                    children=move |t| {
                        let name = t.target.clone();
                        let name_for_click = t.target.clone();
                        let pick = pick.clone();
                        let delta = trend(&t);
                        let live = matches!(t.latest_status.as_str(), "running" | "pending");
                        let meta = format!("{} · {}", fmt::target_kind(&t.target_source),
                            if live { "run live".to_string() } else { t.schedule.clone().unwrap_or_else(|| "manual".into()) });
                        view! {
                            <button type="button" class="tg-item"
                                aria-current=move || if selected.get().as_deref() == Some(name.as_str()) { "true" } else { "false" }
                                on:click=move |_| pick(name_for_click.clone())>
                                <span class="frow-main">
                                    <span class="trow-name">{t.target.clone()}</span>
                                    <span class="trow-meta">{meta}</span>
                                </span>
                                <span class="tg-score">
                                    <span class=match delta { Some(d) if d < 0 => "tg-trend worse", _ => "tg-trend" }>
                                        {match delta { Some(d) if d > 0 => format!("\u{2191}{}", d), Some(d) if d < 0 => format!("\u{2193}{}", -d), Some(_) => "=".into(), None => String::new() }}
                                    </span>
                                    <span class="tg-grade">{if live { "\u{2026}".to_string() } else { t.grade.clone().unwrap_or_else(|| "\u{2014}".into()) }}</span>
                                </span>
                            </button>
                        }
                    }
                />
            </nav>

            <section class="tg-detail" aria-label="Selected target" aria-live="polite">
                {move || {
                    let _ = shape.get();
                    detail.get_untracked().map(|d| target_detail(d, now))
                }}
            </section>
        </div>
    }
}

/// The selected target. Rebuilt whenever the selection changes, so its
/// sections rise in again and the score line redraws.
fn target_detail(d: TargetDetail, now: ReadSignal<f64>) -> impl IntoView {
    let t = d.summary.clone();
    let scored: Vec<&TargetRun> = d.runs.iter().filter(|r| r.score.is_some()).collect();
    let summary = match (scored.first().and_then(|r| r.score), scored.last().and_then(|r| r.score)) {
        (Some(a), Some(b)) if scored.len() > 1 => format!("Score {} to {} over {} runs. {} open, {} fixed in the last 30 days.", a, b, scored.len(), d.open.len(), d.fixed.len()),
        (Some(b), _) => format!("Score {} from its only completed run. {} open.", b, d.open.len()),
        _ => "No completed run yet.".to_string(),
    };
    let kind = format!("{} · {}", fmt::target_kind(&t.target_source), t.schedule.clone().unwrap_or_else(|| "manual".into()));

    // Score chart: runs evenly spaced, completed runs joined by the line.
    let n = d.runs.len().max(1);
    let x_of = |i: usize| if n == 1 { 50.0 } else { 6.0 + i as f64 * 88.0 / (n - 1) as f64 };
    let y_of = |score: i64| 10.0 + (100 - score) as f64 / 100.0 * 160.0;
    let line: String = d.runs.iter().enumerate()
        .filter_map(|(i, r)| r.score.map(|s| (i, s)))
        .enumerate()
        .map(|(k, (i, s))| format!("{}{:.1} {:.1}", if k == 0 { "M" } else { "L" }, x_of(i) * 10.0, y_of(s)))
        .collect();
    let points: Vec<(f64, f64, bool, String)> = d.runs.iter().enumerate().map(|(i, r)| {
        let label = format!("#{} · {}", r.run.id, r.score.map(|s| s.to_string()).unwrap_or_else(|| r.run.status.clone()));
        match r.score {
            Some(s) => (x_of(i), y_of(s), i == n - 1, label),
            None => (x_of(i), y_of(0), false, label),
        }
    }).collect();
    let bands = [(90, "A+"), (80, "A"), (70, "B"), (60, "C"), (50, "D"), (25, "E")];
    let score_line = t.score.map(|s| format!("{} · {}", t.grade.clone().unwrap_or_default(), s)).unwrap_or_else(|| "no score".into());
    let runs_newest: Vec<TargetRun> = d.runs.iter().rev().cloned().collect();
    let fixed_empty = d.fixed.is_empty();
    let open_empty = d.open.is_empty();

    view! {
        <div class="in" style="--d: 0ms">
            <div class="eyebrow">{kind}</div>
            <h2 class="tg-name">{t.target.clone()}</h2>
            <p class="page-lede">{summary}</p>
        </div>

        <div class="section-head side-head in" style="--d: 60ms">
            <h2>"Score over time"</h2>
            <span class="num c-dim">{score_line}</span>
        </div>
        <div class="chart in" style="--d: 80ms">
            <div class="chart-bands" aria-hidden="true">
                {bands.into_iter().map(|(v, g)| view! {
                    <div class="chart-band" style=format!("top: {:.2}%", y_of(v) / 180.0 * 100.0)><span>{g}</span></div>
                }).collect_view()}
            </div>
            <svg class="chart-svg" viewBox="0 0 1000 180" preserveAspectRatio="none" role="img"
                aria-label=format!("Score by run for {}", t.target)>
                <path class="score-line" d=if line.is_empty() { "M0 0".to_string() } else { line } pathLength="1000"></path>
            </svg>
            {points.into_iter().enumerate().map(|(i, (x, y, last, label))| view! {
                <span class=if last { "chart-dot last" } else { "chart-dot" } style=format!("left: {:.2}%; top: {:.1}px; --d: {}ms", x, y, 120 + i * 60)></span>
                <span class="chart-label" style=format!("left: {:.2}%; --d: {}ms", x, 120 + i * 60)>{label}</span>
            }).collect_view()}
        </div>

        <div class="section-head side-head in" style="--d: 120ms">
            <h2>"Runs"</h2>
            <span class="section-note">"Newest on top. Height is severity."</span>
        </div>
        {runs_newest.into_iter().enumerate().map(|(i, r)| {
            let id = r.run.id;
            let label = format!("#{} · {}", id, fmt::short_date(&r.run.started_at));
            let live = matches!(r.run.status.as_str(), "running" | "pending");
            let pending = r.run.status == "pending";
            let progress = r.run.progress;
            let marks = r.run.marks.clone();
            let grade = r.grade.clone().unwrap_or_else(|| if live { "LIVE".into() } else { r.run.status.to_uppercase() });
            let score = r.score.map(|s| format!(" {}", s)).unwrap_or_default();
            view! {
                <a class="trun in" href=format!("/runs/{}", id) style=format!("--d: {}ms", 160 + i * 40)>
                    <span class="trun-label">{label}</span>
                    <RunTrace
                        marks=Signal::derive(move || marks.clone())
                        progress=Signal::derive(move || progress)
                        live=Signal::derive(move || live)
                        pending=Signal::derive(move || pending)
                        seed=id
                        width=600.0
                        height=30.0
                    />
                    <span class="trun-grade">{grade}<small>{score}</small></span>
                </a>
            }
        }).collect_view()}

        <div class="tg-two">
            <div class="in" style="--d: 200ms">
                <div class="section-head side-head">
                    <h2>"Open"</h2>
                    <span class="num c-dim">{d.open.len()}</span>
                </div>
                {d.open.into_iter().map(|a| {
                    let first_seen = a.first_seen.clone();
                    view! {
                        <a class="frow" href=format!("/runs/{}", a.scan_id)>
                            <span class=format!("sevmark {}", a.severity)><i></i>{fmt::sev_label(&a.severity)}</span>
                            <span class="frow-main">
                                <span class="frow-title">{a.title.clone()}</span>
                                <span class="frow-meta">{a.tool.clone()}</span>
                            </span>
                            <span class="frow-side">
                                {a.confirmed.then(|| view! { <span class="tag critical">"Confirmed"</span> })}
                                {a.new.then(|| view! { <span class="tag">"New"</span> })}
                                <span>{move || fmt::open_for(first_seen.as_deref(), now.get())}</span>
                            </span>
                        </a>
                    }
                }).collect_view()}
                {open_empty.then(|| view! { <p class="empty-line">"Nothing open."</p> })}
            </div>
            <div class="in" style="--d: 240ms">
                <div class="section-head side-head">
                    <h2>"Fixed in 30 days"</h2>
                    <span class="num c-dim">{d.fixed.len()}</span>
                </div>
                {d.fixed.into_iter().map(|f| view! {
                    <div class="fixed-row">
                        <span class="fixed-sev">{fmt::sev_label(&f.severity)}</span>
                        <span class="frow-main">
                            <span class="fixed-title">{f.title.clone()}</span>
                            <span class="frow-meta">{format!("{} · gone in run #{} · {}", f.tool, f.run_id, fmt::short_date(&f.fixed_at))}</span>
                        </span>
                    </div>
                }).collect_view()}
                {fixed_empty.then(|| view! { <p class="empty-line">"Nothing fixed in the last 30 days."</p> })}
            </div>
        </div>
    }
}

async fn fetch_targets() -> Result<Vec<TargetSummary>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/targets").send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<TargetSummary>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn fetch_detail(target: &str) -> Result<TargetDetail, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/targets/detail?target={}", fmt::encode(target)))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<TargetDetail> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("Not found".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = target; Err("SSR".into()) }
}
