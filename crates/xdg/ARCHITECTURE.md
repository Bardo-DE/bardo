# xdg

What a file is, and the freedesktop.org formats a desktop reads about files and apps. Three crates,
split so a program takes only the half it needs.

| Crate | Kind | What |
| --- | --- | --- |
| `file-types` | types | `FileName`, `MimeType`, `FormatKind`, the format families (`RasterFormat`, `ArchiveFormat`, ...) with their extensions and MIME types, and `of_mime`: the one table from a MIME type to a kind. Depends on `base-words` only. |
| `xdg-core` | pure | Parsers over text: `DesktopFile`, the desktop-entry layer (`DesktopId`, `XdgEnv`, `ApplicationEntry`, `match_app`, `AppIdentity`), the shared MIME database and `mimeapps.list` (`mime::*`), and `OsRelease`. Reads no file and no environment. |
| `xdg-fs` | io | Reads what the parsers take: `load_entries` (the walk of the `applications` folders), `load_mime_db` and `load_associations` (from the folders in `XdgDirs`), and `process_xdg_env`. |

The viewer's kinds and the file manager's names and types are the same types, so a MIME type
means one kind everywhere: `anyview-core`, `files-core`, the shell's file search and the file icon
resolver all use `file-types`.

`file-types` sits in this domain, below `anyview`, because the viewer's vocabulary builds on it.

| Crate | May depend on |
| --- | --- |
| `file-types` | `base-words` |
| `xdg-core` | `base-fs` (`ReadCap`, the size limit a desktop file is read under), `base-xdg`, `file-types` |
| `xdg-fs` | `base-fs`, `xdg-core` |
