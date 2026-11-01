use super::attach_command;

#[test]
fn finds_the_program_given_to_attach() {
    assert_eq!(attach_command("zmx attach dev claude"), Some("claude"));
    assert_eq!(
        attach_command("zmx a dev claude --resume"),
        Some("claude --resume")
    );
    assert_eq!(
        attach_command("  /usr/bin/zmx   a  dev   nvim  "),
        Some("nvim")
    );
    assert_eq!(
        attach_command("zmx attach --min-window dev claude"),
        Some("claude")
    );
}

#[test]
fn ignores_attach_without_a_program() {
    assert_eq!(attach_command("zmx attach dev"), None);
    assert_eq!(attach_command("zmx a"), None);
    assert_eq!(attach_command("zmx"), None);
}

#[test]
fn ignores_other_commands() {
    assert_eq!(attach_command("zmx run dev claude"), None);
    assert_eq!(attach_command("zmxx attach dev claude"), None);
    assert_eq!(attach_command("echo zmx attach dev claude"), None);
    assert_eq!(attach_command(""), None);
}
