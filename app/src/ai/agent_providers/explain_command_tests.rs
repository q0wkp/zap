use super::*;

#[test]
fn parses_risk_line_and_details() {
    let reply = "RISK: HIGH - deletes the build directory\n\
                 - `rm -rf build` removes `build` and everything in it\n";
    assert_eq!(
        parse_reply(reply),
        Some(CommandExplanation {
            risk: Some(CommandRisk::High),
            risk_reason: Some("deletes the build directory".to_owned()),
            details: "- `rm -rf build` removes `build` and everything in it".to_owned(),
        })
    );
}

#[test]
fn parses_risk_line_variants() {
    for (line, risk, reason) in [
        (
            "RISK: LOW - only reads files",
            CommandRisk::Low,
            Some("only reads files"),
        ),
        (
            "**Risk: medium** — installs a package",
            CommandRisk::Medium,
            Some("installs a package"),
        ),
        ("risk:high", CommandRisk::High, None),
        ("RISK: HIGH: 删除数据", CommandRisk::High, Some("删除数据")),
    ] {
        assert_eq!(
            parse_risk_line(line),
            Some((risk, reason.map(str::to_owned))),
            "{line}"
        );
    }
    for line in [
        "RISK: unknown",
        "Risky command",
        "- `ls` lists files",
        "RISK",
    ] {
        assert_eq!(parse_risk_line(line), None, "{line}");
    }
}

#[test]
fn reply_without_risk_line_is_kept_whole() {
    let reply = "\n\nThis lists the files.\n- `ls`\n";
    assert_eq!(
        parse_reply(reply),
        Some(CommandExplanation {
            risk: None,
            risk_reason: None,
            details: "This lists the files.\n- `ls`".to_owned(),
        })
    );
}

#[test]
fn empty_reply_is_none() {
    assert_eq!(parse_reply(""), None);
    assert_eq!(parse_reply("  \n\n"), None);
    assert_eq!(parse_reply("<think>only reasoning</think>\n"), None);
}

#[test]
fn reasoning_blocks_are_removed() {
    assert_eq!(
        strip_reasoning("<think>a\nb</think>RISK: LOW<reasoning>c</reasoning>"),
        "RISK: LOW"
    );
    assert_eq!(
        strip_reasoning("RISK: LOW\n<think>unfinished"),
        "RISK: LOW\n"
    );
}

#[test]
fn long_commands_are_cut_and_marked() {
    let short = user_prompt("ls -la");
    assert!(short.contains("<command>\nls -la\n</command>"));
    assert!(!short.contains("cut short"));

    let long_command = "é".repeat(MAX_COMMAND_CHARS + 10);
    let long = user_prompt(&long_command);
    assert!(long.contains(&"é".repeat(MAX_COMMAND_CHARS)));
    assert!(!long.contains(&"é".repeat(MAX_COMMAND_CHARS + 1)));
    assert!(long.ends_with("The command was cut short here: the rest is not shown."));
}
