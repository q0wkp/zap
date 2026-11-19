//! The helper is run with `/bin/sh`, as ssh runs it, with `ZAP_ASKPASS_TTY` standing in for the
//! terminal: a file holding what the user would type, to which the helper appends its prompts.

use std::os::unix::fs::{FileTypeExt, PermissionsExt};

use command::blocking::Command;

use super::*;

struct Run {
    stdout: String,
    status: i32,
    tty: String,
}

fn run_helper(dir: &Path, env: &[(OsString, OsString)], prompt: &str, typed: &str) -> Run {
    run_helper_with(dir, env, prompt, typed, None)
}

fn run_helper_with(
    dir: &Path,
    env: &[(OsString, OsString)],
    prompt: &str,
    typed: &str,
    askpass_prompt: Option<&str>,
) -> Run {
    let tty = dir.join(format!("tty-{}", uuid::Uuid::new_v4()));
    fs::write(&tty, typed).unwrap();
    let helper = &env.iter().find(|(name, _)| name == "SSH_ASKPASS").unwrap().1;
    let mut command = Command::new("/bin/sh");
    command
        .arg(helper)
        .arg(prompt)
        .envs(env.iter().map(|(name, value)| (name, value)))
        .env("ZAP_ASKPASS_TTY", &tty);
    if let Some(kind) = askpass_prompt {
        command.env("SSH_ASKPASS_PROMPT", kind);
    }
    let output = command.output().unwrap();
    Run {
        stdout: String::from_utf8(output.stdout).unwrap(),
        status: output.status.code().unwrap(),
        tty: fs::read_to_string(&tty).unwrap(),
    }
}

fn fifo_of(env: &[(OsString, OsString)]) -> PathBuf {
    PathBuf::from(&env.iter().find(|(name, _)| name == "ZAP_ASKPASS_FIFO").unwrap().1)
}

#[test]
fn prepare_makes_a_private_helper_and_fifo() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("askpass");
    let (channel, env) = AskpassChannel::prepare_in(&dir).unwrap();

    let names: Vec<_> = env.iter().map(|(name, _)| name.to_str().unwrap()).collect();
    assert_eq!(names, ["SSH_ASKPASS", "SSH_ASKPASS_REQUIRE", "ZAP_ASKPASS_FIFO"]);
    assert_eq!(env[1].1, "force");
    assert_eq!(fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
    let helper = PathBuf::from(&env[0].1);
    assert_eq!(fs::read_to_string(&helper).unwrap(), HELPER_SCRIPT);
    assert_eq!(fs::metadata(&helper).unwrap().permissions().mode() & 0o777, 0o700);
    let fifo = fifo_of(&env);
    let meta = fs::metadata(&fifo).unwrap();
    assert!(meta.file_type().is_fifo());
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);

    drop(channel);
    assert!(!fifo.exists(), "an unserved FIFO is removed with its channel");
}

#[test]
fn a_directory_others_can_read_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("askpass");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(AskpassChannel::prepare_in(&dir).is_err());
}

#[test]
fn the_password_prompt_gets_the_secret_once_then_the_terminal_asks() {
    let root = tempfile::tempdir().unwrap();
    let (channel, env) = AskpassChannel::prepare_in(&root.path().join("askpass")).unwrap();
    let fifo = fifo_of(&env);
    channel.serve(Zeroizing::new("s3cret pass".to_owned()), Duration::from_secs(10));

    let first = run_helper(&root.path().join("askpass"), &env, "me@host's password: ", "");
    assert_eq!(first.stdout, "s3cret pass\n");
    assert_eq!(first.status, 0);
    assert_eq!(first.tty, "", "the saved secret is answered without asking");

    // The serving thread removes the FIFO once it has written.
    let deadline = Instant::now() + Duration::from_secs(5);
    while fifo.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!fifo.exists());

    // A second prompt (the saved password was wrong) is asked on the terminal.
    let retry = run_helper(&root.path().join("askpass"), &env, "me@host's password: ", "typed\n");
    assert_eq!(retry.stdout, "typed\n");
    assert_eq!(retry.tty, "typed\nme@host's password: \n");
}

#[test]
fn a_passphrase_prompt_is_answered_from_the_fifo() {
    let root = tempfile::tempdir().unwrap();
    let (channel, env) = AskpassChannel::prepare_in(&root.path().join("askpass")).unwrap();
    channel.serve(Zeroizing::new("phrase".to_owned()), Duration::from_secs(10));
    let run = run_helper(
        root.path(),
        &env,
        "Enter passphrase for key '/home/me/.ssh/id_ed25519': ",
        "",
    );
    assert_eq!(run.stdout, "phrase\n");
}

#[test]
fn other_questions_are_asked_on_the_terminal_even_with_a_secret_waiting() {
    let root = tempfile::tempdir().unwrap();
    let (channel, env) = AskpassChannel::prepare_in(&root.path().join("askpass")).unwrap();
    let fifo = fifo_of(&env);
    channel.serve(Zeroizing::new("never".to_owned()), Duration::from_secs(10));

    let host_key = "The authenticity of host 'h (10.0.0.1)' can't be established.\n\
                    Are you sure you want to continue connecting (yes/no/[fingerprint])? ";
    let run = run_helper(&root.path().join("askpass"), &env, host_key, "yes\n");
    assert_eq!(run.stdout, "yes\n");
    assert_eq!(run.tty, format!("yes\n{host_key}"));

    let otp = run_helper(&root.path().join("askpass"), &env, "Verification code: ", "123456\n");
    assert_eq!(otp.stdout, "123456\n");

    assert!(fifo.exists(), "the secret still waits for its own prompt");
}

#[test]
fn confirmations_answer_with_the_exit_status() {
    let root = tempfile::tempdir().unwrap();
    let (_channel, env) = AskpassChannel::prepare_in(&root.path().join("askpass")).unwrap();
    let prompt = "Allow use of key id_ed25519?";
    let yes = run_helper_with(&root.path().join("askpass"), &env, prompt, "yes\n", Some("confirm"));
    assert_eq!(yes.status, 0);
    let no = run_helper_with(&root.path().join("askpass"), &env, prompt, "no\n", Some("confirm"));
    assert_eq!(no.status, 1);
}

#[test]
fn an_unclaimed_secret_is_withdrawn_after_the_timeout() {
    let root = tempfile::tempdir().unwrap();
    let (channel, env) = AskpassChannel::prepare_in(&root.path().join("askpass")).unwrap();
    let fifo = fifo_of(&env);
    channel.serve(Zeroizing::new("late".to_owned()), Duration::from_millis(200));
    let deadline = Instant::now() + Duration::from_secs(5);
    while fifo.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!fifo.exists());
    let run = run_helper(&root.path().join("askpass"), &env, "me@host's password: ", "typed\n");
    assert_eq!(run.stdout, "typed\n");
}
