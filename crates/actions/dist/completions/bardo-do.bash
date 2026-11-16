# bash completion for bardo-do. What to offer is bardo-do's own answer (`bardo-do __complete`),
# so apps, actions, parameter flags and choices follow the installed manifests.
_bardo_do() {
    local IFS=$'\n'
    COMPREPLY=($(bardo-do __complete "${COMP_WORDS[@]:1:$COMP_CWORD}" 2>/dev/null))
}
complete -F _bardo_do bardo-do
