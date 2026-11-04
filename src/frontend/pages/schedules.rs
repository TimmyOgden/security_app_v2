use leptos::*;
use crate::models::*;
use super::super::components::fmt;

const WEEK_MS: f64 = 7.0 * 86_400_000.0;
const INTERVALS: [(i64, &str); 5] = [(6, "Every 6h"), (12, "Every 12h"), (24, "Daily"), (72, "Every 3 days"), (168, "Weekly")];

/// Recurring runs as a timetable: each schedule is a lane across the next
/// seven days with its upcoming runs marked, a live countdown, and a switch
/// to pause it.
#[component]
pub fn SchedulesPage() -> impl IntoView {
    let (refresh_counter, set_refresh_counter) = create_signal(0u32);
    let schedules_res = create_local_resource(move || refresh_counter.get(), |_| async { fetch_schedules().await });
    let presets = create_local_resource(|| (), |_| async { fetch_presets().await });
    let (schedules, set_schedules) = create_signal(Vec::<ScheduledScan>::new());
    let (loaded, set_loaded) = create_signal(false);
    create_effect(move |_| {
        if let Some(Ok(list)) = schedules_res.get() {
            set_schedules.set(list);
            set_loaded.set(true);
        }
    });
    // Starts at 0 on both sides so hydration sees the same tree; the effect
    // below sets the real clock once mounted.
    #[allow(unused_variables)]
    let (now, set_now) = create_signal(0.0);
    #[cfg(feature = "hydrate")]
    create_effect(move |_| {
        set_now.set(fmt::now_ms());
        let clock = gloo_timers::callback::Interval::new(1_000, move || set_now.set(fmt::now_ms()));
        let refresh = gloo_timers::callback::Interval::new(30_000, move || set_refresh_counter.update(|v| *v += 1));
        on_cleanup(move || {
            drop(clock);
            drop(refresh);
        });
    });

    let (open, set_open) = create_signal(false);
    let (scan_type, set_scan_type) = create_signal("web".to_string());
    let (target, set_target) = create_signal(String::new());
    let (target_source, set_target_source) = create_signal("url".to_string());
    let (interval_hours, set_interval_hours) = create_signal(24i64);
    let (create_msg, set_create_msg) = create_signal(Option::<(bool, String)>::None);

    let create_schedule = create_action(move |_: &()| {
        let req = CreateScheduleRequest {
            scan_type: scan_type.get(),
            target: target.get(),
            target_source: target_source.get(),
            tools: None,
            interval_hours: interval_hours.get(),
        };
        let hours = req.interval_hours;
        async move {
            match do_create_schedule(req).await {
                Ok(_) => {
                    set_create_msg.set(Some((false, format!("Schedule created. Its first run is {} from now.", fmt::cadence(hours).replace("every ", "").replace("daily", "a day").replace("weekly", "a week")))));
                    set_target.set(String::new());
                    set_open.set(false);
                    set_refresh_counter.update(|v| *v += 1);
                }
                Err(e) => set_create_msg.set(Some((true, e))),
            }
        }
    });

    let active = move || schedules.with(|l| l.iter().filter(|s| s.enabled).count());
    let next_in = move || {
        let t = now.get();
        let soonest = schedules.with(|l| l.iter().filter(|s| s.enabled)
            .map(|s| fmt::utc_ms(&s.next_run_at)).filter(|v| !v.is_nan())
            .fold(f64::NAN, |a: f64, b| if a.is_nan() || b < a { b } else { a }));
        if soonest.is_nan() { String::new() } else { fmt::countdown(soonest, t) }
    };
    let days = move || {
        let t = now.get();
        if t <= 0.0 { return Vec::new(); }
        let first_midnight = ((t / 86_400_000.0).floor() + 1.0) * 86_400_000.0;
        let mut out = vec![(0.0, { let (_, d, wd, _, _) = fmt::civil(t); format!("{} {}", fmt::weekday_name(wd), d) }, true)];
        let mut m = first_midnight;
        while m < t + WEEK_MS {
            let (_, d, wd, _, _) = fmt::civil(m);
            out.push(((m - t) / WEEK_MS * 100.0, format!("{} {}", fmt::weekday_name(wd), d), false));
            m += 86_400_000.0;
        }
        out
    };
    let preview = move || {
        let step = interval_hours.get().max(1) as f64 * 3_600_000.0;
        (0..42).map(|k| {
            let at = step * (k + 1) as f64;
            (if at <= WEEK_MS { at / WEEK_MS * 100.0 } else { 100.0 }, at <= WEEK_MS, k == 0)
        }).collect::<Vec<_>>()
    };
    let preview_count = move || preview().iter().filter(|p| p.1).count();
    let lane_items = move || schedules.get();
    let on_hours = move |ev: leptos::ev::Event| {
        if let Ok(v) = event_target_value(&ev).parse::<i64>() {
            set_interval_hours.set(v.max(1));
        }
    };
    let empty = move || loaded.get() && schedules.with(|l| l.is_empty());

    view! {
        <section aria-label="Schedules">
            <div class="page-head">
                <div class="in" style="--d: 40ms">
                    <div class="eyebrow">"Schedules · next 7 days"</div>
                    <h1 class="page-title">
                        {active}" active"
                        {move || { let n = next_in(); if n.is_empty() { String::new() } else { format!(", next run {}", n) } }}
                    </h1>
                </div>
                <button type="button" class=move || if open.get() { "wt-btn wt-btn-ghost in" } else { "wt-btn wt-btn-solid in" }
                    style="--d: 80ms" aria-expanded=move || if open.get() { "true" } else { "false" }
                    on:click=move |_| set_open.update(|v| *v = !*v)>
                    {move || if open.get() { "Close" } else { "New schedule" }}
                </button>
            </div>
            {move || create_msg.get().map(|(err, msg)| view! { <p class=if err { "notice error" } else { "notice ok" } role="status">{msg}</p> })}

            <div class=move || if open.get() { "expand open" } else { "expand" }>
                <div>
                    <div class="sched-form">
                        <div class="seg" role="radiogroup" aria-label="Target source">
                            {[("url", "URL"), ("local", "Local folder")].into_iter().map(|(key, label)| view! {
                                <button type="button" role="radio" class="seg-btn"
                                    aria-checked=move || if target_source.get() == key { "true" } else { "false" }
                                    on:click=move |_| set_target_source.set(key.to_string())>{label}</button>
                            }).collect_view()}
                        </div>
                        <div class="sched-grid">
                            <label class="field-label">"Target"
                                <input type="text" class="field"
                                    placeholder=move || if target_source.get() == "url" { "https://example.com" } else { "/projects/my-app" }
                                    prop:value=move || target.get()
                                    on:input=move |ev| set_target.set(event_target_value(&ev))/>
                            </label>
                            <label class="field-label">"Preset"
                                <select class="field prose" on:change=move |ev| set_scan_type.set(event_target_value(&ev))>
                                    {move || presets.get().and_then(|r| r.ok()).unwrap_or_default().into_iter().map(|p| {
                                        let name = p.name.clone();
                                        view! { <option value={p.name.clone()} selected=move || scan_type.get() == name>{p.display_name.clone()}</option> }
                                    }).collect_view()}
                                </select>
                            </label>
                        </div>
                        <div class="field-label">"Repeat"</div>
                        <div class="seg" role="radiogroup" aria-label="Repeat every">
                            {INTERVALS.into_iter().map(|(h, label)| view! {
                                <button type="button" role="radio" class="seg-btn"
                                    aria-checked=move || if interval_hours.get() == h { "true" } else { "false" }
                                    on:click=move |_| set_interval_hours.set(h)>{label}</button>
                            }).collect_view()}
                            <label class="seg-custom">
                                <input type="number" class="field-sm" min="1" aria-label="Hours between runs"
                                    prop:value=move || interval_hours.get().to_string()
                                    on:input=on_hours/>
                                "hours"
                            </label>
                        </div>
                        <div class="preview-head"><span class="field-label">"Preview · next 7 days"</span><span class="num c-dim">{move || format!("{} runs", preview_count())}</span></div>
                        <div class="preview" aria-hidden="true">
                            {move || preview().into_iter().map(|(x, on, first)| view! {
                                <span class=if first { "ptick first" } else { "ptick" } style=format!("left: {:.3}%; opacity: {}", x, if on { 1 } else { 0 })></span>
                            }).collect_view()}
                        </div>
                        <div class="form-actions">
                            <button type="button" class="wt-btn wt-btn-solid"
                                disabled=move || target.get().is_empty()
                                on:click=move |_| create_schedule.dispatch(())>"Create schedule"</button>
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <section class="in" style="--d: 120ms" aria-label="Timetable">
            <div class="tt-scroll">
                <div class="tt">
                    <div class="tt-head" aria-hidden="true">
                        <span>"Schedule"</span>
                        <span class="tt-days">
                            // A midnight right after "today" would overlap its label.
                            {move || days().into_iter().filter(|(x, _, today)| *today || *x >= 6.0).map(|(x, label, today)| view! {
                                <span class=if today { "tt-day today" } else { "tt-day" } style=format!("left: {:.3}%", x)>{label}</span>
                            }).collect_view()}
                        </span>
                        <span class="tt-right">"Next run"</span>
                    </div>
                    <For
                        each=lane_items
                        key=|s| (s.id, s.enabled, s.next_run_at.clone(), s.interval_hours)
                        children=move |s| lane(s, now, set_refresh_counter, days)
                    />
                    <Show when=empty>
                        <p class="empty-line">"No schedules yet. A schedule starts a run on an interval, with no one at the keyboard."</p>
                    </Show>
                </div>
            </div>
        </section>
    }
}

