use leptos::*;
use leptos_router::*;
use crate::models::*;
use super::super::components::fmt;
use super::super::components::trace::RunTrace;

/// What needs attention now: open critical/high findings, runs in progress
/// drawn as live traces, every target's recent runs, and the next schedules.
///
/// Data refreshes every 5 s into signals, and lists are keyed, so a poll only
/// touches what changed: rows enter once, counts roll only when they change.
#[component]
pub fn OverviewPage() -> impl IntoView {
    let (tick, set_tick) = create_signal(0u32);
    let resource = create_local_resource(move || tick.get(), |_| async move { fetch_overview().await });
    let (data, set_data) = create_signal(Option::<OverviewData>::None);
    #[allow(unused_variables)]
    let (now, set_now) = create_signal(0f64);

    create_effect(move |_| {
        if let Some(Ok(d)) = resource.get() {
            set_data.set(Some(d));
        }
    });

    #[cfg(feature = "hydrate")]
    {
        create_effect(move |_| {
            set_now.set(fmt::now_ms());
            let refresh = gloo_timers::callback::Interval::new(5_000, move || set_tick.update(|t| *t += 1));
            let clock = gloo_timers::callback::Interval::new(1_000, move || set_now.set(fmt::now_ms()));
            on_cleanup(move || {
                drop(refresh);
                drop(clock);
            });
        });

        // First run: nothing scanned yet and the welcome flow never seen.
        create_effect(move |_| {
            let empty = data.with(|d| d.as_ref().map(|d| d.targets.is_empty()).unwrap_or(false));
            if empty {
                let seen = web_sys::window()
                    .and_then(|w| w.local_storage().ok().flatten())
                    .and_then(|s| s.get_item("watchtower-welcome-seen").ok().flatten())
                    .is_some();
                if !seen {
                    use_navigate()("/welcome", Default::default());
                }
            }
        });
    }
    let _ = set_tick;

    let loaded = create_memo(move |_| data.with(|d| d.is_some()));
    let crit = create_memo(move |_| data.with(|d| d.as_ref().map(|d| d.critical_open).unwrap_or(0)));
    let crit_targets = create_memo(move |_| data.with(|d| d.as_ref().map(|d| d.critical_targets).unwrap_or(0)));
    let high = create_memo(move |_| data.with(|d| d.as_ref().map(|d| d.high_open).unwrap_or(0)));
    let live_runs = create_memo(move |_| data.with(|d| d.as_ref().map(|d| d.live.clone()).unwrap_or_default()));
    let attention = create_memo(move |_| data.with(|d| d.as_ref().map(|d| d.attention.clone()).unwrap_or_default()));
    let targets = create_memo(move |_| data.with(|d| d.as_ref().map(|d| d.targets.clone()).unwrap_or_default()));
    let next_runs = create_memo(move |_| data.with(|d| d.as_ref().map(|d| d.next_runs.clone()).unwrap_or_default()));

    let live_sentence = move || match live_runs.with(|l| l.len()) {
        0 => "no runs live".to_string(),
        1 => "1 run live".to_string(),
        n => format!("{} runs live", n),
    };
    let next_sentence = move || {
        next_runs.with(|n| n.first().map(|n| fmt::countdown(fmt::utc_ms(&n.next_run_at), now.get())))
            .filter(|s| !s.is_empty())
            .map(|s| format!(" · next scheduled run {}", s))
            .unwrap_or_default()
    };

    let live_items = move || attention.get().into_iter().filter(|a| a.live).collect::<Vec<_>>();
    let open_items = move || attention.get().into_iter().filter(|a| !a.live).collect::<Vec<_>>();
    let any_live = move || attention.with(|a| a.iter().any(|x| x.live));
    let any_open = move || attention.with(|a| a.iter().any(|x| !x.live));
    let none_open = move || loaded.get() && attention.with(|a| a.is_empty());
    let target_items = move || targets.get().into_iter().enumerate().collect::<Vec<_>>();
    let no_schedules = move || loaded.get() && next_runs.with(|n| n.is_empty());

    view! {
        <section aria-label="Status">
            <div class="eyebrow in" style="--d: 40ms">
                {move || format!("Overview · {} targets", targets.with(|t| t.len()))}
            </div>
            <Show
                when=move || loaded.get()
                fallback=|| view! { <h1 class="ov-headline">"Loading the latest runs..."</h1> }
            >
                <h1 class="ov-headline in" style="--d: 80ms">
                    {move || if crit.get() == 0 {
                        view! { "No critical findings are open." }.into_view()
                    } else {
                        view! {
                            <span class="roll num c-critical">{crit.get()}</span>
                            {if crit.get() == 1 { " critical finding is open, on " } else { " critical findings are open, on " }}
                            <span class="roll num">{crit_targets.get()}</span>
                            {if crit_targets.get() == 1 { " target." } else { " targets." }}
                        }.into_view()
                    }}
                </h1>
                <p class="ov-sub in" style="--d: 120ms">
                    {move || view! { <span class="roll num c-high">{high.get()}</span> }}
                    " high · "
                    {live_sentence}
                    {next_sentence}
                </p>
            </Show>
        </section>

        <For
            each=move || live_runs.get()
            key=|r| r.id
            children=move |r| {
                let id = r.id;
                let run = create_memo(move |_| live_runs.with(|v| v.iter().find(|x| x.id == id).cloned()));
                let marks = Signal::derive(move || run.with(|r| r.as_ref().map(|r| r.marks.clone()).unwrap_or_default()));
                let progress = Signal::derive(move || run.with(|r| r.as_ref().map(|r| r.progress).unwrap_or(0.0)));
                let meta = move || run.with(|r| r.as_ref().map(|r| {
                    let tool = r.current_tool.clone().unwrap_or_else(|| "queued".into());
                    let n = r.critical + r.high + r.other;
                    format!("{} · {} · {} finding{}", tool, fmt::clock(r.elapsed_seconds), n, if n == 1 { "" } else { "s" })
                }).unwrap_or_default());
                view! {
                    <a class="ov-live in" style="--d: 160ms" href=format!("/runs/{}", id)>
                        <div class="ov-live-head">
                            <span class="ov-live-state"><span class="dot live"></span>"Live"</span>
                            <span class="ov-live-target num"><span class="c-dim">{format!("#{} ", id)}</span>{r.target.clone()}</span>
                            <span class="ov-live-meta num">{meta}</span>
                            <span class="ov-live-open">"Open run →"</span>
                        </div>
                        <RunTrace
                            marks=marks
                            progress=progress
                            live=Signal::derive(|| true)
                            pending=Signal::derive(|| false)
                            seed=id
                            width=1000.0
                            height=80.0
                            amp=3.0
                            label=format!("Run {} so far", id)
                        />
                    </a>
                }
            }
        />

        <div class="ov-cols">
            <section class="ov-main in" style="--d: 200ms" aria-label="Needs attention">
                <div class="section-head">
                    <h2>"Needs attention"</h2>
                    <span class="section-note">"Open critical and high findings, latest run per target"</span>
                </div>
                <Show when=any_live>
                    <div class="subhead">"From runs in progress"</div>
                    <For
                        each=live_items
                        key=|a| (a.id, a.severity.clone())
                        children=move |a| attention_row(a, now)
                    />
                </Show>
                <Show when=any_open>
                    <div class="subhead">"Still open"</div>
                    <For
                        each=open_items
                        key=|a| (a.id, a.severity.clone())
                        children=move |a| attention_row(a, now)
                    />
                </Show>
                <Show when=none_open>
                    <p class="empty-line">"Nothing critical or high is open."</p>
                </Show>
            </section>

            <aside class="ov-side in" style="--d: 260ms" aria-label="Targets and schedule">
                <div class="section-head">
                    <h2>"Targets"</h2>
                    <span class="section-note">"Findings per run · score"</span>
                </div>
                <For
                    each=target_items
                    key=|(_, t)| t.target.clone()
                    children=move |(index, t)| {
                        let name = t.target.clone();
                        let row = create_memo(move |_| targets.with(|v| v.iter().find(|x| x.target == name).cloned()));
                        target_row(index, row)
                    }
                />

                <div class="section-head" style="margin-top: 36px">
                    <h2>"Next up"</h2>
                    <A class="section-link" href="/schedules">"All schedules →"</A>
                </div>
                <For
                    each=move || next_runs.get()
                    key=|n| (n.target.clone(), n.next_run_at.clone())
                    children=move |n| {
                        let at = fmt::utc_ms(&n.next_run_at);
                        let every = match n.interval_hours {
                            24 => "daily".to_string(),
                            168 => "weekly".to_string(),
                            h if h % 24 == 0 => format!("every {} days", h / 24),
                            h => format!("every {}h", h),
                        };
                        view! {
                            <div class="next-row">
                                <span class="frow-main">
                                    <span class="trow-name">{n.target.clone()}</span>
                                    <span class="trow-meta">{format!("{} · {}", n.scan_type, every)}</span>
                                </span>
                                <span class="num">{move || fmt::countdown(at, now.get())}</span>
                            </div>
                        }
                    }
                />
                <Show when=no_schedules>
                    <p class="empty-line">"No schedules yet."</p>
                </Show>
            </aside>
        </div>
    }
}

