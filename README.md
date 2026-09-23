# mpxj-rs

A Rust rewrite/port of the **reader portion** of [MPXJ](https://github.com/joniles/mpxj):
read-only, implemented method-by-method against the Java source, with results landing on the same
`ProjectFile` domain model as Java.

> This crate is an **unofficial** port of `mpxj` and **only covers the reader**: no writers
> (`MPXWriter` / `MSPDIWriter`), no full CPM scheduling, and no P6 / Primavera-specific data.
> The package is named `mpxj-rs` to mark it as a "Rust port" and to distinguish it from the
> upstream Java project — it is **not maintained by the MPXJ author**. The upstream is pinned as a
> git submodule at `joniles/mpxj` @ `c5e1320cde0acd404031495aa66c17639a57239b`
> (`v16.7.0-14-gc5e1320cde`).

## Supported formats

| Format | Versions | Notes |
|---|---|---|
| **MPP** | MPP8 / MPP9 / MPP12 / MPP14 | Microsoft Project binary (Project 98 / 2000–2003 / 2007 / 2010+), `MPPReader` |
| **MPX** | MPX 3.0 / 4.0 | Microsoft Project Exchange text, `MPXReader`; includes localized (e.g. German) name tables |
| **MSPDI** | Project 2002–2019 | Microsoft Project XML, `MSPDIReader`; includes embedded external subprojects, timephased data and custom field lookup |

**Not supported**: MPD, P6 / Primavera (XER / PMXML / P6 database), Planner, Suretrak,
Asta, Phoenix, FastTrack, Ganttproject, Synchro, SDEF, Project Commander, Open Plan and the other
formats MPXJ supports; there is also no `UniversalProjectReader` (Java dispatches via reflection,
which Rust lacks), so MSPDI must be identified manually by its root element
`xmlns="http://schemas.microsoft.com/project"`.

## Installation

```toml
[dependencies]
mpxj-rs = "0.1"
```

The library name remains `mpxj`:

```rust
use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::reader::project_reader::ProjectReader;

let mut reader = MPPReader::new();
let file = reader.read_file(std::path::Path::new("plan.mpp"))?;

let file = file.borrow();
for task in file.get_tasks().iter() {
    println!("{}", task.borrow().get_name().unwrap_or_default());
}
# Ok::<(), mpxj::MpxjError>(())
```

MPX works the same way, using `mpxj::mpx::mpx_reader::MPXReader`; MPX files for non-English locales
require `reader.set_locale(..)` first (for example, the German samples use `MpxLocale::De`).

For MSPDI (XML), use `mpxj::mspdi::mspdi_reader::MSPDIReader`; encoding defaults to UTF-8 and can be
overridden with `reader.set_charset(..)`.

## Features and limitations

- **Read-only**: no write paths such as `MPXWriter` / `MSPDIWriter`.
- **CPM scheduling**: only `MicrosoftSlackCalculator` (total slack) is ported; full scheduling is not provided.
- The domain model, field-number mappings and file-header comments are checked against the Java
  source method by method; every difference from Java is recorded in the corresponding file-header
  differences table and in `TECH_DEBT.md`.
- Library code never panics: invalid input returns `MpxjError`, and out-of-range/missing fields
  degrade to `None` / `0` / skipping the record.

## Development

```bash
git clone --recurse-submodules <repo-url>   # tests rely on real samples under mpxj/junit/data
cargo build --all-targets
cargo test
cargo run --release --example sample_survey
```

## License

Ported from [MPXJ](https://github.com/joniles/mpxj), under the upstream license
[LGPL-2.1-or-later](https://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html).
