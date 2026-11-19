#!/bin/sh
# Answers the prompts of an ssh process started by the SSH manager. ssh runs this with the
# prompt as its only argument and reads the answer from standard output.
#
# The secret is handed over only at the prompt it is meant for, so that a passphrase is never
# sent to a server as a password.
case "$1" in
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
