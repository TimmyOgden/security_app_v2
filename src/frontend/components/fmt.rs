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

/// Percent-encodes a value for a URL query string.
pub fn encode(value: &str) -> String {
    value.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{:02X}", b),
    }).collect()
}

/// "Source folder" or "Web target", from a run's target_source.
pub fn target_kind(source: &str) -> &'static str {
    match source {
        "url" => "Web target",
        "azure" => "Azure DevOps repository",
        _ => "Source folder",
    }
}

/// UTC calendar fields for a time in ms: (month 1-12, day, weekday 0=Sun, hour, minute).
pub fn civil(ms: f64) -> (u32, u32, u32, u32, u32) {
    let days = (ms / 86_400_000.0).floor() as i64;
    let secs = ((ms / 1000.0).floor() as i64).rem_euclid(86_400);
    let weekday = ((days % 7 + 7) % 7 + 4) % 7;
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (month, day, weekday as u32, (secs / 3600) as u32, ((secs % 3600) / 60) as u32)
}

pub fn weekday_name(weekday: u32) -> &'static str {
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][weekday as usize % 7]
}

/// "Wed 09:05" for a time in ms (UTC); empty if unknown.
pub fn day_time(ms: f64) -> String {
    if ms.is_nan() || ms <= 0.0 {
        return String::new();
    }
    let (_, _, wd, h, m) = civil(ms);
    format!("{} {:02}:{:02}", weekday_name(wd), h, m)
}

/// "every 6h", "daily", "every 3 days", "weekly".
pub fn cadence(hours: i64) -> String {
    match hours {
        24 => "daily".into(),
        168 => "weekly".into(),
        h if h % 168 == 0 => format!("every {} weeks", h / 168),
        h if h % 24 == 0 => format!("every {} days", h / 24),
        h => format!("every {}h", h),
    }
}
