#!/bin/sh
# Answers the prompts of an ssh process started by the SSH manager. ssh runs this with the
# prompt as its only argument and reads the answer from standard output.
#
# The secret is handed over only at the prompt it is meant for, so that a passphrase is never
# sent to a server as a password.
case "$1" in
*"Are you sure you want to continue connecting"*)
    # ssh has no record of the server's host key. It takes the key's fingerprint as an answer
    # and compares it with the key the server offered, so answering with the fingerprint the
    # user confirmed accepts that key and no other. Without one, refuse, and hand the prompt
    # (which names the fingerprint) back through ssh's standard error.
    if [ -n "$ZAP_SSH_ASKPASS_HOST_KEY" ]; then
        printf '%s\n' "$ZAP_SSH_ASKPASS_HOST_KEY"
    else
        printf '%s\n%s\n%s\n' ZAP_SSH_HOST_KEY_PROMPT_BEGIN "$1" ZAP_SSH_HOST_KEY_PROMPT_END >&2
        echo no
    fi
    ;;
"Please type "*)
    # ssh asking again because the fingerprint it was given is not the key's.
    echo no
    ;;
"Enter passphrase for "*)
    [ "$ZAP_SSH_ASKPASS_SECRET_KIND" = passphrase ] || exit 1
    printf '%s\n' "$ZAP_SSH_ASKPASS_SECRET"
    ;;
*"'s password: ")
    [ "$ZAP_SSH_ASKPASS_SECRET_KIND" = password ] || exit 1
    printf '%s\n' "$ZAP_SSH_ASKPASS_SECRET"
    ;;
*)
    exit 1
    ;;
esac
