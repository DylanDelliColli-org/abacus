use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "abacus-merge-jsonl-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn merge_jsonl_subcommand_overwrites_ours_with_the_merged_jsonl() {
    let temp = TempDir::new();
    let ours = temp.file("ours.jsonl");
    let base = temp.file("base.jsonl");
    let theirs = temp.file("theirs.jsonl");
    std::fs::write(
        &ours,
        concat!(
            r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:02Z","side":"ours"}"#,
            "\n",
            r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:04Z","side":"ours"}"#,
            "\n",
            r#"{"id":"ab-only-ours","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        ),
    )
    .unwrap();
    std::fs::write(
        &base,
        concat!(
            r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:01Z","side":"base"}"#,
            "\n",
            r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:01Z","side":"base"}"#,
            "\n",
            r#"{"id":"ab-only-base","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        ),
    )
    .unwrap();
    std::fs::write(
        &theirs,
        concat!(
            r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:03Z","side":"theirs"}"#,
            "\n",
            r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:03Z","side":"theirs"}"#,
            "\n",
            r#"{"id":"ab-only-theirs","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["merge-jsonl"])
        .arg(&ours)
        .arg(&base)
        .arg(&theirs)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&ours).unwrap(),
        concat!(
            r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:03Z","side":"theirs"}"#,
            "\n",
            r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:04Z","side":"ours"}"#,
            "\n",
            r#"{"id":"ab-only-base","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
            r#"{"id":"ab-only-ours","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
            r#"{"id":"ab-only-theirs","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        )
    );
}

#[test]
fn merge_jsonl_subcommand_fails_without_overwriting_ours_on_bad_input() {
    let temp = TempDir::new();
    let ours = temp.file("ours.jsonl");
    let base = temp.file("base.jsonl");
    let theirs = temp.file("theirs.jsonl");
    let original_ours = concat!(
        r#"{"id":"ab-ours","updated_at":"2026-08-13T10:00:01Z"}"#,
        "\n",
    );
    std::fs::write(&ours, original_ours).unwrap();
    std::fs::write(
        &base,
        concat!(
            r#"{"id":"ab-base","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        ),
    )
    .unwrap();
    std::fs::write(&theirs, "{not json}\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["merge-jsonl"])
        .arg(&ours)
        .arg(&base)
        .arg(&theirs)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("theirs"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(&ours).unwrap(), original_ours);
}
