//! Small display helpers shared by the Overview and Runs pages.
//! Server timestamps are UTC "YYYY-MM-DD HH:MM:SS".

/// Milliseconds since the epoch for a server timestamp; NaN on the server
/// render or if it does not parse.
pub fn utc_ms(ts: &str) -> f64 {
    #[cfg(feature = "hydrate")]
    {
        js_sys::Date::parse(&format!("{}Z", ts.replacen(' ', "T", 1)))
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = ts;
        f64::NAN
    }
}

/// The browser clock in ms; 0 during the server render.
pub fn now_ms() -> f64 {
    #[cfg(feature = "hydrate")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        0.0
    }
}

/// 82 -> "01:22", 3700 -> "1:01:40".
pub fn clock(secs: i64) -> String {
    let s = secs.max(0);
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, sec)
    } else {
        format!("{:02}:{:02}", m, sec)
    }
}

/// "2026-11-03 17:40:02" -> "Nov 3 17:40".
pub fn short_date(ts: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let month = ts.get(5..7).and_then(|m| m.parse::<usize>().ok()).filter(|m| (1..=12).contains(m));
    let day = ts.get(8..10).and_then(|d| d.parse::<u32>().ok());
    match (month, day, ts.get(11..16)) {
        (Some(m), Some(d), Some(hm)) => format!("{} {} {}", MONTHS[m - 1], d, hm),
        _ => ts.to_string(),
    }
}

/// Time until `target_ms`, e.g. "in 17h 05m 40s"; empty before hydration.
pub fn countdown(target_ms: f64, now: f64) -> String {
    if now <= 0.0 || target_ms.is_nan() {
        return String::new();
    }
    let secs = ((target_ms - now) / 1000.0).floor() as i64;
    if secs <= 0 {
        return "due now".into();
    }
    let (d, h, m, s) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60, secs % 60);
    if d > 0 {
        format!("in {}d {:02}h {:02}m", d, h, m)
    } else {
        format!("in {:02}h {:02}m {:02}s", h, m, s)
    }
}

/// How long a finding has been open, from the first run that reported it.
pub fn open_for(first_seen: Option<&str>, now: f64) -> String {
    let Some(ts) = first_seen else { return String::new() };
    let t = utc_ms(ts);
    if now <= 0.0 || t.is_nan() {
        return String::new();
    }
    match ((now - t) / 86_400_000.0).floor() as i64 {
        d if d <= 0 => "open today".into(),
        d => format!("open {} d", d),
    }
}

/// Short severity label; CSS sets the case.
pub fn sev_label(severity: &str) -> &'static str {
    match severity {
        "critical" => "Crit",
        "high" => "High",
        "medium" => "Med",
        "low" => "Low",
        _ => "Info",
    }
}