fn attention_row(a: AttentionItem, now: ReadSignal<f64>) -> impl IntoView {
    let first_seen = a.first_seen.clone();
    view! {
        <a class="frow row-enter" href=format!("/runs/{}", a.scan_id)>
            <span class=format!("sevmark {}", a.severity)><i></i>{fmt::sev_label(&a.severity)}</span>
            <span class="frow-main">
                <span class="frow-title">{a.title.clone()}</span>
                <span class="frow-meta">{format!("{} · {}", a.target, a.tool)}</span>
            </span>
            <span class="frow-side">
                {a.confirmed.then(|| view! { <span class="tag critical">"Confirmed"</span> })}
                {a.new.then(|| view! { <span class="tag">"New"</span> })}
                <span>{move || fmt::open_for(first_seen.as_deref(), now.get())}</span>
            </span>
        </a>
    }
}

fn target_row(index: usize, row: Memo<Option<TargetSummary>>) -> impl IntoView {
    let href = move || row.with(|t| t.as_ref().map(|t| format!("/runs/{}", t.latest_scan_id)).unwrap_or_else(|| "/runs".into()));
    let live = move || row.with(|t| t.as_ref().map(|t| t.latest_status == "running" || t.latest_status == "pending").unwrap_or(false));
    let meta = move || row.with(|t| t.as_ref().map(|t| {
        let kind = if t.target_source == "url" { "Web" } else { "Source" };
        match (&t.schedule, live()) {
            (_, true) => format!("{} · run live", kind),
            (Some(s), _) => format!("{} · {}", kind, s),
            (None, _) => format!("{} · manual", kind),
        }
    }).unwrap_or_default());
    let grade = move || row.with(|t| t.as_ref().map(|t| {
        if live() { "\u{2026}".to_string() } else { t.grade.clone().unwrap_or_else(|| "\u{2014}".into()) }
    }).unwrap_or_default());
    let delta = move || row.with(|t| t.as_ref().map(|t| match (t.score, t.previous_score) {
        (Some(s), Some(p)) if s >= p => format!("{} · \u{2191}{}", s, s - p),
        (Some(s), Some(p)) => format!("{} · \u{2193}{}", s, p - s),
        (Some(s), None) => s.to_string(),
        _ => "no score".to_string(),
    }).unwrap_or_default());

    let bar_items = move || row.with(|t| t.as_ref().map(|t| t.runs.clone()).unwrap_or_default()).into_iter().enumerate().collect::<Vec<_>>();

    view! {
        <a class="trow" href=href>
            <span class="frow-main">
                <span class="trow-name">{move || row.with(|t| t.as_ref().map(|t| t.target.clone()).unwrap_or_default())}</span>
                <span class="trow-meta">{meta}</span>
            </span>
            <span class="bars" aria-hidden="true">
                <For
                    each=bar_items
                    key=|(_, r)| (r.id, r.status.clone(), r.critical, r.high, r.other)
                    children=move |(i, r)| {
                        let total = (r.critical + r.high + r.other).max(1) as f64;
                        let scale = (40.0 / total).min(2.0);
                        let is_live = r.status == "running" || r.status == "pending";
                        view! {
                            <span class=if is_live { "bar live" } else { "bar" } style=format!("--d: {}ms", 300 + index * 40 + i * 30)>
                                <span class="c" style=format!("height: {:.1}px", r.critical as f64 * scale)></span>
                                <span class="h" style=format!("height: {:.1}px", r.high as f64 * scale)></span>
                                <span class="o" style=format!("height: {:.1}px", r.other as f64 * scale)></span>
                            </span>
                        }
                    }
                />
            </span>
            <span class="trow-score">
                <span class="trow-grade">{grade}</span>
                <span class="trow-delta">{delta}</span>
            </span>
        </a>
    }
}

async fn fetch_overview() -> Result<OverviewData, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/overview")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let api: ApiResponse<OverviewData> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Err("SSR".into())
    }
}
