# Architecture

This tree carries the slice of anyview the desktop uses: the format library (`anyview-core` and the
decoders), the light peek and thumbnail tier, the recently-viewed history the launcher reads, and the
platform edge the launcher, the file manager and the settings app call (single instance hand-off and the
shared thumbnail cache). The viewer program itself (its window, state machines, PDF and media
back ends, exports, plugins and the `anyview` binary) lives in the separate anyview repository, which
pins this tree by tag. This file is the map of what is here: which crate owns what, what may depend on
what, and where each concept lives. `CONVENTIONS.md` points to the style guides every repo shares.

## 1. Crates and allowed edges

Layers go lowest first. A crate names only crates in a lower layer, and
`scripts/check-boundary.sh` enforces the edges and the external boundaries below.

| Layer | Crate | Purpose |
| --- | --- | --- |
| L0 | `anyview-core` | pure vocabulary: kinds, sniffing, units, sequence, actions, edits, exports, view memory, the `Peek` trait |
| L1 | `anyview-store` | the recently-viewed history on disk, read side only (`read_history`, `History`, `HistoryEntry`, `ResumeLabel`); the launcher reads it, the viewer repository writes it |
| L1 | `anyview-image` | raster and vector images: decode to upright RGBA8, a downscaled peek with EXIF facts, encode for export, lossless JPEG rotation |
| L1 | `anyview-text` | text: encodings and windowed lines, code highlighting into token classes, Markdown to HTML, CSV tables, spreadsheets (calamine), JSON trees, and the five text peeks |
| L1 | `anyview-archive` | archives: zip, tar, 7z and compressed-stream listings read inside a byte budget, extracting one entry, the archive peek, and an office package's title, author, count and embedded picture |
| L1 | `anyview-book` | books: an EPUB's package (metadata, reading order, contents), a chapter as sealed HTML, a comic zip's pages in natural order, and the covers of both; reads the zip through `anyview-archive` |
| L1 | `anyview-font` | fonts: names and glyph count read with skrifa, the specimen as vector outlines, and the font peek |
| L1 | `anyview-fs` | the one place a path is opened: `OnDisk` makes the `Input` of a `FilePath` or a `Source`, `OpenFile` and `open_regular` open a regular file (a FIFO, a device or a socket is refused) |
| L2 | `anyview-platform` | the edge the desktop uses: the single-instance `Request` hand-off (D-Bus or a per-user socket) and the shared thumbnail cache, with fakes |
| L3 | `anyview-peek` | the light tier: the registry that maps every kind to its `Peek`, the folder, video and audio (pure-Rust header parsers) and facts-only peeks, the type-erased `AnyPeeked`, `look` (its one front door: probe a file and peek at it as `Peeking` says), and the pane view (what the launcher links) |

### Allowed edges (workspace crates and Bardo; everything else is forbidden)

