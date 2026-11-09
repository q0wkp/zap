set -g WARP_SESSION_ID (random)
set _hostname (command -v hostname >/dev/null 2>&1 && command hostname 2>/dev/null || uname -n)
set _user (command -v whoami >/dev/null 2>&1 && command whoami 2>/dev/null || echo $USER)
set _msg (printf "{\"hook\": \"InitShell\", \"value\": {\"session_id\": $WARP_SESSION_ID, \"shell\": \"fish\", \"user\": \"%s\", \"hostname\": \"%s\"}}" "$_user" "$_hostname" | command od -An -v -tx1 | command tr -d " \n")
printf '\e\x50\x24\x64%s\x9c' "$_msg"
set -e _hostname _user _msg
