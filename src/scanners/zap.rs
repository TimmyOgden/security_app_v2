use crate::db::{self, DbPool};
use crate::models::ToolFinding;

/// How long to wait between status polls.
const SPIDER_POLL_SECS: u64 = 2;
const ASCAN_POLL_SECS: u64 = 3;
/// Hard ceilings so a target with nothing crawlable (or an unreachable host)
/// can never hang the scan forever.
const SPIDER_MAX_POLLS: u32 = 150; // ~5 minutes
const ASCAN_MAX_POLLS: u32 = 400; // ~20 minutes

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    let zap_api_key = db::get_setting(pool, "zap_api_key").await;
    let api_key = if zap_api_key.is_empty() { "changeme".to_string() } else { zap_api_key };
    let zap_url = std::env::var("ZAP_URL").unwrap_or_else(|_| "http://zap:8080".into());
    let client = reqwest::Client::new();

    db::insert_scan_log(pool, scan_job_id, "info", Some("zap"),
        &format!("Starting ZAP spider on {}", target)).await;

    // ── Spider ──────────────────────────────────────────────────────────
    let spider_url = format!("{}/JSON/spider/action/scan/?apikey={}&url={}&maxChildren=10&recurse=true",
        zap_url, api_key, target);
    let spider_resp = client.get(&spider_url).send().await;
    let spider_scan_id = match spider_resp {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(body) => body["scan"].as_str().map(|s| s.to_string()),
            Err(e) => {
                db::insert_scan_log(pool, scan_job_id, "error", Some("zap"),
                    &format!("ZAP spider returned an unparseable response: {}", e)).await;
                None
            }
        },
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("zap"),
                &format!("ZAP spider failed: {}", e)).await;
            return vec![];
        }
    };

    if let Some(ref id) = spider_scan_id {
        wait_for_progress(&client, &zap_url, &api_key, "spider", id, SPIDER_POLL_SECS, SPIDER_MAX_POLLS, pool, scan_job_id).await;
    } else {
        db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
            "ZAP did not return a spider scan ID — nothing to crawl, skipping to active scan").await;
    }

    db::insert_scan_log(pool, scan_job_id, "info", Some("zap"), "Spider complete, starting active scan").await;

    // ── Active scan ─────────────────────────────────────────────────────
    let ascan_url = format!("{}/JSON/ascan/action/scan/?apikey={}&url={}&recurse=true",
        zap_url, api_key, target);
    let ascan_resp = client.get(&ascan_url).send().await;
    let ascan_scan_id = match ascan_resp {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(body) => body["scan"].as_str().map(|s| s.to_string()),
            Err(e) => {
                db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                    &format!("ZAP active scan returned an unparseable response: {}", e)).await;
                None
            }
        },
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                &format!("ZAP active scan request failed: {}", e)).await;
            None
        }
    };

    if let Some(ref id) = ascan_scan_id {
        wait_for_progress(&client, &zap_url, &api_key, "ascan", id, ASCAN_POLL_SECS, ASCAN_MAX_POLLS, pool, scan_job_id).await;
    } else {
        db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
            "ZAP did not start an active scan (nothing crawlable/parameterized found) — skipping to results").await;
    }

    // ── Alerts ──────────────────────────────────────────────────────────
    let alerts_url = format!("{}/JSON/alert/view/alerts/?apikey={}&baseurl={}&start=0&count=500",
        zap_url, api_key, target);

    let mut findings = Vec::new();
    match client.get(&alerts_url).send().await {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(body) => {
                if let Some(alerts) = body["alerts"].as_array() {
                    for alert in alerts {
                        let risk = alert["risk"].as_str().unwrap_or("Informational");
                        let severity = match risk {
                            "High" => "high",
                            "Medium" => "medium",
                            "Low" => "low",
                            _ => "info",
                        };
                        let cwe = alert["cweid"].as_str()
                            .filter(|s| !s.is_empty() && *s != "-1")
                            .map(|s| format!("CWE-{}", s));

                        findings.push(ToolFinding {
                            tool: "zap".into(),
                            severity: severity.into(),
                            title: alert["alert"].as_str().unwrap_or("ZAP Alert").into(),
                            description: alert["description"].as_str().map(|s| s.to_string()),
                            file_path: alert["url"].as_str().map(|s| s.to_string()),
                            line_number: None,
                            cwe_id: cwe,
                            cvss_score: None,
                            recommendation: alert["solution"].as_str().map(|s| s.to_string()),
                            issue_type: None,
                        });
                    }
                }
            }
            Err(e) => {
                db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                    &format!("Failed to parse ZAP alerts: {}", e)).await;
            }
        },
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                &format!("Failed to fetch ZAP alerts: {}", e)).await;
        }
    }

    findings
}

/// Polls a ZAP `<module>/view/status/?scanId=<id>` endpoint until it reports 100%,
/// the response stops looking like a valid progress payload, or `max_polls` is hit.
/// This is the fix for the hang: every exit path is bounded, and an error/missing
/// `status` field ends the wait instead of looping forever.
#[allow(clippy::too_many_arguments)]
async fn wait_for_progress(
    client: &reqwest::Client,
    zap_url: &str,
    api_key: &str,
    module: &str,
    scan_id: &str,
    poll_secs: u64,
    max_polls: u32,
    pool: &DbPool,
    scan_job_id: i64,
) {
    let mut polls = 0u32;
    loop {
        let status_url = format!("{}/JSON/{}/view/status/?apikey={}&scanId={}", zap_url, module, api_key, scan_id);
        match client.get(&status_url).send().await {
            Ok(r) => match r.json::<serde_json::Value>().await {
                Ok(body) => match body["status"].as_str() {
                    Some(s) => {
                        if s == "100" {
                            return;
                        }
                    }
                    None => {
                        db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                            &format!("ZAP {} status came back without a progress field ({}) — treating as finished", module, body)).await;
                        return;
                    }
                },
                Err(e) => {
                    db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                        &format!("ZAP {} status response unparseable ({}) — treating as finished", module, e)).await;
                    return;
                }
            },
            Err(e) => {
                db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                    &format!("ZAP {} status request failed ({}) — treating as finished", module, e)).await;
                return;
            }
        }

        polls += 1;
        if polls >= max_polls {
            db::insert_scan_log(pool, scan_job_id, "warn", Some("zap"),
                &format!("ZAP {} did not finish within the time budget — moving on with partial results", module)).await;
            return;
        }

        tokio::time::sleep(std::time::Duration::from_secs(poll_secs)).await;
    }
}
