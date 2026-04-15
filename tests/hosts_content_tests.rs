use darp::os::{HOSTS_FOOTER, HOSTS_HEADER, build_hosts_content};

fn lines(entries: &[&str]) -> Vec<String> {
    entries.iter().map(|s| s.to_string()).collect()
}

#[test]
fn empty_existing_content() {
    let result = build_hosts_content("", &lines(&["0.0.0.0   hello.test\n"]));

    assert!(result.contains(HOSTS_HEADER));
    assert!(result.contains(HOSTS_FOOTER));
    assert!(result.contains("127.0.0.1   hello.test"));
}

#[test]
fn no_existing_block() {
    let existing = "127.0.0.1   localhost\n::1         localhost\n";
    let result = build_hosts_content(existing, &lines(&["0.0.0.0   app.test\n"]));

    // Original content preserved
    assert!(result.contains("127.0.0.1   localhost"));
    assert!(result.contains("::1         localhost"));
    // New block appended
    assert!(result.contains("127.0.0.1   app.test"));
    assert!(result.contains(HOSTS_HEADER));
}

#[test]
fn replaces_existing_block() {
    let existing = format!(
        "127.0.0.1   localhost\n{header}\n127.0.0.1   old.test\n{footer}\n::1   localhost\n",
        header = HOSTS_HEADER,
        footer = HOSTS_FOOTER,
    );

    let result = build_hosts_content(&existing, &lines(&["0.0.0.0   new.test\n"]));

    // Old entry removed
    assert!(!result.contains("old.test"));
    // New entry present
    assert!(result.contains("127.0.0.1   new.test"));
    // Surrounding content preserved
    assert!(result.contains("127.0.0.1   localhost"));
    assert!(result.contains("::1   localhost"));
}

#[test]
fn header_without_footer() {
    let existing = format!(
        "127.0.0.1   localhost\n{header}\n127.0.0.1   orphan.test\n",
        header = HOSTS_HEADER,
    );

    let result = build_hosts_content(&existing, &lines(&["0.0.0.0   fixed.test\n"]));

    // Should treat everything as "before" since footer is missing
    assert!(result.contains("127.0.0.1   fixed.test"));
    assert!(result.contains(HOSTS_HEADER));
    assert!(result.contains(HOSTS_FOOTER));
}

#[test]
fn crlf_normalization() {
    let existing = "127.0.0.1   localhost\r\n::1   localhost\r\n";
    let result = build_hosts_content(existing, &lines(&["0.0.0.0   app.test\n"]));

    assert!(!result.contains("\r\n")); // No CRLF in output
    assert!(result.contains("127.0.0.1   localhost"));
    assert!(result.contains("127.0.0.1   app.test"));
}

#[test]
fn empty_entries() {
    let result = build_hosts_content("127.0.0.1   localhost\n", &[]);

    // Block exists but has no host lines
    assert!(result.contains(HOSTS_HEADER));
    assert!(result.contains(HOSTS_FOOTER));
    assert!(result.contains("127.0.0.1   localhost"));
    // No extra 127.0.0.1 lines beyond the original localhost
    let count = result.matches("127.0.0.1").count();
    assert_eq!(count, 1);
}
