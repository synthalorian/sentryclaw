use axum::{
    extract::State,
    response::Html,
    Json,
};
use serde::Serialize;

use crate::{AppState, db::Database};

#[derive(Serialize)]
pub struct ApiStats {
    total_reviews: i64,
    approved: i64,
    request_changes: i64,
    commented: i64,
    avg_inline_comments: f64,
}

pub async fn dashboard_handler(State(state): State<AppState>) -> Html<String> {
    let db = state.database.clone();
    let refresh = state.config.dashboard_config().refresh_seconds;

    match render_dashboard(&db, refresh).await {
        Ok(html) => Html(html),
        Err(e) => Html(format!("<h1>Dashboard Error</h1><p>{}</p>", e)),
    }
}

pub async fn stats_api_handler(State(state): State<AppState>) -> Json<ApiStats> {
    let db = state.database.clone();

    match db.get_stats().await {
        Ok(stats) => Json(ApiStats {
            total_reviews: stats.total_reviews,
            approved: stats.approved,
            request_changes: stats.request_changes,
            commented: stats.commented,
            avg_inline_comments: stats.avg_inline_comments,
        }),
        Err(_) => Json(ApiStats {
            total_reviews: 0,
            approved: 0,
            request_changes: 0,
            commented: 0,
            avg_inline_comments: 0.0,
        }),
    }
}

async fn render_dashboard(db: &Database, refresh_seconds: u64) -> anyhow::Result<String> {
    let stats = db.get_stats().await?;
    let recent = db.get_recent_reviews(50).await?;

    let mut rows = String::new();
    for review in &recent {
        let emoji = match review.verdict.as_str() {
            "Approve" => "✅",
            "RequestChanges" => "❌",
            _ => "💬",
        };
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}#{}</td><td>{} {}</td><td>{}</td><td>{}</td></tr>\n",
            review.created_at.format("%Y-%m-%d %H:%M"),
            review.provider,
            review.repo,
            review.pr_number,
            emoji,
            review.verdict,
            review.inline_count,
            escape_html(&review.summary).chars().take(100).collect::<String>()
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>SentryShark Dashboard</title>
    <style>
        :root {{ --bg: #0d1117; --card: #161b22; --border: #30363d; --text: #c9d1d9; --accent: #58a6ff; --success: #3fb950; --danger: #f85149; --warn: #d29922; }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif; background: var(--bg); color: var(--text); line-height: 1.6; }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 2rem; }}
        header {{ margin-bottom: 2rem; }}
        header h1 {{ font-size: 2rem; display: flex; align-items: center; gap: 0.5rem; }}
        .stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-bottom: 2rem; }}
        .stat-card {{ background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 1.5rem; }}
        .stat-card h3 {{ font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; color: #8b949e; margin-bottom: 0.5rem; }}
        .stat-card .value {{ font-size: 2rem; font-weight: 700; }}
        .stat-card.success .value {{ color: var(--success); }}
        .stat-card.danger .value {{ color: var(--danger); }}
        .stat-card.warn .value {{ color: var(--warn); }}
        .stat-card.accent .value {{ color: var(--accent); }}
        table {{ width: 100%; border-collapse: collapse; background: var(--card); border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }}
        th, td {{ padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid var(--border); }}
        th {{ background: rgba(88, 166, 255, 0.1); font-weight: 600; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; }}
        tr:hover {{ background: rgba(255,255,255,0.03); }}
        .empty {{ text-align: center; padding: 3rem; color: #8b949e; }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🦈 SentryShark Dashboard</h1>
            <p>Code review analytics and history</p>
        </header>
        <div class="stats">
            <div class="stat-card accent">
                <h3>Total Reviews</h3>
                <div class="value">{}</div>
            </div>
            <div class="stat-card success">
                <h3>Approved</h3>
                <div class="value">{}</div>
            </div>
            <div class="stat-card danger">
                <h3>Changes Requested</h3>
                <div class="value">{}</div>
            </div>
            <div class="stat-card warn">
                <h3>Commented</h3>
                <div class="value">{}</div>
            </div>
            <div class="stat-card accent">
                <h3>Avg Inline Comments</h3>
                <div class="value">{:.1}</div>
            </div>
        </div>
        <h2 style="margin-bottom:1rem">Recent Reviews</h2>
        <table>
            <thead>
                <tr><th>Time</th><th>Provider</th><th>PR/MR</th><th>Verdict</th><th>Inline</th><th>Summary</th></tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
    <script>
        setTimeout(() => location.reload(), {}000);
    </script>
</body>
</html>"#,
        stats.total_reviews,
        stats.approved,
        stats.request_changes,
        stats.commented,
        stats.avg_inline_comments,
        if rows.is_empty() { "<tr><td colspan=\"6\" class=\"empty\">No reviews yet. Start reviewing some code!</td></tr>".to_string() } else { rows },
        refresh_seconds
    );

    Ok(html)
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