fn lane(
    s: ScheduledScan,
    now: ReadSignal<f64>,
    #[allow(unused_variables)] set_refresh: WriteSignal<u32>,
    days: impl Fn() -> Vec<(f64, String, bool)> + Copy + 'static,
) -> impl IntoView {
    #[allow(unused_variables)]
    let sid = s.id;
    let next = fmt::utc_ms(&s.next_run_at);
    let step = s.interval_hours.max(1) as f64 * 3_600_000.0;
    let enabled = s.enabled;
    // Which runs fall in the window changes rarely; positions change every
    // second. The memo keeps the tick set stable so their grow-in plays once.
    let ticks = create_memo(move |_| {
        let t = now.get();
        if t <= 0.0 || next.is_nan() { return Vec::new(); }
        let mut out = Vec::new();
        let mut at = next;
        let mut k = 0;
        while at <= t + WEEK_MS && k < 60 {
            if at >= t { out.push((at as i64, k == 0, out.len())); }
            at += step;
            k += 1;
        }
        out
    });
    let meta = format!("{} · {}{}", s.scan_type, fmt::cadence(s.interval_hours), if enabled { "" } else { " · paused" });
    let toggle_label = format!("{} {}", if enabled { "Pause" } else { "Resume" }, s.target);
    view! {
        <div class=if enabled { "tt-row row-enter" } else { "tt-row row-enter paused" }>
            <div class="tt-name">
                <button type="button" class="switch inline" role="switch" aria-checked=if enabled { "true" } else { "false" } aria-label=toggle_label
                    on:click=move |_| {
                        #[cfg(feature = "hydrate")]
                        wasm_bindgen_futures::spawn_local(async move {
                            let _ = do_toggle_schedule(sid).await;
                            set_refresh.update(|v| *v += 1);
                        });
                    }>
                    <span class="switch-track"><span class="switch-knob"></span></span>
                </button>
                <span class="frow-main">
                    <span class="trow-name">{s.target.clone()}</span>
                    <span class="trow-meta">{meta}</span>
                </span>
            </div>
            <div class="tt-lane" aria-hidden="true">
                {move || days().into_iter().skip(1).map(|(x, _, _)| view! { <span class="tt-gridline" style=format!("left: {:.3}%", x)></span> }).collect_view()}
                <span class="tt-base"></span>
                <For
                    each=move || ticks.get()
                    key=|(at, first, _)| (*at, *first)
                    children=move |(at, first, i)| view! {
                        <span class=if first { "tt-tick first" } else { "tt-tick" }
                            style=move || format!("left: {:.3}%; --d: {}ms", (at as f64 - now.get()) / WEEK_MS * 100.0, 200 + i * 40)></span>
                    }
                />
            </div>
            <div class="tt-right">
                <span class="tt-count num">{move || if enabled { fmt::countdown(next, now.get()) } else { "paused".to_string() }}</span>
                <span class="tt-when num">{if enabled { fmt::day_time(next) } else { format!("would be {}", fmt::day_time(next)) }}</span>
                <button type="button" class="mini-btn" aria-label=format!("Delete the schedule for {}", s.target) on:click=move |_| {
                    #[cfg(feature = "hydrate")]
                    {
                        let ok = web_sys::window().and_then(|w| w.confirm_with_message("Delete this schedule? Past runs are kept.").ok()).unwrap_or(false);
                        if ok {
                            wasm_bindgen_futures::spawn_local(async move {
                                let _ = do_delete_schedule(sid).await;
                                set_refresh.update(|v| *v += 1);
                            });
                        }
                    }
                }>"Delete"</button>
            </div>
        </div>
    }
}

async fn fetch_schedules() -> Result<Vec<ScheduledScan>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/schedules")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ScheduledScan>> = resp.json().await.map_err(|e| e.to_string())?;
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

async fn do_create_schedule(req: CreateScheduleRequest) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/schedules")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(()) } else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = req; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn do_toggle_schedule(id: i64) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        gloo_net::http::Request::post(&format!("/api/schedules/{}/toggle", id))
            .send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn do_delete_schedule(id: i64) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        gloo_net::http::Request::delete(&format!("/api/schedules/{}", id))
            .send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}
