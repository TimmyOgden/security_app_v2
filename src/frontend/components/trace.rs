use leptos::*;
use crate::models::TraceMark;

/// Spike height as a share of the trace's drawable height, by severity.
fn spike_height(severity: &str) -> f64 {
    match severity {
        "critical" => 0.82,
        "high" => 0.62,
        "medium" => 0.42,
        "low" => 0.26,
        _ => 0.13,
    }
}

fn spike_class(severity: &str) -> &'static str {
    match severity {
        "critical" => "spike s-crit",
        "high" => "spike s-high",
        _ => "spike s-rest",
    }
}

/// Deterministic jitter for the activity line, so history never moves.
fn noise(x: f64, seed: f64) -> f64 {
    (x * 0.91 + seed).sin() * 0.5
        + (x * 2.37 + 1.7 + seed * 0.5).sin() * 0.32
        + (x * 5.13 + 0.4 + seed * 0.3).sin() * 0.18
}

/// A run drawn as a seismograph: x is run time, spike height is severity.
/// New spikes grow in (CSS `.spike`); the head line marks "now" while live.
#[component]
pub fn RunTrace(
    #[prop(into)] marks: Signal<Vec<TraceMark>>,
    #[prop(into)] progress: Signal<f64>,
    #[prop(into)] live: Signal<bool>,
    #[prop(into)] pending: Signal<bool>,
    seed: i64,
    /// viewBox width; the SVG stretches to its box.
    width: f64,
    /// Height in px (and viewBox units).
    height: f64,
    /// Activity-line amplitude in px; 0 draws a flat baseline only.
    #[prop(default = 0.0)]
    amp: f64,
    /// Accessible name; without one the trace is decorative.
    #[prop(optional, into)]
    label: Option<String>,
) -> impl IntoView {
    let base = height - 2.0;
    let span = height - 6.0;
    let half = (width / 320.0).max(1.4);

    let base_d = move || {
        let p = progress.get().clamp(0.0, 1.0);
        let end = if live.get() || pending.get() || p >= 1.0 { width } else { p * width };
        format!("M0 {:.1}H{:.1}", base, end)
    };
    let noise_d = move || {
        if amp <= 0.0 {
            return "M0 0".to_string();
        }
        let head = progress.get().clamp(0.0, 1.0) * width;
        let step = (width / 400.0).max(1.0);
        let seed = seed as f64;
        let mut d = format!("M0 {:.1}", base);
        let mut x = 0.0;
        while x <= head {
            d.push_str(&format!("L{:.1} {:.1}", x, base - noise(x / width * 400.0, seed) * amp));
            x += step;
        }
        d
    };
    let head_x = move || format!("{:.2}", progress.get().clamp(0.0, 1.0) * width);
    let spikes = move || marks.get().into_iter().enumerate().collect::<Vec<_>>();
    let hidden = label.is_none();

    view! {
        <svg
            class="trace"
            viewBox=format!("0 0 {} {}", width, height)
            preserveAspectRatio="none"
            height=height.to_string()
            role=if hidden { None } else { Some("img") }
            aria-label=label
            aria-hidden=if hidden { Some("true") } else { None }
        >
            <path class=move || if pending.get() { "base pending" } else { "base" } d=base_d></path>
            <path class="noise" d=noise_d></path>
            <For
                each=spikes
                key=|(i, m)| (*i, m.severity.clone())
                children=move |(_, m)| {
                    let x = m.pos.clamp(0.0, 1.0) * width;
                    let h = spike_height(&m.severity) * span;
                    view! {
                        <path
                            class=spike_class(&m.severity)
                            d=format!("M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}", x - half, base, x, base - h, x + half, base)
                        ></path>
                    }
                }
            />
            <line
                class="head"
                x1=head_x
                x2=head_x
                y1="0"
                y2=base.to_string()
                style=move || if live.get() { "opacity: 1" } else { "opacity: 0" }
            ></line>
        </svg>
    }
}
