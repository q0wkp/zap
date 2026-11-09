# command -p resolves the given command with the system default PATH, ensuring the shell
# can find them even if the user has a clobbered PATH value.
command -p stty raw
HISTCONTROL=ignorespace
HISTIGNORE=" *"
WARP_SESSION_ID="$(command -p date +%s)$RANDOM"
_hostname=$(command -pv hostname >/dev/null 2>&1 && command -p hostname 2>/dev/null || command -p uname -n)
_user=$(command -pv whoami >/dev/null 2>&1 && command -p whoami 2>/dev/null || echo $USER)
if [[ "$OS" == Windows_NT ]]; then WARP_IN_MSYS2=true; else WARP_IN_MSYS2=false; fi
_msg=$(printf "{\"hook\": \"InitShell\", \"value\": {\"session_id\": $WARP_SESSION_ID, \"shell\": \"bash\", \"user\": \"%s\", \"hostname\": \"%s\"}}" "$_user" "$_hostname" | command -p od -An -v -tx1 | command -p tr -d " \n")
printf '\e\x50\x24\x64%s\x9c' "$_msg"
unset _hostname _user _msg
