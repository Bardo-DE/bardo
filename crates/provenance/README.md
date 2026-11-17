# prov

Provenance for the assistant: who acted (`Actor`, `AgentRef`), what an action does (`Effect`), and
the labels that follow data (integrity, confidentiality, source) with the `Witness` that may raise
them. Also the message shape and the OpenTelemetry names the program's daemons share.

Used by the actions service, the memory service, the shell, the Settings app and files, each pinned by git rev. Its ids come from
`account-core`, pinned at the same the accounts rev as every consumer (the accounts service moves first, then prov).

Moved out of the accounts service on 2026-10-28, unchanged since the accounts service dcc16ada.

Licence: MIT OR Apache-2.0.
