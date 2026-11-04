use leptos::*;
use leptos_router::*;
use crate::models::*;
use crate::frontend::app::RESTRICTED_TOOLS;
use super::super::components::fmt;
use super::super::components::trace::{spike_height, RunTrace};

const SEVERITIES: [&str; 5] = ["critical", "high", "medium", "low", "info"];
const TRACE_H: f64 = 260.0;
const TRACE_TOP: f64 = 28.0;

/// One run: the run drawn as a seismograph with its tool lanes, the findings
/// it produced (filter, triage, PDF export), the severity ledger, the score
/// and the comparison with the target's previous run, and the raw log.
#[component]
pub fn ResultsPage() -> impl IntoView {
    let params = use_params_map();
    let scan_id = move || {
        params.with(|p| p.get("id").cloned().unwrap_or_default().parse::<i64>().unwrap_or(0))
    };
    // Route params can notify more than once while they settle; effects that
    // open connections read through this memo so they run once per id.
    let scan_id_memo = create_memo(move |_| scan_id());

    #[allow(unused_variables)]
    let (log_entries, set_log_entries) = create_signal(Vec::<LogEntry>::new());
    #[allow(unused_variables)]
    let (tick, set_tick) = create_signal(0u32);
    #[allow(unused_variables)]
    let (now, set_now) = create_signal(0f64);
    #[allow(unused_variables)]
    let (fetched_at, set_fetched_at) = create_signal(0f64);
    let (severity_filter, set_severity_filter) = create_signal("all".to_string());
    let (tool_filter, set_tool_filter) = create_signal("all".to_string());
    let (issue_type_filter, set_issue_type_filter) = create_signal("all".to_string());
    let (search_query, set_search_query) = create_signal(String::new());
    let (hide_dismissed, set_hide_dismissed) = create_signal(true);
    let (show_log, set_show_log) = create_signal(false);
    let (hover, set_hover) = create_signal(Option::<i64>::None);
    let (expanded, set_expanded) = create_signal(Option::<i64>::None);
    let (overlay, set_overlay) = create_signal(false);

    let advanced_mode = use_context::<ReadSignal<bool>>().unwrap_or_else(|| create_signal(false).0);
    let allowed = move |tool: &str| advanced_mode.get() || !RESTRICTED_TOOLS.contains(&tool);

    // PDF export
    let (show_export, set_show_export) = create_signal(false);
    let (col_severity, set_col_severity) = create_signal(true);
    let (col_tool, set_col_tool) = create_signal(true);
    let (col_title, set_col_title) = create_signal(true);
    let (col_file, set_col_file) = create_signal(true);
    let (col_cwe, set_col_cwe) = create_signal(true);
    let (col_issue_type, set_col_issue_type) = create_signal(true);
    let (col_cvss, set_col_cvss) = create_signal(false);
    let (col_description, set_col_description) = create_signal(false);
    let (pdf_tool_scope, set_pdf_tool_scope) = create_signal("all".to_string());
    let (pdf_type_scope, set_pdf_type_scope) = create_signal("all".to_string());
    let (pdf_busy, set_pdf_busy) = create_signal(false);
    let (pdf_msg, set_pdf_msg) = create_signal(Option::<(bool, String)>::None);

    let generate_pdf = move |_: leptos::ev::MouseEvent| {
        #[allow(unused_variables)]
        let id = scan_id();
        let mut cols: Vec<String> = vec![];
        if col_severity.get_untracked() { cols.push("severity".into()); }
        if col_tool.get_untracked() { cols.push("tool".into()); }
        if col_title.get_untracked() { cols.push("title".into()); }
        if col_file.get_untracked() { cols.push("file".into()); }
        if col_cwe.get_untracked() { cols.push("cwe".into()); }
        if col_issue_type.get_untracked() { cols.push("type".into()); }
        if col_cvss.get_untracked() { cols.push("cvss".into()); }
        if col_description.get_untracked() { cols.push("description".into()); }
        let opts = PdfExportOptions {
            tool_filter: Some(pdf_tool_scope.get_untracked()),
            severity_filter: Some(severity_filter.get_untracked()),
            issue_type_filter: Some(pdf_type_scope.get_untracked()),
            search_query: {
                let q = search_query.get_untracked();
                if q.is_empty() { None } else { Some(q) }
            },
            columns: Some(cols),
        };
        set_pdf_busy.set(true);
        set_pdf_msg.set(None);
        #[cfg(feature = "hydrate")]
        wasm_bindgen_futures::spawn_local(async move {
            let result = match gloo_net::http::Request::post(&format!("/api/reports/{}/generate-pdf", id)).json(&opts) {
                Ok(req) => match req.send().await {
                    Ok(resp) if resp.ok() => Ok(()),
                    Ok(resp) => Err(resp.text().await.unwrap_or_else(|_| "Server error".into())),
                    Err(e) => Err(e.to_string()),
                },
                Err(e) => Err(e.to_string()),
            };
            set_pdf_busy.set(false);
            match result {
                Ok(()) => {
                    set_pdf_msg.set(Some((false, "PDF ready, downloading".into())));
                    if let Some(window) = web_sys::window() {
                        let _ = window.open_with_url_and_target(&format!("/api/reports/{}/download-pdf", id), "_blank");
                    }
                }
                Err(e) => set_pdf_msg.set(Some((true, e))),
            }
        });
        #[cfg(not(feature = "hydrate"))]
        { let _ = opts; }
    };

    let source = move || (scan_id(), tick.get());
    let detail_res = create_local_resource(source, |(id, _)| async move { fetch_detail(id).await });
    let findings_res = create_local_resource(source, |(id, _)| async move { fetch_findings(id).await });
    let scores_res = create_local_resource(source, |(id, _)| async move { fetch_scores(id).await });
    let diff_res = create_local_resource(source, |(id, _)| async move { fetch_diff(id).await });

    // Hold the last good value of each, so a refresh never blanks the page.
    let (detail, set_detail) = create_signal(Option::<RunDetail>::None);
    let (all_findings, set_all_findings) = create_signal(Vec::<Finding>::new());
    let (scores, set_scores) = create_signal(Option::<ScanScoreResponse>::None);
    let (diff, set_diff) = create_signal(Option::<ScanDiffResponse>::None);
    create_effect(move |_| {
        if let Some(Ok(d)) = detail_res.get() {
            set_detail.set(Some(d));
            set_fetched_at.set(fmt::now_ms());
        }
    });
    create_effect(move |_| if let Some(Ok(f)) = findings_res.get() { set_all_findings.set(f) });
    create_effect(move |_| if let Some(Ok(s)) = scores_res.get() { set_scores.set(Some(s)) });
    create_effect(move |_| if let Some(Ok(d)) = diff_res.get() { set_diff.set(Some(d)) });

    let run = create_memo(move |_| detail.with(|d| d.as_ref().map(|d| d.run.clone())));
    let status = create_memo(move |_| run.with(|r| r.as_ref().map(|r| r.status.clone()).unwrap_or_default()));
    let live = create_memo(move |_| matches!(status.get().as_str(), "running" | "pending"));
    let previous = create_memo(move |_| detail.with(|d| d.as_ref().and_then(|d| d.previous.clone())));
    let lanes = create_memo(move |_| detail.with(|d| d.as_ref().map(|d| d.lanes.clone()).unwrap_or_default()));
    let counts = create_memo(move |_| detail.with(|d| d.as_ref().map(|d| d.counts).unwrap_or_default()));

    // Refresh while the run is live: logs stream over SSE, everything else
    // is re-read every 3 s and only what changed re-renders.
    #[cfg(feature = "hydrate")]
    {
        create_effect(move |_| {
            let _ = scan_id_memo.get();
            set_now.set(fmt::now_ms());
            let refresh = gloo_timers::callback::Interval::new(3_000, move || {
                if live.get_untracked() || detail.with_untracked(|d| d.is_none()) {
                    set_tick.update(|t| *t += 1);
                }
            });
            let clock = gloo_timers::callback::Interval::new(1_000, move || set_now.set(fmt::now_ms()));
            on_cleanup(move || {
                drop(refresh);
                drop(clock);
            });
        });

        use wasm_bindgen::JsCast;
        create_effect(move |_| {
            let id = scan_id_memo.get();
            set_log_entries.set(Vec::new());
            if let Ok(es) = web_sys::EventSource::new(&format!("/api/scans/{}/stream", id)) {
                let es_clone = es.clone();
                let onmessage = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                    move |ev: web_sys::MessageEvent| {
                        if let Some(text) = ev.data().as_string() {
                            if let Ok(entry) = serde_json::from_str::<LogEntry>(&text) {
                                set_log_entries.update(|v| v.push(entry));
                            }
                        }
                    },
                );
                es.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                onmessage.forget();
                let ondone = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                    move |_ev: web_sys::MessageEvent| es_clone.close(),
                );
                let _ = es.add_event_listener_with_callback("done", ondone.as_ref().unchecked_ref());
                ondone.forget();
                on_cleanup(move || es.close());
            }
        });
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = scan_id_memo;

    // ── Derived views of the data ──
    let visible_findings = create_memo(move |_| {
        all_findings.get().into_iter().filter(|f| allowed(&f.tool)).collect::<Vec<_>>()
    });
    let filtered = create_memo(move |_| {
        let sev = severity_filter.get();
        let tool = tool_filter.get();
        let itype = issue_type_filter.get();
        let query = search_query.get().to_lowercase();
        let hide = hide_dismissed.get();
        visible_findings.get().into_iter().filter(|f| {
            (sev == "all" || f.severity == sev)
                && (tool == "all" || f.tool == tool)
                && (itype == "all" || f.issue_type.as_deref().unwrap_or("") == itype)
                && (!hide || !matches!(f.triage_status.as_deref(), Some("false_positive") | Some("wont_fix")))
                && (query.is_empty()
                    || f.title.to_lowercase().contains(&query)
                    || f.tool.to_lowercase().contains(&query)
                    || f.file_path.as_deref().unwrap_or("").to_lowercase().contains(&query))
        }).collect::<Vec<_>>()
    });
    let available_tools = create_memo(move |_| {
        let mut tools: Vec<String> = visible_findings.get().iter().map(|f| f.tool.clone()).collect();
        tools.sort();
        tools.dedup();
        tools
    });
    let sev_count = move |s: &str| visible_findings.with(|v| v.iter().filter(|f| s == "all" || f.severity == s).count());
    let new_ids = create_memo(move |_| diff.with(|d| d.as_ref().map(|d| d.new_findings.iter().map(|f| f.id).collect::<Vec<_>>()).unwrap_or_default()));
    let has_previous = create_memo(move |_| diff.with(|d| d.as_ref().map(|d| d.compared_to_scan_id.is_some()).unwrap_or(false)));
    // While a run is live, a previous finding only counts as fixed once the
    // tool that reported it has finished without reporting it again.
    let fixed_list = create_memo(move |_| {
        let done_tools: Vec<String> = lanes.with(|ls| ls.iter().filter(|l| l.state == "done").map(|l| l.tool.clone()).collect());
        let running = live.get();
        diff.with(|d| d.as_ref().map(|d| d.resolved_findings.iter()
            .filter(|f| !running || done_tools.contains(&f.tool))
            .cloned().collect::<Vec<_>>()).unwrap_or_default())
    });

    let marks = Signal::derive(move || run.with(|r| r.as_ref().map(|r| r.marks.clone()).unwrap_or_default())
        .into_iter().filter(|m| allowed(&m.tool)).collect::<Vec<_>>());
    let ghost = Signal::derive(move || previous.with(|p| p.as_ref().map(|p| p.marks.clone()).unwrap_or_default())
        .into_iter().filter(|m| allowed(&m.tool)).collect::<Vec<_>>());
    let progress = Signal::derive(move || run.with(|r| r.as_ref().map(|r| r.progress).unwrap_or(0.0)));
    let is_live = Signal::derive(move || status.get() == "running");
    let is_pending = Signal::derive(move || status.get() == "pending");

    // Seconds since the run started, ticking between refreshes while live.
    let elapsed = move || run.with(|r| r.as_ref().map(|r| {
        if matches!(r.status.as_str(), "running" | "pending") && fetched_at.get() > 0.0 {
            r.elapsed_seconds + ((now.get() - fetched_at.get()) / 1000.0).max(0.0) as i64
        } else {
            r.duration_seconds.unwrap_or(r.elapsed_seconds)
        }
    }).unwrap_or(0));
    // The whole axis in seconds (the run's expected length while live).
    let axis_seconds = move || run.with(|r| r.as_ref().map(|r| {
        let e = r.elapsed_seconds.max(1) as f64;
        if r.progress > 0.0 { e / r.progress } else { e }
    }).unwrap_or(1.0));
    let grid = Signal::derive(move || {
        let total = axis_seconds();
        let step = if total > 7200.0 { 600.0 } else if total > 1800.0 { 300.0 } else if total > 600.0 { 60.0 } else { 30.0 };
        let mut lines = Vec::new();
        let mut t = step;
        while t < total {
            lines.push((t / total, ((t / step).round() as i64) % 5 == 0));
            t += step;
        }
        lines
    });
    let ticks = move || {
        let total = axis_seconds();
        let step = if total > 7200.0 { 3000.0 } else if total > 1800.0 { 1500.0 } else if total > 600.0 { 300.0 } else { 150.0 };
        let mut out = vec![(0.0, "00:00".to_string())];
        let mut t = step;
        while t < total * 0.97 {
            out.push((t / total * 100.0, fmt::clock(t as i64)));
            t += step;
        }
        out
    };

    let found_offset = move |f: &Finding| -> String {
        let start = run.with(|r| r.as_ref().map(|r| fmt::utc_ms(&r.started_at)).unwrap_or(f64::NAN));
        match f.found_at.as_deref().map(fmt::utc_ms) {
            Some(t) if !t.is_nan() && !start.is_nan() => format!("+{}", fmt::clock(((t - start) / 1000.0) as i64)),
            _ => String::new(),
        }
    };
    let locator = move || {
        let id = hover.get()?;
        let m = marks.with(|ms| ms.iter().find(|m| m.id == id).cloned())?;
        let f = all_findings.with(|fs| fs.iter().find(|f| f.id == id).cloned())?;
        Some((m.pos * 100.0, format!("{} · {} {}", fmt::sev_label(&f.severity), f.title, found_offset(&f))))
    };
    let new_dots = move || {
        if !overlay.get() { return Vec::new(); }
        let ids = new_ids.get();
        marks.get().into_iter().filter(|m| ids.contains(&m.id)).map(|m| {
            let top = TRACE_TOP + (TRACE_H - 2.0) - spike_height(&m.severity) * (TRACE_H - 6.0) - 12.0;
            (m.pos * 100.0, top)
        }).collect::<Vec<_>>()
    };

    // Run again with the same target, preset and tools.
    let navigate = use_navigate();
    let run_again = move |_: leptos::ev::MouseEvent| {
        let Some(d) = detail.get_untracked() else { return };
        let req = StartScanRequest {
            scan_type: d.run.scan_type.clone(),
            target: d.run.target.clone(),
            target_source: d.target_source.clone(),
            tools: Some(d.tools.clone()),
        };
        let navigate = navigate.clone();
        #[cfg(feature = "hydrate")]
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(new_id) = start_again(req).await {
                navigate(&format!("/runs/{}", new_id), Default::default());
            }
        });
        #[cfg(not(feature = "hydrate"))]
        { let _ = (req, navigate); }
    };
    let act = move |action: &'static str| {
        #[allow(unused_variables)]
        let id = scan_id();
        #[cfg(feature = "hydrate")]
        wasm_bindgen_futures::spawn_local(async move {
            let _ = post_scan_action(id, action).await;
            set_tick.update(|t| *t += 1);
        });
        #[cfg(not(feature = "hydrate"))]
        let _ = action;
    };

    let state_label = move || match status.get().as_str() {
        "running" => "Scanning",
        "pending" => "Queued",
        "completed" => "Complete",
        "failed" => "Failed",
        "stopped" => "Stopped",
        _ => "Loading",
    };
    let score_now = create_memo(move |_| scores.with(|s| s.as_ref().map(|s| (s.overall_score, s.overall_grade.clone()))));
    let tool_scores = move || scores.with(|s| s.as_ref().map(|s| s.tool_scores.clone()).unwrap_or_default())
        .into_iter().filter(|t| allowed(&t.tool)).collect::<Vec<_>>();
    let band_of = |score: i64| -> usize {
        match score { 90..=100 => 6, 80..=89 => 5, 70..=79 => 4, 60..=69 => 3, 50..=59 => 2, 25..=49 => 1, _ => 0 }
    };

    let title_view = move || run.with(|r| r.as_ref().map(|r| {
        let (proto, rest) = match r.target.find("://") {
            Some(i) => (r.target[..i + 3].to_string(), r.target[i + 3..].to_string()),
            None => (String::new(), r.target.clone()),
        };
        view! { <span class="proto">{proto}</span>{rest} }
    }));
    let eyebrow = move || run.with(|r| r.as_ref().map(|r| {
        let mut s = format!(" / #{} · {} preset · started {}", r.id, r.scan_type, fmt::short_date(&r.started_at));
        if let Some(p) = previous.get() { s.push_str(&format!(" · compared with run #{}", p.id)); }
        s
    }).unwrap_or_default());

    let lane_items = move || lanes.get().into_iter().enumerate().collect::<Vec<_>>();
    let tab_items = move || {
        ["all", "critical", "high", "medium", "low", "info"].into_iter().map(|k| (k, sev_count(k))).collect::<Vec<_>>()
    };
    let ledger_items = move || {
        let c = counts.get();
        let p = previous.get().map(|p| p.counts);
        SEVERITIES.iter().enumerate().map(|(i, s)| (*s, c[i], p.map(|p| p[i]))).collect::<Vec<_>>()
    };
    let none_shown = move || detail.with(|d| d.is_some()) && filtered.with(|f| f.is_empty());
    let shown_logs = move || {
        let logs = log_entries.get();
        if show_log.get() || logs.len() <= 6 { logs } else { logs[logs.len() - 6..].to_vec() }
    };

    view! {
        <section aria-label="Run summary">
            <div class="eyebrow in" style="--d: 40ms">
                <A class="eyebrow-link" href="/runs">"Runs"</A>{eyebrow}
            </div>
            <div class="run-top">
                <h1 class="run-title in" style="--d: 80ms">{title_view}</h1>
                <div class="run-stats in" style="--d: 120ms">
                    <div>
                        <div class="run-stat-label">
                            <span class=move || if is_live.get() { "dot live" } else if status.get() == "completed" { "dot on" } else { "dot" }></span>
                            {state_label}
                        </div>
                        <div class="run-stat-value">{move || fmt::clock(elapsed())}</div>
                    </div>
                    <div>
                        <div class="run-stat-label">"Tools"</div>
                        <div class="run-stat-value">
                            {move || run.with(|r| r.as_ref().and_then(|r| r.tools_completed).unwrap_or(0))}
                            <span class="c-dim">{move || format!(" / {}", run.with(|r| r.as_ref().and_then(|r| r.tools_total).unwrap_or(0)))}</span>
                        </div>
                    </div>
                    <div class="run-actions-top">
                        {move || match status.get().as_str() {
                            "running" => view! {
                                <button type="button" class="wt-btn wt-btn-ghost" on:click=move |_| act("stop")>"Stop"</button>
                            }.into_view(),
                            "stopped" => view! {
                                <button type="button" class="wt-btn wt-btn-solid" on:click=move |_| act("resume")>"Resume"</button>
                            }.into_view(),
                            "completed" => view! {
                                <a class="wt-btn wt-btn-ghost" href=format!("/api/reports/{}/download-pdf", scan_id()) target="_blank" rel="noopener">"Download PDF"</a>
                                <button type="button" class="wt-btn wt-btn-solid" on:click=run_again.clone()>"Run again"</button>
                            }.into_view(),
                            "failed" => view! {
                                <button type="button" class="wt-btn wt-btn-solid" on:click=run_again.clone()>"Run again"</button>
                            }.into_view(),
                            _ => ().into_view(),
                        }}
                    </div>
                </div>
            </div>
        </section>

        <section class="scope in" style="--d: 180ms" aria-label="Run trace">
            <div class="scope-scroll">
                <div class="scope-inner">
                    <div class="scope-sev" aria-hidden="true">
                        {SEVERITIES[..4].iter().map(|s| {
                            let top = (TRACE_H - 2.0) - spike_height(s) * (TRACE_H - 6.0);
                            view! { <span class=format!("c-sev-{}", s) style=format!("top: {:.0}px", top)>{fmt::sev_label(s)}</span> }
                        }).collect_view()}
                    </div>
                    <RunTrace
                        marks=marks
                        progress=progress
                        live=is_live
                        pending=is_pending
                        seed=scan_id_memo.get_untracked()
                        width=1000.0
                        height=TRACE_H
                        amp=4.0
                        label=format!("Run {} trace", scan_id_memo.get_untracked())
                        ghost=ghost
                        ghost_on=Signal::derive(move || overlay.get())
                        grid=grid
                        guides=true
                    />
                    <div class="scope-now" aria-hidden="true"
                        style=move || format!("left: {:.3}%; opacity: {}", progress.get() * 100.0, if is_live.get() { 1 } else { 0 })>
                        {move || format!("Now {}", fmt::clock(elapsed()))}
                    </div>
                    {move || new_dots().into_iter().map(|(x, y)| view! {
                        <span class="newdot" aria-hidden="true" style=format!("left: {:.3}%; top: {:.0}px", x, y)></span>
                    }).collect_view()}
                    <div class="locator" aria-hidden="true"
                        style=move || match locator() {
                            Some((x, _)) => format!("left: {:.3}%; opacity: 1", x),
                            None => "opacity: 0".to_string(),
                        }>
                        <span>{move || locator().map(|(_, l)| l).unwrap_or_default()}</span>
                    </div>
                    <div class="lanes">
                        <For
                            each=lane_items
                            key=|(i, l)| (*i, l.tool.clone(), l.state.clone())
                            children=move |(i, _)| {
                                let lane = move || lanes.with(|ls| ls.get(i).cloned());
                                view! {
                                    <div
                                        class=move || format!("lane {}", lane().map(|l| l.state).unwrap_or_default())
                                        style=move || lane().map(|l| format!("left: {:.3}%; width: {:.3}%", l.start * 100.0, ((l.end - l.start) * 100.0).max(0.5))).unwrap_or_default()
                                    >
                                        <div class="lane-box">
                                            <div class="lane-fill"></div>
                                            <div class="lane-text">
                                                <div class="lane-name">{move || lane().map(|l| l.tool).unwrap_or_default()}</div>
                                                <div class="lane-sub">{move || lane().map(|l| match l.state.as_str() {
                                                    "pending" => "queued".to_string(),
                                                    _ => format!("{} · {} found", l.seconds.map(fmt::clock).unwrap_or_else(|| "--:--".into()), l.findings),
                                                }).unwrap_or_default()}</div>
                                            </div>
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                    <div class="ticks" aria-hidden="true">
                        {move || ticks().into_iter().map(|(x, label)| view! {
                            <span style=format!("left: {:.3}%", x)>{label}</span>
                        }).collect_view()}
                    </div>
                </div>
            </div>
        </section>

        <div class="run-cols">
            <section class="run-main in" style="--d: 240ms" aria-label="Findings">
                <div class="section-head">
                    <h2>"Findings "<span class="num c-dim">{move || filtered.with(|f| f.len())}</span></h2>
                    <button type="button" class="mini-btn" aria-expanded=move || if show_export.get() { "true" } else { "false" }
                        on:click=move |_| set_show_export.update(|v| *v = !*v)>
                        {move || if show_export.get() { "Close export" } else { "Export PDF" }}
                    </button>
                </div>

                <div class=move || if show_export.get() { "expand open" } else { "expand" }>
                    <div>
                        <div class="export-body">
                            <p class="section-note">"The PDF contains the findings that match the filters below, narrowed by these scopes."</p>
                            <div class="export-grid">
                                <label class="field-label">"Tool scope"
                                    <select class="field-sm" on:change=move |ev| set_pdf_tool_scope.set(event_target_value(&ev))>
                                        <option value="all">"All tools"</option>
                                        {move || available_tools.get().into_iter().map(|t| view! { <option value={t.clone()}>{t}</option> }).collect_view()}
                                    </select>
                                </label>
                                <label class="field-label">"Type scope"
                                    <select class="field-sm" on:change=move |ev| set_pdf_type_scope.set(event_target_value(&ev))>
                                        <option value="all">"All types"</option>
                                        <option value="Vulnerability">"Vulnerability"</option>
                                        <option value="Bug">"Bug"</option>
                                        <option value="Code Smell">"Code smell"</option>
                                        <option value="Security Hotspot">"Security hotspot"</option>
                                    </select>
                                </label>
                                <fieldset class="field-set">
                                    <legend class="field-label">"Columns"</legend>
                                    <label class="check"><input type="checkbox" checked=true on:change=move |ev| set_col_severity.set(event_target_checked(&ev))/>"Severity"</label>
                                    <label class="check"><input type="checkbox" checked=true on:change=move |ev| set_col_tool.set(event_target_checked(&ev))/>"Tool"</label>
                                    <label class="check"><input type="checkbox" checked=true on:change=move |ev| set_col_title.set(event_target_checked(&ev))/>"Title"</label>
                                    <label class="check"><input type="checkbox" checked=true on:change=move |ev| set_col_file.set(event_target_checked(&ev))/>"File"</label>
                                    <label class="check"><input type="checkbox" checked=true on:change=move |ev| set_col_cwe.set(event_target_checked(&ev))/>"CWE"</label>
                                    <label class="check"><input type="checkbox" checked=true on:change=move |ev| set_col_issue_type.set(event_target_checked(&ev))/>"Type"</label>
                                    <label class="check"><input type="checkbox" on:change=move |ev| set_col_cvss.set(event_target_checked(&ev))/>"CVSS"</label>
                                    <label class="check"><input type="checkbox" on:change=move |ev| set_col_description.set(event_target_checked(&ev))/>"Description"</label>
                                </fieldset>
                            </div>
                            <div class="export-actions">
                                <button type="button" class="wt-btn wt-btn-solid" disabled=move || pdf_busy.get() on:click=generate_pdf>
                                    {move || if pdf_busy.get() { "Generating..." } else { "Generate PDF" }}
                                </button>
                                {move || pdf_msg.get().map(|(err, msg)| view! {
                                    <span class=if err { "export-msg c-critical" } else { "export-msg" } role="status">{msg}</span>
                                })}
                            </div>
                        </div>
                    </div>
                </div>

                <div class="tabs tabs-tight" role="tablist" aria-label="Filter by severity">
                    <For
                        each=tab_items
                        key=|(k, n)| (*k, *n)
                        children=move |(k, n)| view! {
                            <button type="button" role="tab" class="tab"
                                aria-selected=move || if severity_filter.get() == k { "true" } else { "false" }
                                on:click=move |_| set_severity_filter.set(k.to_string())>
                                {if k == "all" { "All" } else { fmt::sev_label(k) }}" "<span class="num">{n}</span>
                            </button>
                        }
                    />
                </div>
                <div class="filters">
                    <select class="field-sm" aria-label="Tool" on:change=move |ev| set_tool_filter.set(event_target_value(&ev))>
                        <option value="all">"All tools"</option>
                        {move || available_tools.get().into_iter().map(|t| view! { <option value={t.clone()}>{t}</option> }).collect_view()}
                    </select>
                    <select class="field-sm" aria-label="Issue type" on:change=move |ev| set_issue_type_filter.set(event_target_value(&ev))>
                        <option value="all">"All types"</option>
                        <option value="Vulnerability">"Vulnerability"</option>
                        <option value="Bug">"Bug"</option>
                        <option value="Code Smell">"Code smell"</option>
                        <option value="Security Hotspot">"Security hotspot"</option>
                    </select>
                    <input type="search" class="field-sm field-grow" placeholder="Search title, tool or file" aria-label="Search findings"
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))/>
                    <label class="check">
                        <input type="checkbox" prop:checked=move || hide_dismissed.get()
                            on:change=move |ev| set_hide_dismissed.set(event_target_checked(&ev))/>
                        "Hide dismissed"
                    </label>
                </div>

                <div class="flist">
                    <For
                        each=move || filtered.get()
                        key=|f| (f.id, f.triage_status.clone())
                        children=move |f| {
                            let id = f.id;
                            let is_new = move || has_previous.get() && new_ids.with(|n| n.contains(&id));
                            let dismissed = matches!(f.triage_status.as_deref(), Some("false_positive") | Some("wont_fix"));
                            let triage_tag = match f.triage_status.as_deref() {
                                Some("false_positive") => Some(("False positive", "tag")),
                                Some("wont_fix") => Some(("Won't fix", "tag")),
                                Some("confirmed") => Some(("Confirmed", "tag critical")),
                                _ => None,
                            };
                            let cvss = f.cvss_score.map(|v| (format!("CVSS {:.1}", v), if v >= 9.0 { "cvss c-critical" } else if v >= 7.0 { "cvss c-high" } else { "cvss" }));
                            let location = f.file_path.clone().map(|p| match f.line_number { Some(l) => format!("{}:{}", p, l), None => p });
                            let offset = found_offset(&f);
                            let reset = !matches!(f.triage_status.as_deref(), None | Some("open"));
                            view! {
                                <div class="frow-wrap row-enter">
                                    <button type="button"
                                        class=if dismissed { "frow dismissed" } else { "frow" }
                                        aria-expanded=move || if expanded.get() == Some(id) { "true" } else { "false" }
                                        on:click=move |_| set_expanded.update(|e| *e = if *e == Some(id) { None } else { Some(id) })
                                        on:mouseenter=move |_| set_hover.set(Some(id))
                                        on:mouseleave=move |_| set_hover.set(None)
                                        on:focus=move |_| set_hover.set(Some(id))
                                        on:blur=move |_| set_hover.set(None)
                                    >
                                        <span class=format!("sevmark {}", f.severity)><i></i>{fmt::sev_label(&f.severity)}</span>
                                        <span class="frow-main">
                                            <span class="frow-title">{f.title.clone()}</span>
                                            <span class="frow-meta">{format!("{} · {}", f.tool, location.clone().unwrap_or_else(|| f.issue_type.clone().unwrap_or_default()))}</span>
                                        </span>
                                        <span class="frow-side">
                                            {triage_tag.map(|(t, c)| view! { <span class=c>{t}</span> })}
                                            {move || is_new().then(|| view! { <span class="tag">"New"</span> })}
                                            {cvss.map(|(t, c)| view! { <span class=c>{t}</span> })}
                                            <span>{offset}</span>
                                        </span>
                                    </button>
                                    <div class=move || if expanded.get() == Some(id) { "expand open" } else { "expand" }>
                                        <div>
                                            <div class="fdetail">
                                                {f.description.clone().map(|d| view! { <p class="fdetail-desc">{d}</p> })}
                                                <div class="fdetail-meta">
                                                    {location.clone().map(|l| view! { <span>{l}</span> })}
                                                    {f.issue_type.clone().map(|t| view! { <span>{t}</span> })}
                                                    {f.cwe_id.clone().map(|cwe| view! {
                                                        <a href=format!("https://cwe.mitre.org/data/definitions/{}.html", cwe.trim_start_matches("CWE-")) target="_blank" rel="noopener">{cwe}</a>
                                                    })}
                                                    {f.rule_url.clone().map(|u| view! { <a href=u target="_blank" rel="noopener">"Rule"</a> })}
                                                </div>
                                                {f.recommendation.clone().map(|r| view! { <p class="fdetail-rec">{r}</p> })}
                                                <div class="triage">
                                                    <button type="button" class="mini-btn" on:click=move |_| triage(id, "false_positive", set_tick)>"Not a real issue"</button>
                                                    <button type="button" class="mini-btn" on:click=move |_| triage(id, "wont_fix", set_tick)>"Won't fix"</button>
                                                    <button type="button" class="mini-btn" on:click=move |_| triage(id, "confirmed", set_tick)>"Confirmed"</button>
                                                    {reset.then(|| view! {
                                                        <button type="button" class="mini-btn" on:click=move |_| triage(id, "open", set_tick)>"Reset"</button>
                                                    })}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            }
                        }
                    />
                    <Show when=none_shown>
                        <p class="empty-line">{move || if live.get() && visible_findings.with(|v| v.is_empty()) { "Nothing found yet." } else { "No findings match these filters." }}</p>
                    </Show>
                </div>

                <div class="section-head log-head">
                    <h2>"Raw log"</h2>
                    <button type="button" class="mini-btn" on:click=move |_| set_show_log.update(|v| *v = !*v)>
                        {move || if show_log.get() { "Show less" } else { "Show all" }}
                    </button>
                </div>
                <div class=move || if show_log.get() { "term term-full" } else { "term" } role="log" aria-live="polite">
                    {move || shown_logs().into_iter().map(|l| view! {
                        <div class=format!("term-line {}", l.level)>
                            <span class="term-time">{l.timestamp.get(11..19).unwrap_or(&l.timestamp).to_string()}</span>
                            <span class="term-tool">{l.tool.clone().map(|t| format!("[{}]", t)).unwrap_or_default()}</span>
                            <span class="term-msg">{l.message.clone()}</span>
                        </div>
                    }).collect_view()}
                    <Show when=move || log_entries.with(|l| l.is_empty())>
                        <div class="term-line"><span class="term-msg">"No log lines yet."</span></div>
                    </Show>
                </div>
            </section>

            <aside class="run-side in" style="--d: 300ms" aria-label="Run summary">
                <div class="ledger-head">
                    <h2>"Severity"</h2>
                    <span>{move || previous.get().map(|p| format!("Run #{}", p.id)).unwrap_or_default()}</span>
                </div>
                <For
                    each=ledger_items
                    key=|(s, n, p)| (*s, *n, *p)
                    children=move |(s, n, p)| view! {
                        <div class="ledger-row">
                            <span class=format!("ledger-label c-sev-{}", s)>{fmt::sev_label(s)}</span>
                            <span class=format!("ledger-num roll n-{}{}", s, if n == 0 { " zero" } else { "" })>{n}</span>
                            <span class="ledger-prev">{p.map(|p| p.to_string()).unwrap_or_default()}</span>
                        </div>
                    }
                />

                <div class="section-head side-head">
                    <h2>"Score"</h2>
                    <span class="section-note">{move || previous.get().map(|p| format!("Run #{} · {} · {}", p.id, p.score, p.grade)).unwrap_or_default()}</span>
                </div>
                <div class=move || if live.get() { "score-big provisional" } else { "score-big" }>
                    {move || score_now.get().map(|(s, g)| if live.get() {
                        view! { <b class="roll">{s}</b><small>{format!("provisional · {}", g)}</small> }.into_view()
                    } else {
                        view! { <b class="roll">{g}</b><small>{format!("{} / 100", s)}</small> }.into_view()
                    })}
                </div>
                <div class="gauge" aria-hidden="true">
                    <div class="gauge-bands">
                        {[("F", 25), ("E", 25), ("D", 10), ("C", 10), ("B", 10), ("A", 10), ("A+", 10)].into_iter().enumerate().map(|(i, (_, w))| view! {
                            <span style=format!("flex: {} 1 0", w)
                                class=move || if score_now.get().map(|(s, _)| band_of(s) == i).unwrap_or(false) { "on" } else { "" }></span>
                        }).collect_view()}
                    </div>
                    <div class="gauge-labels">
                        {[("F", 25), ("E", 25), ("D", 10), ("C", 10), ("B", 10), ("A", 10), ("A+", 10)].into_iter().enumerate().map(|(i, (g, w))| view! {
                            <span style=format!("flex: {} 1 0", w)
                                class=move || if score_now.get().map(|(s, _)| band_of(s) == i).unwrap_or(false) { "on" } else { "" }>{g}</span>
                        }).collect_view()}
                    </div>
                    {move || previous.get().map(|p| view! { <span class="prev-mark" title=format!("Run #{}", p.id) style=format!("left: {}%", p.score)></span> })}
                    <span class=move || if live.get() { "needle" } else { "needle final" }
                        style=move || format!("left: {}%", score_now.get().map(|(s, _)| s).unwrap_or(100))></span>
                </div>

                <Show when=move || has_previous.get()>
                    <div class="section-head side-head">
                        <h2>{move || previous.get().map(|p| format!("Vs run #{}", p.id)).unwrap_or_default()}</h2>
                        <span class="section-note">{move || previous.get().map(|p| fmt::short_date(&p.started_at)).unwrap_or_default()}</span>
                    </div>
                    <div class="diff3">
                        <div><b class="roll">{move || diff.with(|d| d.as_ref().map(|d| d.new_findings.len()).unwrap_or(0))}</b><span>"New"</span></div>
                        <div><b class="roll">{move || fixed_list.with(|f| f.len())}</b><span>"Fixed"</span></div>
                        <div><b class="roll">{move || diff.with(|d| d.as_ref().map(|d| d.unchanged_count).unwrap_or(0))}</b><span>"Still open"</span></div>
                    </div>
                    <button type="button" class="switch" role="switch"
                        aria-checked=move || if overlay.get() { "true" } else { "false" }
                        on:click=move |_| set_overlay.update(|v| *v = !*v)>
                        <span class="switch-track"><span class="switch-knob"></span></span>
                        {move || previous.get().map(|p| format!("Overlay run #{} on the trace", p.id)).unwrap_or_default()}
                    </button>
                    {move || fixed_list.get().into_iter().map(|f| view! {
                        <div class="fixed-row">
                            <span class="fixed-sev">{fmt::sev_label(&f.severity)}</span>
                            <span class="fixed-title">{f.title.clone()}</span>
                        </div>
                    }).collect_view()}
                    {move || previous.get().map(|p| view! { <A class="section-link side-link" href=format!("/runs/{}", p.id)>{format!("Open run #{} →", p.id)}</A> })}
                </Show>

                <div class="section-head side-head">
                    <h2>"Tools"</h2>
                    <span class="section-note">"Score by tool"</span>
                </div>
                {move || tool_scores().into_iter().map(|t| {
                    let fill = if t.score < 40 { "bad" } else if t.score < 60 { "poor" } else { "" };
                    view! {
                        <div class="toolscore">
                            <span class="toolscore-name">{t.tool.clone()}</span>
                            <span class=format!("toolscore-bar {}", fill)><span style=format!("width: {}%", t.score)></span></span>
                            <span class="toolscore-grade">{t.grade.clone()}</span>
                        </div>
                    }
                }).collect_view()}
            </aside>
        </div>
    }
}

