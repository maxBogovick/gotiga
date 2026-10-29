//! Вкладка «Механизмы» описывает проходы по ключам, а ключи называет `main.rs`.
//! Тест сверяет их, чтобы новый проход не остался без описания (а описание — без прохода).
//!
//! Файл с описаниями лежит в клиенте (`src/lib/admin/mechanisms.ts`). В образе сервера его
//! нет, поэтому без него тест молча пропускается: проверка нужна там, где есть оба дерева.

use std::path::PathBuf;

/// Ключи из вызовов `track_job("ключ", …)`: первая строка в кавычках после имени вызова,
/// даже если вызов разнесён на несколько строк.
fn job_keys(source: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut rest = source;
    while let Some(i) = rest.find("track_job(") {
        rest = &rest[i + "track_job(".len()..];
        let Some(open) = rest.find('"') else { break };
        // Между скобкой и ключом ничего, кроме пробелов и переводов строки.
        if !rest[..open].trim().is_empty() {
            continue;
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        keys.push(after[..close].to_string());
    }
    keys
}

fn registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/lib/admin/mechanisms.ts")
}

#[test]
fn every_tracked_job_is_described_in_the_mechanisms_tab() {
    let Ok(registry) = std::fs::read_to_string(registry_path()) else {
        eprintln!("mechanisms.ts не найден — проверка пропущена (нет дерева клиента)");
        return;
    };
    let main = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
        .expect("main.rs");
    let keys = job_keys(&main);
    assert!(keys.len() >= 15, "ключей проходов слишком мало, разбор сломался: {keys:?}");

    let missing: Vec<_> = keys
        .iter()
        .filter(|k| !registry.contains(&format!("'{k}'")))
        .collect();
    assert!(
        missing.is_empty(),
        "проходы есть в main.rs, но не описаны в src/lib/admin/mechanisms.ts: {missing:?}"
    );

    // И наоборот: описанный проход, которого нет в коде, вкладка показывала бы вечно «не шёл».
    let in_registry: Vec<&str> = registry
        .split("jobs: [")
        .skip(1)
        .filter_map(|chunk| chunk.split(']').next())
        .flat_map(|list| list.split(','))
        .map(|k| k.trim().trim_matches('\'').trim())
        .filter(|k| !k.is_empty())
        .collect();
    let stale: Vec<_> = in_registry.iter().filter(|k| !keys.iter().any(|x| x == *k)).collect();
    assert!(
        stale.is_empty(),
        "в mechanisms.ts описаны проходы, которых нет в main.rs: {stale:?}"
    );
}

#[test]
fn job_keys_are_read_across_lines() {
    let src = r#"
        svc.track_job("a_one", 300, x, |_| None)
        svc.track_job(
            "b_two",
            1, y, |_| None)
        let track_job_x = 1;
    "#;
    assert_eq!(job_keys(src), vec!["a_one", "b_two"]);
}
