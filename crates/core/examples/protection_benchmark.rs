use std::time::Instant;
use workspace_core::protection::{Protection, STARTER_FILTERS};

const REQUESTS: usize = 25_000;
const REPEATS: usize = 4;

fn ruleset(size: usize) -> (String, Vec<String>, &'static str) {
    if size == 3 {
        return (
            STARTER_FILTERS.to_string(),
            vec![
                "doubleclick.net".into(),
                "googlesyndication.com".into(),
                "googleadservices.com".into(),
            ],
            "bundled-starter",
        );
    }
    let hosts: Vec<_> = (0..size)
        .map(|index| format!("ad{index}.bench.invalid"))
        .collect();
    let mut text = String::from("[Adblock Plus 2.0]\n! Generated local benchmark rules\n");
    for host in &hosts {
        text.push_str("||");
        text.push_str(host);
        text.push_str("^\n");
    }
    (text, hosts, "synthetic-domain-rules")
}

fn measure(engine: &Protection, hosts: &[String], enabled: bool) -> serde_json::Value {
    let source = "https://publisher.bench.invalid/article";
    let mut blocked = 0usize;
    let start = Instant::now();
    for repeat in 0..REPEATS {
        for index in 0..REQUESTS {
            let absolute = index + repeat * REQUESTS;
            let (url, should_block) = if absolute.is_multiple_of(2) {
                let host = &hosts[(absolute / 2) % hosts.len()];
                (format!("https://{host}/resource.js?case={absolute}"), true)
            } else {
                (
                    format!("https://static.publisher.bench.invalid/resource.js?case={absolute}"),
                    false,
                )
            };
            let result = engine.check(&url, source, "script", enabled);
            blocked += usize::from(result.blocked);
            debug_assert_eq!(result.blocked, enabled && should_block);
        }
    }
    let elapsed = start.elapsed();
    let total = REQUESTS * REPEATS;
    let expected_blocked = if enabled { total / 2 } else { 0 };
    assert_eq!(
        blocked, expected_blocked,
        "unexpected request matching result"
    );
    serde_json::json!({
        "enabled": enabled,
        "requests": total,
        "blocked": blocked,
        "elapsedMs": elapsed.as_secs_f64() * 1000.0,
        "requestsPerSecond": total as f64 / elapsed.as_secs_f64(),
        "meanMicrosecondsPerRequest": elapsed.as_secs_f64() * 1_000_000.0 / total as f64,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    for size in [3usize, 1_000, 10_000] {
        let (text, hosts, source) = ruleset(size);
        let start = Instant::now();
        let engine = Protection::compile(&text)?;
        let compile_ms = start.elapsed().as_secs_f64() * 1000.0;
        results.push(serde_json::json!({
            "rules": size,
            "source": source,
            "ruleTextBytes": text.len(),
            "compileMs": compile_ms,
            "disabled": measure(&engine, &hosts, false),
            "enabled": measure(&engine, &hosts, true),
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "passed",
            "benchmark": "workspace-core protection request matching",
            "requestsPerMode": REQUESTS * REPEATS,
            "results": results,
        }))?
    );
    Ok(())
}
