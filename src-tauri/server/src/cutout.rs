//! Вырезание фигурок из админки, только на своей машине (CLAUDE.md § 21).
//!
//! Модель вырезания (BiRefNet, Segment Anything) стоит у автора на компьютере,
//! в `tools/cutout`, а не на сервере сайта. Поэтому ручки подключаются только
//! при локальном `PUBLIC_URL` (`api::router`): снаружи их нет вовсе, и
//! страница админки по отсутствию ответа понимает, что показывать нечего.
//!
//! Здесь нет ни обработки, ни своего окна: сервер лишь запускает программу
//! `tools/cutout/cutout` (она сама заводит окружение и поднимает страницу на
//! 127.0.0.1) и сообщает адрес этой страницы. Саму страницу админка
//! показывает рамкой.

use axum::Json;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;
use tokio::process::{Child, Command};

/// Последний запуск, сделанный отсюда. Живой процесс значит «запускается или
/// работает»: второе нажатие не должно поднимать второй, пока первый ставит
/// окружение (первый запуск занимает минуты).
static LAUNCH: Mutex<Option<Launch>> = Mutex::new(None);

struct Launch {
    child: Child,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CutoutStatus {
    /// Программа лежит в `tools/cutout`.
    installed: bool,
    /// Окно отвечает; `url` — его адрес.
    running: bool,
    url: Option<String>,
    /// Запущено отсюда и ещё не ответило (первый запуск ставит зависимости).
    starting: bool,
    /// Запуск оборвался; хвост журнала запуска.
    error: Option<String>,
}

fn tool_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/cutout")
}

fn home_dir() -> PathBuf {
    match dotenvy::var("CUTOUT_HOME") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cutout"),
    }
}

fn launch_log() -> PathBuf {
    home_dir().join("admin-launch.log")
}

/// Порт из `server.json`, который пишет сама программа.
fn port_of(server_json: &str) -> Option<u16> {
    let v: serde_json::Value = serde_json::from_str(server_json).ok()?;
    u16::try_from(v.get("port")?.as_u64()?).ok().filter(|p| *p > 0)
}

/// Адрес окна, если оно живо. Запрос `/api/ping` заодно считается для
/// программы обращением страницы: пока админка открыта и спрашивает, она не
/// выключается по бездействию.
async fn live_url() -> Option<String> {
    let text = std::fs::read_to_string(home_dir().join("server.json")).ok()?;
    let port = port_of(&text)?;
    let url = format!("http://127.0.0.1:{port}/");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;
    let ok = client
        .get(format!("{url}api/ping"))
        .send()
        .await
        .ok()?
        .status()
        .is_success();
    ok.then_some(url)
}

fn tail_of_log() -> Option<String> {
    let text = std::fs::read_to_string(launch_log()).ok()?;
    let lines: Vec<&str> = text.lines().rev().take(6).collect();
    let tail = lines.into_iter().rev().collect::<Vec<_>>().join("\n");
    (!tail.trim().is_empty()).then_some(tail)
}

/// Что стало с запущенным отсюда процессом: ещё идёт / оборвался с причиной.
fn launch_state() -> (bool, Option<String>) {
    let mut guard = LAUNCH.lock().unwrap_or_else(|e| e.into_inner());
    let Some(launch) = guard.as_mut() else {
        return (false, None);
    };
    match launch.child.try_wait() {
        Ok(None) => (true, None),
        Ok(Some(status)) if status.success() => {
            *guard = None;
            (false, None)
        }
        Ok(Some(status)) => {
            *guard = None;
            let why = tail_of_log().unwrap_or_else(|| format!("программа завершилась: {status}"));
            (false, Some(why))
        }
        Err(_) => (false, None),
    }
}

async fn status_now() -> CutoutStatus {
    let installed = tool_dir().join("cutout").is_file();
    let url = live_url().await;
    let (alive, error) = launch_state();
    CutoutStatus {
        installed,
        running: url.is_some(),
        starting: url.is_none() && alive,
        error: if url.is_some() { None } else { error },
        url,
    }
}

pub async fn status() -> Json<CutoutStatus> {
    Json(status_now().await)
}

/// Запустить программу, если она не работает. Ответ — то же состояние, что у
/// `status`; готовности ждёт страница, опрашивая его.
pub async fn start() -> Json<CutoutStatus> {
    let now = status_now().await;
    if now.running || now.starting || !now.installed {
        return Json(now);
    }
    let log = std::fs::create_dir_all(home_dir())
        .and_then(|_| std::fs::File::create(launch_log()));
    let (out, err) = match log.and_then(|f| f.try_clone().map(|g| (f, g))) {
        Ok((a, b)) => (Stdio::from(a), Stdio::from(b)),
        Err(_) => (Stdio::null(), Stdio::null()),
    };
    let spawned = Command::new(tool_dir().join("cutout"))
        // Страницу открывает админка, а не отдельная вкладка браузера.
        .env("CUTOUT_NO_BROWSER", "1")
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        // Своя группа процессов: перезапуск сервера (cargo watch) не должен
        // убивать окно вместе с собой.
        .process_group(0)
        .spawn();
    match spawned {
        Ok(child) => {
            *LAUNCH.lock().unwrap_or_else(|e| e.into_inner()) = Some(Launch { child });
            Json(CutoutStatus {
                starting: true,
                ..now
            })
        }
        Err(e) => Json(CutoutStatus {
            error: Some(format!("не удалось запустить tools/cutout/cutout: {e}")),
            ..now
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{port_of, start, status_now};

    #[test]
    fn reads_the_port_the_program_wrote() {
        assert_eq!(port_of(r#"{"port": 8765, "pid": 41}"#), Some(8765));
    }

    #[test]
    fn rejects_what_is_not_a_port() {
        assert_eq!(port_of(""), None);
        assert_eq!(port_of(r#"{"pid": 41}"#), None);
        assert_eq!(port_of(r#"{"port": 0}"#), None);
        assert_eq!(port_of(r#"{"port": 70000}"#), None);
    }

    /// Настоящий запуск `tools/cutout/cutout` со своим `CUTOUT_HOME`. Нужен
    /// Python и окружение программы, поэтому только вручную:
    /// `cargo test --lib cutout -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn starts_the_program_and_finds_its_page() {
        let home = std::env::temp_dir().join(format!("cutout-test-{}", std::process::id()));
        // SAFETY: тест один в своём процессе меняет окружение, до запуска потоков.
        unsafe { std::env::set_var("CUTOUT_HOME", &home) };

        assert!(!status_now().await.running);
        assert!(start().await.0.starting);
        // Повторное нажатие второго процесса не поднимает.
        assert!(start().await.0.starting);

        let mut url = None;
        for _ in 0..120 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if let Some(u) = status_now().await.url {
                url = Some(u);
                break;
            }
        }
        let url = url.expect("программа не ответила за две минуты");
        let page = reqwest::get(&url).await.unwrap().text().await.unwrap();
        assert!(page.contains("<html"));

        let pid = std::fs::read_to_string(home.join("server.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["pid"].as_u64())
            .expect("pid в server.json");
        let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
        let _ = std::fs::remove_dir_all(&home);
    }
}
