set -g WARP_SESSION_ID (random)
set _hostname (command -v hostname >/dev/null 2>&1 && command hostname 2>/dev/null || uname -n)
set _user (command -v whoami >/dev/null 2>&1 && command whoami 2>/dev/null || echo $USER)
set -g WARP_IN_MSYS2 (test "$OS" = Windows_NT; and echo true; or echo false)
if test "$WARP_IN_MSYS2" = true; set _msg "\e]9278;k;A;InitShell\a\e]9278;k;B;session_id;$WARP_SESSION_ID\a\e]9278;k;B;shell;fish\a\e]9278;k;B;user;$_user\a\e]9278;k;B;hostname;$_hostname\a\e]9278;k;B;is_subshell;true\a\e]9278;k;C\a"; else; set _msg (printf "{\"hook\": \"InitShell\", \"value\": {\"session_id\": $WARP_SESSION_ID, \"user\": \"%s\", \"hostname\": \"%s\", \"shell\": \"fish\", \"is_subshell\": true}}" "$_user" "$_hostname" | command od -An -v -tx1 | command tr -d " \n"); end
printf '\e\x50\x24\x64%s\x9c' "$_msg"
set -e _hostname _user _msg