#[allow(unused_variables)]
fn triage(id: i64, status: &'static str, set_tick: WriteSignal<u32>) {
    #[cfg(feature = "hydrate")]
    wasm_bindgen_futures::spawn_local(async move {
        let _ = post_triage(id, status).await;
        set_tick.update(|t| *t += 1);
    });
}

fn event_target_value(ev: &leptos::ev::Event) -> String {
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()
                .map(|e| e.value())
                .or_else(|| ev.target()
                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|e| e.value())))
            .unwrap_or_default()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = ev;
        String::new()
    }
}

fn event_target_checked(ev: &leptos::ev::Event) -> bool {
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|e| e.checked())
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = ev;
        false
    }
}

async fn fetch_detail(id: i64) -> Result<RunDetail, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/runs/{}", id))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<RunDetail> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("Not found".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

async fn fetch_findings(id: i64) -> Result<Vec<Finding>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}/findings", id))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<Finding>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

async fn fetch_scores(id: i64) -> Result<ScanScoreResponse, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}/score", id))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<ScanScoreResponse> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

async fn fetch_diff(id: i64) -> Result<ScanDiffResponse, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}/diff", id))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<ScanDiffResponse> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn start_again(req: StartScanRequest) -> Result<i64, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/scans/start")
            .json(&req).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.and_then(|d| d.get("scan_id").and_then(|v| v.as_i64())).ok_or_else(|| api.message.unwrap_or("Could not start".into()))
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = req; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn post_triage(finding_id: i64, status: &str) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let req = TriageRequest { status: status.to_string(), note: None };
        let resp = gloo_net::http::Request::post(&format!("/api/findings/{}/triage", finding_id))
            .json(&req).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(()) } else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = (finding_id, status); Err("SSR".into()) }
}

#[allow(dead_code)]
async fn post_scan_action(id: i64, action: &str) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let _ = gloo_net::http::Request::post(&format!("/api/scans/{}/{}", id, action))
            .send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = (id, action); Err("SSR".into()) }
}
