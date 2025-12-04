use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{routing::get, Json, Router};
use chrono::{DateTime, FixedOffset};
use clap::Parser;
use glob::glob;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::{signal, task, time::sleep};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Config {
    inputs: Vec<String>,
    pattern: Option<String>,
    since: Option<String>,
    until: Option<String>,
    date_format: Option<String>,
    follow: Option<bool>,
    serve: Option<u16>,
    export_html: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            pattern: None,
            since: None,
            until: None,
            date_format: None,
            follow: Some(false),
            serve: None,
            export_html: None,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Loglyzer - analyseur de logs avec suivi temps réel", long_about = None)]
struct Cli {
    /// Fichiers ou glob (ex: *.log)
    #[arg(required = true)]
    inputs: Vec<String>,

    /// Regex de parsing (nommez vos groupes: ip, url, status, time)
    #[arg(long)]
    pattern: Option<String>,

    /// Filtrer depuis cette date (ex: "2024-01-15 10:00")
    #[arg(long)]
    since: Option<String>,

    /// Filtrer jusqu'à cette date
    #[arg(long)]
    until: Option<String>,

    /// Format date pour parser le champ time (defaut Apache: "%d/%b/%Y:%H:%M:%S %z")
    #[arg(long)]
    date_format: Option<String>,

    /// Suivi temps réel (tail -f)
    #[arg(long, default_value_t = false)]
    follow: bool,

    /// Lancer un serveur web sur ce port
    #[arg(long)]
    serve: Option<u16>,

    /// Export HTML vers ce fichier
    #[arg(long)]
    export_html: Option<String>,

    /// Fichier de config TOML (.loglyzer.toml)
    #[arg(long)]
    config: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LogEntry {
    raw: String,
    ip: Option<String>,
    url: Option<String>,
    status: Option<u16>,
    time: Option<DateTime<FixedOffset>>,
}

/// Extension point for formats : implémentez ce trait et branchez votre parser.
trait LogParser: Send + Sync {
    fn parse(&self, line: &str) -> Option<LogEntry>;
}

#[derive(Clone)]
struct RegexParser {
    re: Regex,
    date_fmt: String,
}

impl LogParser for RegexParser {
    fn parse(&self, line: &str) -> Option<LogEntry> {
        let caps = self.re.captures(line)?;
        let ip = caps.name("ip").map(|m| m.as_str().to_string());
        let url = caps.name("url").map(|m| m.as_str().to_string());
        let status = caps
            .name("status")
            .and_then(|m| m.as_str().parse::<u16>().ok());
        let time = caps
            .name("time")
            .and_then(|m| parse_time(m.as_str(), &self.date_fmt));

        Some(LogEntry {
            raw: line.to_string(),
            ip,
            url,
            status,
            time,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
struct Summary {
    total: usize,
    by_status: HashMap<u16, usize>,
}

fn load_config(path: Option<&str>) -> Config {
    let candidate = path.map(PathBuf::from).or_else(|| {
        let p = PathBuf::from(".loglyzer.toml");
        if p.exists() {
            Some(p)
        } else {
            None
        }
    });

    if let Some(p) = candidate {
        match std::fs::read_to_string(&p) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    } else {
        Config::default()
    }
}

fn merge_config(cfg: Config, cli: &Cli) -> Config {
    Config {
        inputs: if !cli.inputs.is_empty() {
            cli.inputs.clone()
        } else {
            cfg.inputs
        },
        pattern: cli.pattern.clone().or(cfg.pattern),
        since: cli.since.clone().or(cfg.since),
        until: cli.until.clone().or(cfg.until),
        date_format: cli.date_format.clone().or(cfg.date_format),
        follow: Some(cli.follow || cfg.follow.unwrap_or(false)),
        serve: cli.serve.or(cfg.serve),
        export_html: cli.export_html.clone().or(cfg.export_html),
    }
}

fn collect_paths(patterns: &[String]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for pat in patterns {
        if pat.contains('*') || pat.contains('?') || pat.contains('[') {
            if let Ok(entries) = glob(pat) {
                for e in entries.flatten() {
                    paths.push(e);
                }
            }
        } else {
            paths.push(PathBuf::from(pat));
        }
    }
    paths
}

fn build_regex(pattern: Option<String>) -> Regex {
    let default = r#"(?P<ip>\S+) [^ ]+ [^ ]+ \[(?P<time>[^\]]+)\] \"(?:GET|POST|PUT|DELETE|PATCH|OPTIONS|HEAD) (?P<url>[^" ]+)[^\"]*\" (?P<status>\d{3})"#.to_string();
    let pat = pattern.unwrap_or(default);
    Regex::new(&pat).expect("invalid regex pattern")
}

fn parse_time(s: &str, fmt: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_str(s, fmt).ok()
}

fn within_window(
    entry: &LogEntry,
    since: &Option<DateTime<FixedOffset>>,
    until: &Option<DateTime<FixedOffset>>,
) -> bool {
    if let Some(t) = entry.time {
        if let Some(s) = since {
            if t < *s {
                return false;
            }
        }
        if let Some(u) = until {
            if t > *u {
                return false;
            }
        }
    }
    true
}

fn load_entries(
    paths: &[PathBuf],
    parser: &dyn LogParser,
    since: &Option<DateTime<FixedOffset>>,
    until: &Option<DateTime<FixedOffset>>,
) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    for p in paths {
        if let Ok(f) = File::open(p) {
            let reader = BufReader::new(f);
            for line in reader.lines().flatten() {
                if let Some(e) = parser.parse(&line) {
                    if within_window(&e, since, until) {
                        entries.push(e);
                    }
                }
            }
        }
    }
    entries
}

fn summarize(entries: &[LogEntry]) -> Summary {
    let mut by_status = HashMap::new();
    for e in entries {
        if let Some(s) = e.status {
            *by_status.entry(s).or_insert(0) += 1;
        }
    }
    Summary {
        total: entries.len(),
        by_status,
    }
}

fn export_html(path: &str, entries: &[LogEntry], summary: &Summary) -> std::io::Result<()> {
    let mut html = String::new();
    html.push_str(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Loglyzer</title></head><body>",
    );
    html.push_str(&format!(
        "<h1>Loglyzer</h1><p>Total: {}</p><h2>Par status</h2><ul>",
        summary.total
    ));
    for (status, count) in summary.by_status.iter() {
        html.push_str(&format!("<li>{status}: {count}</li>"));
    }
    html.push_str("</ul><h2>Dernières entrées</h2><pre>");
    for e in entries.iter().rev().take(50) {
        html.push_str(&format!("{}\n", e.raw));
    }
    html.push_str("</pre></body></html>");
    std::fs::write(path, html)
}

async fn serve(port: u16, state: Arc<Mutex<Vec<LogEntry>>>) {
    let app = Router::new().route(
        "/data",
        get(move || {
            let state = state.clone();
            async move {
                let data = state.lock().unwrap().clone();
                Json(data)
            }
        }),
    );

    let addr = format!("0.0.0.0:{port}");
    println!("Serving dashboard JSON on http://{}/data", addr);
    axum::serve(tokio::net::TcpListener::bind(&addr).await.unwrap(), app)
        .with_graceful_shutdown(async {
            let _ = signal::ctrl_c().await;
        })
        .await
        .ok();
}

async fn follow_file(
    path: PathBuf,
    parser: RegexParser,
    since: Option<DateTime<FixedOffset>>,
    until: Option<DateTime<FixedOffset>>,
    state: Arc<Mutex<Vec<LogEntry>>>,
) {
    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot open {}: {e}", path.display());
            return;
        }
    };
    let mut pos = file.seek(SeekFrom::End(0)).unwrap_or(0);
    loop {
        file.seek(SeekFrom::Start(pos)).ok();
        let mut reader = BufReader::new(&file);
        let mut buf = String::new();
        while let Ok(bytes) = reader.read_line(&mut buf) {
            if bytes == 0 {
                break;
            }
            if let Some(entry) = parser.parse(buf.trim_end_matches('\n')) {
                if within_window(&entry, &since, &until) {
                    println!("{}", entry.raw);
                    state.lock().unwrap().push(entry);
                }
            }
            buf.clear();
        }
        pos = file.seek(SeekFrom::Current(0)).unwrap_or(pos);
        sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let cfg_file = cli.config.clone();
    let base_cfg = load_config(cfg_file.as_deref());
    let cfg = merge_config(base_cfg, &cli);

    let re = build_regex(cfg.pattern.clone());
    let date_fmt = cfg
        .date_format
        .clone()
        .unwrap_or_else(|| "%d/%b/%Y:%H:%M:%S %z".to_string());
    let parser = RegexParser {
        re,
        date_fmt: date_fmt.clone(),
    };

    let since = cfg
        .since
        .as_deref()
        .and_then(|s| DateTime::parse_from_str(s, "%Y-%m-%d %H:%M").ok());
    let until = cfg
        .until
        .as_deref()
        .and_then(|s| DateTime::parse_from_str(s, "%Y-%m-%d %H:%M").ok());

    let paths = collect_paths(&cfg.inputs);
    let state: Arc<Mutex<Vec<LogEntry>>> = Arc::new(Mutex::new(Vec::new()));

    if cfg.follow.unwrap_or(false) {
        let mut handles = Vec::new();
        for p in paths {
            let since_cl = since.clone();
            let until_cl = until.clone();
            let st = state.clone();
            handles.push(task::spawn(follow_file(
                p,
                parser.clone(),
                since_cl,
                until_cl,
                st,
            )));
        }

        if let Some(port) = cfg.serve {
            let st = state.clone();
            task::spawn(serve(port, st));
        }

        futures::future::join_all(handles).await;
        return;
    }

    let entries = load_entries(&paths, &parser, &since, &until);
    let summary = summarize(&entries);

    println!("Total: {}", summary.total);
    println!("Par status:");
    for (s, c) in summary.by_status.iter() {
        println!("  {s}: {c}");
    }

    if let Some(path) = cfg.export_html.as_deref() {
        if let Err(e) = export_html(path, &entries, &summary) {
            eprintln!("Export HTML échoué: {e}");
        } else {
            println!("Export HTML -> {path}");
        }
    }

    if let Some(port) = cfg.serve {
        *state.lock().unwrap() = entries.clone();
        serve(port, state.clone()).await;
    }
}