| Crate | May depend on |
| --- | --- |
| `anyview-core` | `ui-core` |
| `anyview-store` | `anyview-core`, `base-fs` (`read_capped` for the history file) |
| `anyview-fs` | `anyview-core`, `base-fs` (`read_capped` behind `read_whole`, the capped whole-file read every reader of the person's files uses) |
| `anyview-image` | `anyview-core`, `ui-core` (`Word`, for the facts' labels) |
| `anyview-text` | `anyview-core`, `anyview-fs` (`OpenFile`, for a window of lines read by offset), `ui-core` (`Word` for token classes, and `base64` for `data:` URLs) |
| `anyview-archive` | `anyview-core`, `ui-core` (`Word` for entry kinds) |
| `anyview-book` | `anyview-archive`, `anyview-core` (its `data:` URLs through `image_data_url`) |
| `anyview-font` | `anyview-core` |
| `anyview-platform` | `anyview-core`, `ui-core` (`Word` for `ThumbSize`), `base-fs` (the thumbnail cache's capped reads and atomic writes), `service-locator` (the single-instance socket); with `desktop`, `zbus` and the action crates for the bus name and agent intents |
| `anyview-peek` | `anyview-archive`, `anyview-book`, `anyview-core`, `anyview-font`, `anyview-fs`, `anyview-image`, `anyview-text`, `ui-core`, and with `pane` Bardo view crates |

### Features

No crate turns a heavy dependency on by default: a default build of a library is its types and its portable code.

| crate | feature | what it adds |
|---|---|---|
| `anyview-platform` | `desktop` | D-Bus single instance with bus activation and the agent intents (Linux) |
| `anyview-platform` | `testing` | the fakes and the private bus |
| `anyview-peek` | `pane` | the pane view the launcher and the file manager link |
| `anyview-peek` | `media` | recordings' facts and an audio cover |


### External boundaries (`scripts/check-boundary.sh`)

| Crate | Never reaches |
| --- | --- |
| `anyview-core` | `dioxus`, `tokio`, `zbus`, `wgpu`, `pdfrum`, `mpv-wgpu-player`, `rsmpv`, `ffmpeg-next`, `ffmpeg-sys-next`, `image`, `syntect`, `blitz-dom`, `anyrender`; `serde_json` outside tests |
| `anyview-store` | `dioxus`, `tokio`, `zbus`, `wgpu`, `pdfrum`, `mpv-wgpu-player`, `rsmpv`, `ffmpeg-next`, `image`, `blitz-dom`, `blitz-paint`, `anyrender`: blocking file I/O only, so the launcher links it cheaply |
| `anyview-image` | `dioxus`, `tokio`, `zbus`, `wgpu`, `pdfrum`, `mpv-wgpu-player`, `rsmpv`, `ffmpeg-next`, the `blitz-*` crates, `anyrender`, `syntect`, `pulldown-cmark`: blocking decode and encode on the caller's worker, no spawning, no clock |
| `anyview-text` | `dioxus`, `tokio`, `zbus`, `wgpu`, `pdfrum`, `mpv-wgpu-player`, `rsmpv`, `ffmpeg-next`, the `blitz-*` crates, `anyrender`, `image`, `resvg`, `jxl-oxide`: blocking reads on the caller's worker, no spawning, no clock |
| `anyview-peek` | `mpv-wgpu-player`, `rsmpv`, `rsmpv-sys`, `ffmpeg-next`, `ffmpeg-sys-next`, `zbus`, `ashpd`, `cpal`, `alsa`, `alsa-sys` anywhere in its tree: no libmpv, no libav, no D-Bus and no sound card in the launcher's process, and no `anyview-media` at all (it probes with `symphonia`; playing is the media crate's). Its `media` feature (off by default; the viewer and the launcher ask for it) pulls in `symphonia`, `mp4parse` and `matroska-demuxer`, pure-Rust readers of a recording's header (section 2f); without it the crate leaves them out and a recording is peeked as facts only. `wgpu`, pdfrum and `tokio` are in its tree (they come with `ui-blitz`, which the launcher links) but it never names them itself: the DIRECT table of the script. Its tree is held to a package-count budget, and so is its headless tree (`--no-default-features`: no `pane`, so no `ui-blitz`, `dioxus` of its own or renderer, and no `anyview-image` encoder), which reaches none of `ui-blitz`, `wgpu`, `rav1e` or `zbus` and builds on macOS and Windows |
| `anyview-archive` | `dioxus`, `tokio`, `zbus`, `wgpu`, `pdfrum`, `mpv-wgpu-player`, `rsmpv`, `ffmpeg-next`, `ffmpeg-sys-next`, the `blitz-*` crates, `anyrender`, `image`, `resvg`, `jxl-oxide`, `syntect`, `pulldown-cmark`, `skrifa`: blocking reads on the caller's worker inside a byte budget, no spawning, no clock; the one crate that names the container codecs |
| `anyview-book` | `dioxus`, `tokio`, `zbus`, `wgpu`, `pdfrum`, `mpv-wgpu-player`, `rsmpv`, `ffmpeg-next`, `ffmpeg-sys-next`, the `blitz-*` crates, `anyrender`, `image`, `resvg`, `jxl-oxide`, `syntect`, `pulldown-cmark`, `skrifa`: blocking reads on the caller's worker, no spawning, no clock; it names no codec (the zip comes through `anyview-archive`) and decodes no picture |
| `anyview-font` | the same, and the archive codecs (`zip`, `tar`, `sevenz-rust2`, `flate2`, `bzip2`, `ruzstd`, `lzma-rust2`): the one crate that names `skrifa` for reading a face |
| `anyview-platform` | `dioxus`, `wgpu`, `pdfrum`, `mpv-wgpu-player`, `rsmpv`, `ffmpeg-next`, `image`, the `blitz-*` crates, `anyrender`, `syntect`, `pulldown-cmark`, `resvg`, `jxl-oxide`: the edge knows the desktop, not the pictures; it spawns no thread and runs on the binary's tokio runtime |

`anyview-image` depends on `image` (png and jpeg from the pinned block, gif, webp, bmp, tiff, ico, tga,
qoi, exr and hdr added by its own manifest), `psd` (Photoshop, MIT OR Apache-2.0), `icns` (Apple icons, MIT),
`jxl-oxide`, `resvg` (without text), `kamadak-exif`, `thiserror` and `ui-core`; with the `encode` feature
(which the viewer and its exports turn on) also `img-parts` and `ravif`. `anyview-text` depends on `syntect` (the pure-Rust regex engine, no
oniguruma), `pulldown-cmark`, `csv`, `serde_json`, `serde`, `encoding_rs`, `unicode-segmentation` (the editor's caret motion), `thiserror` and `ui-core`.

`anyview-archive` depends on `zip` (deflate only: the central directory needs no codec, extracting an entry does), `tar`, `flate2` (its pure-Rust back end), `ruzstd`, `lzma-rust2`, `bzip2` (its pure-Rust `libbz2-rs-sys` back end), `sevenz-rust2` (decoders only, no encoder), `thiserror`, `anyview-core` and `ui-core`; no C library. `anyview-book` depends on `anyview-archive`, `roxmltree` (the package documents of an EPUB as a read-only tree), `thiserror`, `anyview-core` and `ui-core`. `anyview-font` depends on `skrifa`, `miniz_oxide` (the zlib streams of a WOFF), `thiserror` and `anyview-core`.

`anyview-peek` depends on the five crates below it, `anyview-fs`, `ui-core` and `thiserror`; its `pane` feature adds `ds`,
`ui-blitz` (feature `pdf`) and `dioxus`. Outside `card/` (and the PDF raster that feeds it) no module names a type of
`ds` or dioxus: the PDF page is the peek's own `PageLook` and `PageTrouble`, and the card converts them to Bardo's
`PdfPage` at its boundary (`card/page.rs`). The headless tree has no dioxus, `ui-motion` or `ui-style`, and
`scripts/check-boundary.sh` fails if one returns. `wgpu` is not an exception to its rule so much as a fact of `ui-blitz`: `TextureLayer` and the PDF
thumbnail cache both live there, the launcher is a Blitz window and already links the renderer, and the
pane reaches the device only through `ds_blitz::use_gpu`, never by naming a `wgpu` type (FINDINGS, "The pane
and `wgpu`").

`anyview-core` depends on `serde`, `thiserror`, `infer` and `ui-core` and nothing else. It does no
I/O and reads no clock: a function that needs bytes takes them (`FileHead`, `ZipEntries`), and one
that needs the time takes it.

## 2. Modules inside `anyview-core`

`lib.rs` declares private modules and re-exports each public item once at the crate root; there is
no other public path. A module names only modules above it in this list.

| Module | Holds |
| --- | --- |
| `error` | `CoreError`, the crate's one error |
| `units` | page, section (`SectionIndex`, `SectionCount`: the chapter or comic page of a book), media (time, length, volume, speed, chapter, time range, bitrate), ratio, zoom, turn, content-space and pixel newtypes; all integer |
| `media` | what a player says of a recording: `StreamKind`, `MediaTrack`, `MediaChapter`, `VideoPresence`, `MediaTags` |
| `source` | `FilePath`, `FileName`, `FileStamp`, `Source` (a file's serialisable identity), `ReadAt` (bytes read by offset: a byte slice, a `Vec`, an `Arc`, an open `File`, or anything a host implements it for) and `ReadAtStream` (a `Read + Seek` over one), and `Input` (what a peek reads: name, stamp, shared bytes and, when they are a file of their own, the path; made from `&FilePath`, `&Source` or a name and a `Vec<u8>`) |
| `kind` | `FormatKind`, `Mime`, `FormatDetail`, `SyntaxName`, `kind_of_mime` and the format families |
| `sniff` | `sniff`, `sniff_zip`, `sniff_ole`, `Sniffed` and the head, entries and OLE directory names (`OleStreams`) they read; a head that is not UTF-8 but is printable bytes is plain text under a text extension (a legacy encoding) |
| `sequence` | `NonEmpty`, `Sequence`, `moved`, `neighbours` |
| `markup` | what a person writes on the pages of a document and the file keeps as standard annotations: `PageRect` and `PagePoint` (thousandths of the displayed page, from the top left), `Mark` (a page, a `MarkColor`, a `MarkShape`: highlight, underline, strike-through, note, rectangle, oval, line, arrow, text box; and `existing`, the `MarkRef` of the annotation the file already held that it stands for; `bounds`, `is_at`, `moved`), `MarkTool`, `FieldFill` and `MarkupSave` (the fields filled, the marks to write, made or changed, and the held marks removed, which the host writes) |
| `form` | `FormWidget` (one place a field is drawn: its name, `WidgetKind`, `read_only`, page, `PageRect`, value, the state that means on, the options), in the order the document lists them, which is the order Tab walks |
| `edit` | `Edit`, `EditKind`, and `Adjust` (a picture's whole adjustment as one value: cut to a `PixelRect`, `mirrored`, `QuarterTurn`, resized; `turned`, `flipped` and `cropped` fold each new step into it, so a series of edits is one `Edit::Adjust`) |
| `find` | `FindOptions` (how a find matches: the bools `match_case` and `whole_words`, with `DEFAULT` and `with_*`; the one set text and PDF search both read) |
| `action` | `FileAction`, `Reach`, `reach`, `shortcut`, `own_keys`, `file_action_of` |
| `export` | the per-format export enums, `AudioTarget`, `ExportChoice`, `ExportJob` (its `Transcode` is a cut, a track or a conversion of a recording) and its payloads |
| `resume` | `Resume`, `TrackChoice` |
| `facts` | `Fact`, `Facts`, `FactLabel` (and where each label is listed by default: `group`, `headline`), `FactGroup` (the sections of the Info panel, in display order, `General` last), the `headline` flag of a row (headline rows make the summary line, `Facts::summary`), `FactValue` and its constructors (size with exact bytes, date, coordinate, altitude, version), `FactTime`, `LocalZone` (the person's time zone, read from the system by `jiff`; every shown time is converted to it and never names a zone), `Coordinate`, `kind_name` (Finder's "JPEG image"), and `FileDetails` with `Facts::general` (the General section: kind, size, created, modified, where, where from, permissions); `label`, `group`, `value`, `when`, `place`, `summary`, `kind_name` and `file` are private |
| `peek` | `Peek`, `PeekBudget` (`deadline_from_now`), `Deadline` (the budget's time as an instant a long loop asks about; cooperative; an `Interrupt`) |
| `work` | `Backend`, `Stop` (`stopped`, `stopped_at`: plain bools), `Interrupt` (the one question a loop asks to end early: a peek's `Deadline` and a viewer job's `Stop` both answer it; reading the clock is its implementations', not the loop's), `Ticket`, `Ticketed`: the contract with the threads. The one public module: reached as `anyview_core::work::X` |
| `APP_NAME` | the name a person sees ("Viewer"): the welcome title, the helper sheet, the now-playing identity. Identifiers (binary, app id, bus names) keep `anyview` |
| `profile` | the one match on `FormatKind`: `actions_for`, `edits_for`, `mime_for`, `has_stage` |

## 2a. Modules inside `anyview-store`

Same rules as section 2: private modules, each public item re-exported once at the crate root. `io` is
the only module that touches the disk.

| Module | Holds |
| --- | --- |
| `error` | `StoreError`, `StoreOp` |
| `label` | `ResumeLabel` and `resume_label`, the row subtitle derived from a `Resume` |
| `history` | `HistoryEntry`, `History` |
| `io` | the capped whole-file JSON read |
| `reader` | `read_history`, `HistoryRead`: the API the launcher links; it never locks, never writes and never fails |

On disk, under `<data>/anyview` (`STORE_FOLDER`): `history.json` (the `History`, newest first). The
viewer replaces it by rename, so a reader never locks and never sees a partial file.

## 2c. Modules inside `anyview-image`

Same rules as section 2: private modules, each public item re-exported once at the crate root.
Everything is blocking and runs on the caller's worker: no runtime, no spawning, no clock. `decode`
and `decode_bytes` are the one way pixels come out, and `encode` the one way they go in.

| Module | Holds |
| --- | --- |
| `error` | `ImageError` |
| `pixels` | `Rgba8` (straight alpha), `PremultipliedRgba8`, and the one conversion between them |
| `orientation` | `ExifOrientation` (a mirror, `mirrored`, then a clockwise `QuarterTurn`), its tag table and `applied` |
| `exif` | `ExifFacts`, `Exposure`, `Ratio`, `SignedRatio`, `Flash`, `Location`: read with `kamadak-exif` (camera, lens, settings, bias, flash, software, copyright, density, and the GPS block; the body and lens serial numbers are never read); `ExifFacts::camera_facts` are the rows a preview may list; `ExifFacts` holds no place, and `Location::of_file` (read by `picture_facts` alone) is the place, which only the viewer's own Info panel lists; `format` words them; `patch` writes the orientation entry (private) |
| `picture_facts`, `resolution` | `picture_facts`: the colour, density, camera and location rows of the viewer's Info panel; `Resolution`: dots an inch from EXIF, a PNG `pHYs` chunk or a JPEG's JFIF block |
| `scale` | `resized` (the export's `Resize`); peek-budget fitting (private) |
| `decode` | `decode`, `decode_bytes`, `declared_size` (the upright size from the header and EXIF alone), `Decoded` (a still, an `Animation` with its `Plays`, or a `HeldStill` when the frames pass the 256 MiB cap), `Frame`, `ColourInfo`; `codec` is the one match on `RasterFormat`; `ceiling` (the peak memory a decode may hold, checked from the header before any pixel is decoded), `stills`, `plays` (loop counts from the container), `frame_count` (frame counts from the container, without decoding), `highrange` (EXR and HDR, tone mapped with extended Reinhard then sRGB), `layered` (Photoshop composite, largest icon of an ICNS), `jxl`, `svg`, `svg_limits` (a drawing's filter work and nested pictures, checked from its tree before it is drawn) and `look` are private |
| `decode::natural` | `natural_size`: the upright size a picture shows at, read from the first 256 KiB of the file (a header and its EXIF orientation, or an SVG's root tag); nothing is decoded and the claim is not checked against a decode's ceiling |
| `peek` | `RasterPeek` and `VectorPeek` (the two `Peek` implementations), `ImagePeek`, `PeekedFormat` |
| `encode` | `encode`, `encode_bmp`, `encode_with_metadata`; `codecs`, `avif` and `metadata` (EXIF and ICC splicing with `img-parts`) are private |
| `export` | `plan_export` (an image choice as `ExportJob`s: pure), `encode_file` (a file resized and encoded, keeping its metadata or not), `ImageFile` (a file's, or a comic entry's, bytes and what it sniffed as: its upright picture, an SVG's declared size, whether a JPEG is already upright) |
| `rotate` | `rotate_jpeg`, `flip_jpeg`: lossless rotation and mirroring by rewriting the EXIF orientation segment |
| `edit` | `edited`: an `Edit` applied to a picture file: a JPEG through its orientation tag alone, PNG, WebP, TIFF and BMP decoded, moved and written again with their metadata; animations and the formats with no lossless writer are refused. An `Edit::Adjust` that only turns or mirrors is those lossless edits one after the other; one that cuts or resizes decodes the picture, cuts, mirrors, turns and scales it, and encodes it again in its own format (JPEG at 92) with the colour profile and EXIF carried across, the orientation entry made upright (the turn is in the pixels now), and a place only if the file had one |

**Alpha** is straight (not premultiplied) everywhere in this crate; `Rgba8::premultiplied` is the
conversion a GPU compositor needs, and `PremultipliedRgba8` is a distinct type so the two cannot be
mixed. **Orientation** is applied in the decoder: pixels come back upright and nothing downstream
reads the EXIF tag. Encoding takes upright pixels, so a carried EXIF block has its orientation
reset to upright. A JPEG rotated "in place" is not decoded at all: `rotate_jpeg` rewrites one APP1
segment and copies every other byte.

The `avif` feature adds AVIF decoding through the `image` crate and the dav1d C library (BSD-2,
linked dynamically, found with pkg-config). It is off by default, so the default build needs no C
toolchain pieces; without it an AVIF file is `ImageError::NotCompiledIn`. AVIF encoding is `ravif`
(pure Rust, and with `rav1e` the largest tree in the crate), built only with the `encode` feature: that
feature gates every way pictures go out (the `encode` functions, `edited` and the exports) and the
metadata carried across a re-encode. A caller that only looks at pictures, as `anyview-peek` does,
links none of it.

## 2d. Modules inside `anyview-text`

Same rules as section 2. Blocking and effect-free except `bytes::FileBytes`, which reads ranges of
one file.

| Module | Holds |
| --- | --- |
| `error` | `TextError` |
| `bytes` | `ByteSource` (read a range), `FileBytes`, `HeldBytes` |
| `encoding` | `TextCodec`, `LegacyEncoding`, `detect` (byte-order mark, UTF-8 validity, else the encoding `chardetng` guesses from the first `DETECT_BYTES`, Windows-1252 when nothing else fits), `Coverage`, `Detected` |
| `lines` | `TextLines`: a sparse line index (every 64th line) so any window of lines is read and decoded without the rest; a line past `MAX_LINE_BYTES` (1 MiB) comes back cut there and ending in `LINE_CUT` (an ellipsis) |
| `find` | `Needle` (a phrase, never empty; it ignores case and finds parts of words until `with_options` gives it a `FindOptions`: `match_case` compares characters as they are, `whole_words` rejects a hit that runs into a letter, digit or underscore beside it, but only on a side where the hit itself ends in a word character), `FindHit` (a line and the bytes of it a phrase covers), `ByteOffset`, `MAX_HITS`; `TextLines::find` reads the file once, in batches |
| `code` | `Highlighter`, `SyntaxId`, `CodeLines` (windowed highlighting with saved parser states), `TokenClass`, `TokenSpan`, `TokenLine`, `tokens_html`; `class` is the one table from syntect scopes to classes, `state` and `html` are private |
| `markdown` | `render`, `Rendered`, `RenderEnv`, `LocalFiles`, `NoFiles`, `DiskFiles`, `Heading`, `HeadingLevel`, `Anchor`; `events` (the safety pass), `images`, `links` and `outline` are private |
| `table` | `Table`, `HeaderMode`, `RowIndex`, `Separator` (comma, tab, semicolon or pipe, chosen by the rows) and the read cap (`TABLE_BYTES`); `Workbook` and `Sheet` (calamine: XLSX, ODS, XLS, one `Table` per sheet, held in memory and capped by `SHEET_ROWS` and `WORKBOOK_CELLS`; XLSX and XLSB are streamed cell by cell, so a sheet that claims a grid it does not hold costs the cells it has; `open_start` reads the first sheet's start for a peek). A delimited file is not held: `Table::read` makes one pass that follows the quotes (a record is not a line, a quoted cell may hold line breaks) and keeps the offset of every 64th record, the record count and the widest row, as `lines` does for lines; `rows`, `cell` and `column` then parse a window of 64 records from its offset, and the last four windows are kept. `column` yields one column's cells window by window, so a sort keeps only its keys. `header` (the guess) and `indexed` (the record index and the windows) are private |
| `tree` | `Tree` (`read`, `children`, and `visible` / `visible_count`, the document flattened under an `OpenNodes` set and cut to a window), `TreeRow`, `VisibleRow`, `Openness`, `RowLabel`, `NodeKind`, `ChildCount`; `TreePath` and `OpenNodes` live in `anyview-core`. A tree is never parsed whole: one streaming scan checks the document is well formed (`grammar`) and indexes the children of its top level (`level`: the offset of every 64th child, sooner when children are large, as `lines` does), a container is indexed when it is first shown or opened, and a node's rows are read from the file when asked for, so a document of any size costs its index and the rows on screen and a number reads as written. A file longer than the limit given to `read` shows its start (`Coverage::Prefix`), and a peek of a JSON document over its budget lists the top-level keys that fit. `cursor` (bytes read in chunks) and `rows` are private |
| `edit` | the editor's model: `Buffer` (a piece table over the file's bytes with undo and redo: a run of typing, of Backspaces or of Deletes is one step, cut where the kind of character changes and at a line break, and a paste, a cut or a replaced selection is a step of its own; `Revision` stamps the text so "unsaved" is a comparison with the saved one), `Session` (a buffer with a caret or selection, the IME's preedit, and the commands a key, a click or the clipboard means; `Motion`, `Selection`, `Preedit`; `find` reads the edited text), `EditText` (a file read whole for editing, or the `EditRefusal` that stops it: not UTF-8, over `EDIT_BYTES` (8 MiB), unreadable); `motion` (grapheme and word boundaries, `unicode-segmentation`) is private. Line endings and a UTF-8 byte-order mark stay in the text as the file had them, so `Session::file_bytes` writes back what was read plus the edits. The data views edit without a buffer: `TableEdits` (in `table`) holds a table's changed cells, added rows and rows taken away as a sparse overlay of the file's rows, and `Table::write_edited` copies the file's own bytes around them (a changed cell is written quoted as it was, an added row in the file's line ending, so a diff shows only what changed); `TreeSession` (in `tree`) holds a JSON document as its bytes and edits it by splicing the spans the tree keeps (a value, a key, a node with its comma), so the indentation stays the file's; both keep their history in `edit`'s `Log`, whose `EditStamp` makes "unsaved" a comparison |
| `peek` | `PlainPeek`, `CodePeek`, `MarkdownPeek`, `TablePeek`, `TreePeek` and their `*Peeked` types, `Tally`, `PEEK_LINES` |
| `export` | `plan_export` and `plan_print` (a text choice, or a printout, as `ExportJob`s: pure) and `printable_html` (a Markdown, code or plain-text file as the whole page `ds_blitz::pdf` prints: Markdown rendered with its local images inlined, code highlighted, text as it is; `print.css` is its stylesheet, with fixed colours since paper is white) |
| `escape` | HTML escaping, the one place it is written (private) |

Token classes are words (`keyword`, `string`, `comment`, …), never colours: the stylesheet maps each
`tok-<class>` to a design-system colour token. Markdown never passes raw HTML through (it is shown
as text), keeps only web, mail and relative link targets, and writes local images as `data:` URLs
read through `LocalFiles`, because Bardo's sealed frames load nothing else.

## 2e. Modules inside `anyview-platform`

Same rules as section 2: private modules, each public item re-exported once at the crate root; the
implementations are reached through `portable`, `linux` and `testing`.

**Two halves, one feature.** `portable` is built everywhere and calls no desktop service; `desktop`
(public path `anyview_platform::linux`) is the Linux desktop's D-Bus single instance and agent intents,
built only with the opt-in feature `desktop` on Linux (the code is also gated on
`target_os = "linux"`; `zbus` is an optional dependency of that feature).

| Ability | With `desktop` (Linux) | Without it |
| --- | --- | --- |
| Single instance | `DbusInstance`: the name `org.bardo.Anyview1`, with bus activation | `LatchkeyInstance`: the first launch listens on `$XDG_RUNTIME_DIR/anyview/agent.sock`, a later one forwards one line of JSON and leaves |
| Thumbnails | `FreedesktopThumbnails`, re-exported from `linux` | the same, from `portable` |

Nothing reads `std::env`, `dirs` or a bus address outside `env`: a program builds an `Env`
(`Env::from_process`), a test its own (`Env::isolated`, which names no bus).

| Module | Holds |
| --- | --- |
| `error` | `PlatformError` (`NoBus`, `Bus`, `Io`, `Thumbnail`), `IoOp` |
| `env` | `Env` (`dirs`, `session`), `Dirs`, `BusRoute` (`Usual`, `Address`, `Absent`) |
| `uri` | `file_uri`: the escaped `file://` URI the thumbnail spec hashes |
| `instance` | `Instance`, `Request` (`Open`, `Peek`, `Play`, `Handoff`), `Handoff`, `Claim`, `Primary` |
| `handoff` (private) | a `Handoff` as the fields both wires carry, encoded and parsed once, for the bus and the socket |
| `thumbnail` | `ThumbnailCache`, `ThumbSize`, `ThumbPixels` |
| `portable` | `LatchkeyInstance` (the `frame` module is its line of JSON), `FreedesktopThumbnails` |
| `desktop` (feature `desktop`), public as `linux` | `DbusInstance` (and `forward_over`, `BUS_NAME`), the agent intents (`AGENT_BUS_NAME`, `OPEN_FILES`) |
| `testing` (feature `testing`) | `FakeInstance`, `FakeThumbnails`, and `PrivateBus` (a private `dbus-daemon` with `desktop`) |

On the bus: `org.bardo.Anyview1` at `/org/bardo/Anyview1` has `Open(as)`, `Peek(s)`, `Play(s)` and
`Handoff(s file, s resume, t results, as entries)` over absolute paths (a relative one is an
`InvalidArgs` error). A handoff is a file the launcher's pane was showing: `resume` is the JSON a
`Resume` is stored as, `entries` the paths of the search's results in the order shown with `file`
among them, and `results` the id of that search, so the arrow keys walk the results. The launcher calls
it with `forward_over(&connection, &Request::Handoff(..))` on its own connection, so the wire is written
once, here. Thumbnails live at `<cache>/thumbnails/{normal,large,x-large}/<md5 of the file URI>.png`
with `Thumb::URI`, `Thumb::MTime` and `Thumb::Size` text chunks.

## 2f. Modules inside `anyview-peek`

Same rules as section 2: private modules, each public item re-exported once at the crate root. The peeks
are blocking and run on the caller's worker (`worker` is that worker, for a host that has none); only
`pane` draws. Its `pane` feature (off by default; the viewer asks for it) is everything that draws or rasterises, `ds`, `dioxus` and
`ui-blitz`; without it the crate links none of them, and a PDF is peeked as facts only.

Every peek reads an `anyview_core::Input`: a path (made by `anyview_fs::OnDisk::on_disk` on a
`FilePath` or a `Source`, since `anyview-core` does no I/O and opens nothing) or any bytes a host injects
(`Input::from((name, bytes))`, or its own `ReadAt`). A back end reads it through `ReadAt` or
`Input::reader`; only two things need a real path, and each refuses cleanly when the input has none:
the folder peek (a directory has no bytes) and a thumbnail read from the desktop's cache (the host's
`StillSource`). A PDF takes the page cache when it has a path and `pdf_thumb_bytes` when it has not.

The front door is `look(src, &Peeking) -> AnyPeeked`: it probes the file and peeks at it. `Peeking` is
with `Default` (the launcher pane's settings, `Peeking::pane()`) and `with_*` builders for
the budget, the box a PDF page is fitted into (`fit`, a `PixelSize`) and the host's `StillSource`. `probe` and
`peek` stay for a caller that holds a `Probed` between the two (the viewer's window probes on one job and
peeks on another).

| Module | Holds |
| --- | --- |
| `error` | `PeekError` |
| `registry` | `KindVisitor`, `visit`: the one exhaustive match over `FormatKind` in the light tier |
| `body` | `Body` (the type-erased result), `Light` (a `Peek` whose result and error convert into `Body` and `PeekError`) |
| `probe` | `Probed`, `probe`: what a file is, from its first 4 KiB and, for a zip, its entries (`anyview_archive::zip_entries`); a zip that cannot be listed is a plain archive. It takes a path or any `Input`, as `peek` does |
| `any` | `AnyPeeked` (with `still()`, the picture a card shows) and `peek`: runs the registry's visitor, adds the size and the date, and turns a failure into `Body::Unavailable`; a video that shows only its facts takes the host's still |
| `looking` | `look`, `Peeking`: probe, then peek, as the settings say; one front door for the launcher's pane, the viewer's cards and a host's worker |
| `unavailable` | `Unavailable` (`Missing`, `Unreadable`, `TooBig { len, allowed }`, `Damaged(reason)`, with `label()` for the words): why a card has no peek |
| `frames` | `StillSource`, `NoStills`: the seam through which the host lends a small picture of a file the peek does not decode, a video's cached thumbnail in the launcher and a picture's first frame in the viewer (the peek names no platform crate, so the host reads the cache) |
| `media` | `VideoPeek`, `AudioPeek`, `MediaLook`; with the `media` feature, private `recording` (`Recording`, the header as plain values, and its rows), `audio` (symphonia: MP3, AAC and ALAC in M4A, FLAC, Ogg Vorbis and Opus, WAV, AIFF; tags, track number and the front cover), `mp4` (mp4parse over the `ftyp` and `moov` boxes alone, found by seeking over the rest: MP4, M4V, MOV), `matroska` (matroska-demuxer: MKV, WebM) and `peek` (which parser a container goes to, the cover reduced to the budget); AVI, WMV, FLV, MPEG-TS, MPEG and Ogg video go to no parser; without the feature, `absent` (`FactsPeek`) |
| `described` | `FactsPeek<K>` and one marker per kind with no back end yet (`OtherPeek`; `VideoPeek` and `AudioPeek` only without `media`): what sniffing established, nothing pretended |
| `office` | `OfficePeek`, `OfficeLooked`: the package's facts and its own picture (a document's thumbnail) as `Body::Picture` |
| `book` | `BookPeek`, `BookLook`: an EPUB's title, author, publisher, language and chapters, or a comic's page count, over the cover (the package's cover image, else its first image; a comic's first page), reduced to the budget; a book that cannot be opened still shows its type |
| `folder` | `FolderPeek`, `FolderSummary`: one level, item count, size and kinds |
| `natural` | `natural_size`: the size a picture or a video shows at, from the first bytes of the file (a picture's header through `anyview-image`, a movie's `moov` or Matroska header up to 8 MiB); any other kind, a path that is not a regular file and a header that does not say give `None` |
| `pdf` | `PdfPeek`, `PdfPeeked`, `PageLook`, `PageTrouble` (a page drawn, blank or failed, in no component's types): the first page, through `ui-blitz`'s thumbnail cache for a file and `pdf_thumb_bytes` for bytes handed in; facts only without the `pane` feature |
| `worker` | `PeekWorker` (`spawn`, or `looking` for `look` itself), `WorkerConfig` (with `with_*` builders): the one thread that looks at the latest file asked for (`ask(input, reply)`), a thread per peek with an 8 MiB stack, the 4 s overrun and the cap on abandoned peeks; it takes no async runtime |
| `when` | `modified_text`: a modification time as the Info panel words it, in the person's zone |
| `card` | `Card`, `Part`, `Parts` (which of the media, the name and the facts it draws; all by default), `STYLE`; `page` (a `PageLook` to and from Bardo's `PdfPage`, the only place the two meet), `picture` (a `TextureLayer`), `lines` (plain and highlighted), `grid` (a table, a tree's top level and an archive's first entries, all as Bardo's `Table`), `specimen` (a font's sample lines, each an inline SVG of the face's outlines) and `frame` (Markdown in a sealed frame) are private, and `card.css` is its stylesheet |

`peek(src, sniffed, budget)` never fails: a peek that cannot be made returns a `Body::Unavailable` with the
reason and the facts the file can still give. The pane draws a `Body` over the file's name and a `FactList`
of its facts; the actions under it are the host's.

## 2j. Modules inside `anyview-archive`

Same rules as section 2. Blocking and effect-free except the reads of the one file it is asked about.
A listing reads inside a byte budget: a zip or a 7z whose index is larger than it is refused
(`ArchiveError::OverBudget`), a tar or a compressed stream is listed as far as the budget reaches and
its count is a lower bound (`EntryCount::AtLeast`).

| Module | Holds |
| --- | --- |
| `error` | `ArchiveError` |
| `container` | `container`, the one match on `ArchiveFormat`: `Container` (`Zip`, `Tar`, `Compressed(Codec)`, `SevenZip`) and `Codec`; `format_name` and `kind_name`, the words for a format (private) |
| `entry` | `Entry`, `EntryKind`, `EntryCount`, `EntryLimit`, `Listing`; `Tally`, which keeps the first entries and counts the rest (private) |
| `limit` | `Limited` (a reader that stops at the budget and says so) (private) |
| `list` | `list`: the one entry point, and the compressed-stream case (a tar inside, or one file) |
| `extract` | `extract`, `ExtractLimits`: one entry's bytes, bounded by the entry and by how much of a stream is unpacked |
| `zip_archive`, `tar`, `sevenz`, `stream` | one container each: the central directory (and `zip_entries`, the names and `mimetype` sniffing wants), tar headers (also over a decompressed prefix), the 7z header (`sevenz_header` checks every size it states against the file, a count limit and a byte limit before the 7z crate allocates from it), and gzip, bzip2, xz and Zstandard unpacked up to a cap through readers that stream (private except `zip_entries`) |
| `peek` | `ArchivePeek`, `ArchivePeeked`, `PEEK_ENTRIES` |

A compressed stream is a tar when what it unpacks to parses as one (the header checksum decides), else
one file named like the stream without its last extension. Everything is held in memory up to the cap:
a listing's cap is the peek budget, an extraction's is `ExtractLimits::scanned`.

## 2k. Modules inside `anyview-font`

Same rules as section 2. Blocking: the peek reads the table directory of the first face (a collection's
header first) and only the tables it draws from, which together must fit the byte budget; a WOFF, whose
tables are packed, is read whole.

| Module | Holds |
| --- | --- |
| `error` | `FontError` |
| `face` | `Face`, `Variation`: the family and style names, glyph count and variation axes of a face, read with skrifa |
| `specimen` | `Specimen`, `SpecimenLine`, `EM`: sample lines set with each glyph's advance (no shaping, no kerning) as SVG path data in 1000 units to the em |
| `tables` | `sparse`: the directory read from the source by range, then the wanted tables (names, character map, advances, outlines) laid out as a plain font of their own (private) |
| `woff` | `to_sfnt`: a WOFF's tables unpacked and laid out again as the plain font it wraps, and `sfnt`, which lays any tables out as a font (private) |
| `peek` | `FontPeek`, `FontPeeked` |

The specimen is outlines, not a font handed to the renderer: the pane needs no font loading, the peek
stays a small value whatever the font's size, and what is drawn is the face the file holds. A font that
maps none of the sample letters shows the first characters it does map. A WOFF is unpacked to the
plain font it wraps (`woff.rs`: each table's zlib stream, the tables laid out again as an sfnt; the
tables together may unpack to 64 MiB and may not share stored bytes) and read
like one; WOFF2 is named (`face: None`) and not opened.

## 2o. Modules inside `anyview-book`

Same rules as section 2. Blocking and effect-free except the reads of the one file it is asked about;
the zip is read through `anyview-archive`, one entry at a time.

| Module | Holds |
| --- | --- |
| `error` | `BookError` |
| `epub` | `Epub` (open: container, package document and spine; `contents`, `chapter`), `EpubMeta`, `TocEntry`; `package` (metadata, manifest, spine, cover), `contents` (the EPUB 3 navigation document, else the EPUB 2 NCX, else the chapters by number) and `chapter` (the zip's files behind a chapter, with what it may take in) are private |
| `comic` | `Comic`, `ComicPage`: the images of a zip in natural order, one page's bytes |
| `natural` | `natural_order`: digits by value, text without case |
| `seal` | `Chapter` (`styles` and `body`), `seal`: a chapter's markup rebuilt from an allowlist (`element`), its styles sealed (`css`), its images inlined through `Assets`; `tokens` (a forgiving HTML tokenizer), `walk`, `entities` are private |
| `cover` | `Cover`, `epub_cover`, `comic_cover`: one image's bytes, never decoded here |
| `zip_path` | the references a package makes, resolved to entry names (private) |

A chapter is sealed by writing only what is known: elements and attributes come from allowlists, so a
script, a frame, a form, an event handler or an unknown element never reaches the page (its text does).
Stylesheets lose `@import`, `@font-face` and every `url()` that is not a file of the package, which is
inlined as a `data:` URL, and so do images (an SVG `image` becomes a picture; an image the package does
not hold becomes its alt text). A link keeps only a fragment or a web or mail address. A chapter takes in
at most 24 MiB of files. The viewer lays each chapter out on a fixed reading page and binds the chapters as one PDF the PDF
stage shows (`families/pdf/bound`, below); fixed-layout books, scripts, audio and video, fonts and DRM are not supported.

### Books as PDFs

An EPUB and a comic zip are shown by the PDF stage (the registry maps `FormatKind::Book` to
`PdfStageView`), so they have its page stack, find, zoom, thumbnails, contents and resume (`Resume::Pdf`:
page and offset) and one wheel, with no frame to swallow it. `families/pdf/bound` makes the document on the
open worker: `epub` lays each chapter (sealed HTML with `data:` images, plus the book's styles) out with
`ds_blitz::pdf` on one fixed page of 776 by 1164 points, text 680 wide in 14 point Inter at line-height 1.6
(`book.css`: the Reading face of the text family, in points so that a page at 100% reads at `--fs-reading`);
`comic` puts each picture on a page of its own size (`anyview_pdf::pdf_of_pictures`, twelve at a time so a
comic is never all decoded at once); `anyview_pdf::bind` joins the parts and writes the outline: each line of
the book's contents goes to the first page of its chapter, and, where several lines lead into one chapter file,
to the first page from the previous line's on that shows the line's words. The result is kept for the session by
file and stamp (`kept`: four books, 256 MiB) so opening it again is instant; nothing is written to disk. The
`StageFamily`, `Stage` and `Job` of a book no longer exist; `Resume::Book` stays in `anyview-core` only so an
old store still reads.

## 3. Layer rules

1. **A lower layer never names a higher one.** If something needed lives above, move the shared
   piece down or pass it in.
2. **`anyview-core` is pure.** Effects (reading files, listing a zip, decoding, playing) belong to
   crates above it, which hand it values: the first 4 KiB of a file, a zip's entry names.
3. **Every kind of file is mapped from day one.** A kind without a viewer stage is `PeekOnly` (facts
   only), never a stub.
4. **No `_` arm on our own enums.** The workspace turns on clippy's `wildcard_enum_match_arm` and
   `match_wildcard_for_single_variants` at `deny`. An input a state ignores is listed by name, so
   adding a variant is a compile error everywhere it matters.
5. **One exhaustive match per enum, in one file.** `FormatKind` is matched in `profile`;
   `FileAction` in `action`'s spec; each format family in its own file behind the `Family` trait.
   A second match elsewhere is a missing method.
6. **Typestate where a lifecycle is linear and owned in one place.** `Sniffed` can only be made by
   sniffing, so a loader that takes one is never handed a kind guessed from a name alone.
