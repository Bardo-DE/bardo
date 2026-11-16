# fish completion for bardo-do. What to offer is bardo-do's own answer (`bardo-do __complete`),
# so apps, actions, parameter flags and choices follow the installed manifests.
function __bardo_do_complete
    set -l typed (commandline -opc)[2..-1] (commandline -ct)
    bardo-do __complete $typed 2>/dev/null
end
complete -c bardo-do -f -a '(__bardo_do_complete)'
