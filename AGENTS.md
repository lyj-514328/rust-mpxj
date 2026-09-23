# AGENTS.md

Operating guide for AI agents / new contributors working in this repository. **Read this entire file before touching the code.**

## 1. What Is This Project

A Rust port of the Java library [MPXJ](https://github.com/joniles/mpxj).

- **Positioned as a reader**: it only reads files and does not implement a writer.
- **Supports three formats**: MPP (Microsoft Project binary), MPX (Microsoft Project Exchange text) and MSPDI (Microsoft Project XML).
- **Output matches Java**: it reuses the same unified `ProjectFile` data structure (`src/project_file.rs`).

**The Java source is the sole authority.** The upstream is vendored as a submodule under `mpxj/` (`joniles/mpxj` @ `c5e1320cde0acd404031495aa66c17639a57239b`, `v16.7.0-14-gc5e1320cde`).
Whenever the Rust-side behavior is in question, read the corresponding class and methods under `mpxj/src/main/java/org/mpxj/...` and implement according to the Java behavior; **do not** rewrite it based on what you intuitively think is "more reasonable".

This project is **reader-only**: no writer implementation exists under `src/` (`MPXWriter`, `MSPDIWriter`, and `MPPWriter` are not ported).

Explicitly **out of scope** (already logged as Category A hard limitations in Section 7 of `TECH_DEBT.md`):

- Write paths: `MPXWriter` / `MSPDIWriter` / `MPPWriter` and other writers
- Other reader formats: MPD (Access database), P6 / Primavera, Gnome Planner, and Microsoft Planner (cloud API); the three supported formats are MPP, MPX and MSPDI
- CPM scheduling (`src/cpm/` only ports `MicrosoftSlackCalculator`)
- P6 / Primavera-specific data (`LocationContainer`, `ExpenseItem`, the various `Code` classes …)
- `UniversalProjectReader` (Java dispatches via `Class.forName` reflection; Rust has no reflection)

## 2. Environment and Common Commands

```bash
# The first clone must include submodules: both tests/ and the examples depend on real samples under mpxj/junit/data
git clone --recurse-submodules <repo-url>

cargo build --all-targets                 # Must compile with no errors and no warnings
cargo test                                # All unit tests + integration tests
cargo run --release --example sample_survey   # Iterate real samples and tally the success rate
```

Toolchain: edition 2024 (local rustc 1.97.1). The dependencies are `byteorder` / `cfb` / `chrono` / `encoding_rs` / `roxmltree` / `thiserror` / `uuid`.

## 3. Code Map

| Path | Contents |
|---|---|
| `src/*.rs` (root directory, 139 files) | Domain model layer: `ProjectFile`, `Task`, `Resource`, `ResourceAssignment`, `ProjectCalendar`, `Relation` … |
| `src/common/` (30 files) | Port of `org.mpxj.common`: `*Helper`, `*FieldLists`, the Tokenizer family, MPP field-number mapping tables |
| `src/mpp/` (130 files) | MPP read pipeline: OLE adaptation, `Props*`, `Fixed*`/`Var*` blocks, `FieldMap*`, `MPP8/9/12/14Reader`, presentation-layer classes |
| `src/mpx/` (29 files) | MPX read pipeline: `MPXReader`, `LocaleData`, `MPXJNumberFormat`, etc. |
| `src/mspdi/` (`mspdi_reader.rs`, `datatype_converter.rs`, `xsd_duration.rs`, `xml_node.rs`, `reader_*.rs`, `schema/`) | MSPDI (XML) read pipeline: the reader plus the XML binding model (`schema/`, the port of the JAXB `Project` tree) and the per-section `impl ReaderContext` blocks |
| `src/reader/`, `src/listener/`, `src/cpm/` | Reader base classes, listeners, CPM (slack calculation only) |
| `tests/`, `examples/` | End-to-end integration tests and the sample survey tool |
| `TECH_DEBT.md` | **Authoritative technical debt inventory** (see Section 8) |

## 4. Hard Conventions (Required Reading Before Changing Code)

1. **Comments and documentation are always in English** (for internationalization; the Chinese comments have been migrated to English in full).
2. **Every file starts with a `//!` documentation block** in which "Differences from Java" records every inconsistency in a table:
   ```rust
   //! | Java | Rust | Notes |
   //! |---|---|---|
   ```
   It must also include "Skipped Java members + reason". **Any newly added difference must be recorded in this table**, otherwise later readers will assume it reflects the Java behavior.
3. **Every method carries a corresponding note**: `/// Corresponds to Java: \`public void setXxx(int v)\``.
4. **Library code never panics**:
   - Java uses exceptions for control flow (`ArrayIndexOutOfBoundsException` / NPE / `EOFException`) → return `None` / `0` / `Err`.
   - Array out of bounds → return 0 or skip that record.
   - `RefCell` always uses `try_borrow` / `try_borrow_mut`; on a borrow conflict **degrade gracefully** (return `None`/`Err`/skip) and state in a comment that this is a Rust-specific branch.
5. **Reproducing Java inheritance**: default to "composition + per-method forwarding".
   Only when a subclass **overrides none** of the base class methods may you use `Deref` / `DerefMut` to eliminate the forwarding boilerplate (existing: `src/recurring_task.rs`, `src/mpp/props8/9/12/14.rs`; additionally `src/mpp/gantt_chart_view{9,12,14}.rs` uses `Deref` to express another kind of relationship). Whenever a subclass needs to override base class behavior, always forward explicitly so each method can be compared against Java.
6. **Sharing and ownership**:
   - Shared entities use `Rc<RefCell<T>>` (corresponding to Java's reference semantics).
   - Back-references (child → parent) always use `Weak`, to avoid `Rc` reference cycles leaking.
   - When Java's `this` semantics must be reproduced (e.g. the callback `mapID(this)` inside `setID`), add a `self_ref: Weak<RefCell<T>>` field plus a wiring method that the container is responsible for calling.
7. **Do not add third-party crates**. If you truly need one, confirm with the maintainer first (`TECH_DEBT.md` Section 10 lists several trade-offs that hinge on this). The approved, already-present set is exactly: `byteorder`, `cfb`, `chrono`, `encoding_rs`, `roxmltree`, `thiserror`, `uuid`.
8. **New modules must be declared in `src/lib.rs`**.
9. **Name things to align with Java where possible**: keep the file structure and class names consistent. For constructs Rust cannot express (overloads, trait objects, etc.), follow the repository's existing suffix conventions (`_by_id` / `_at_offset` / `_with_data_type` / `_for_locale` …) and note in a comment that "the Java side forms an overload with X".

## 5. Borrow-Model Pitfalls (Must Read These First)

These are the root causes of real past failures; ask yourself about each one while writing code:

- **The scrutinee temporary of an `if let` / `match` lives until the end of the statement**:
  ```rust
  // ✗ Will panic: RefCell already borrowed
  if let Some(id) = task.borrow().get_subproject_task_unique_id() {
      task.borrow_mut().set_subproject_task_unique_id(Some(id));
  }
  // ✓ Take the value out first, then mutate after the borrow is released
  let id = task.borrow().get_subproject_task_unique_id();
  if let Some(id) = id { task.borrow_mut().set_subproject_task_unique_id(Some(id)); }
  ```
- **Do not hold a borrow of `ProjectFile` while calling a method that borrows the same file in turn**.
- **Java strong references vs Rust `Weak`**: in Java, `m_context` is a strong reference and can be accessed freely; on the Rust side it is `Weak` + `RefCell`, so **ask yourself "can this borrow actually succeed on this path?"**. A cautionary example is `removed` in `src/project_calendar_container.rs`: querying resources/tasks through the context necessarily fails on the real call path (the container can only be obtained via `ProjectContext::get_calendars_mut()`), so it was changed to scan the container elements directly.
- **Do not use tricks like `mem::take` to bypass borrowing**: doing so leaves the container's counts/sub-lists permanently empty (this historically caused "top-level task WBS always 1"). The correct approach is for **the party that owns the context to pass the data in**, rather than having the container call back into the parent file.

## 6. Standard Process for Porting a Piece of Java Code

1. Read the **entire class** in question under `mpxj/src/main/java/org/mpxj/...` (not just the target method).
2. Translate: method names to snake_case, field names with the `m_` prefix removed, preserving call order.
3. Verify field by field and branch by branch, paying special attention to easy misses:
   - ternary branches such as `x == null ? A : B`;
   - two constants that are **identically named but come from different sources** (e.g. `AvailabilityTable.END_DATE_NA` and `LocalDateTimeHelper.END_DATE_NA` differ by 1 minute, and Java mixes them within a single method);
   - the rounding behavior of integer/floating-point arithmetic (`NumberHelper.round`, `intValue() * 52`, etc.).
4. Add the file-header differences table + method correspondence comments.
5. Add unit tests (`#[cfg(test)] mod tests`, English comments, snake_case test names).
   Prefer tests that **distinguish specific values** over tests that merely assert `is_some()`.
6. Run the verification baseline in Section 7.
7. If something truly cannot be done without deviating from Java, **do not force an implementation**: keep the original logic, write the reason into the file-header differences table, and log it in `TECH_DEBT.md`.

## 7. Verification Baseline (Must Be Maintained on Every Change)

```bash
cargo build --all-targets        # No errors and no warnings
cargo test                       # All green
cargo run --release --example sample_survey
# Comment-language guard: must report only the intentional non-ASCII test data
# (encoding/codepage decode assertions, locale data, multi-byte UTF-16 tests).
rg -nP '[\x{4e00}-\x{9fff}]' -g '*.rs' src tests examples
```

Current baseline:

- `cargo build --all-targets`: no errors, **no warnings**
- `cargo test`: **1927** lib unit tests + **18 + 10 + 10** integration tests, plus 4 ignored doctests
- `sample_survey`: **MPP 606/609** (the other 3 are password-protected files, as expected), **MPX 19/19**, **MSPDI 143/148** (the other 5 XML samples are not MSPDI and are skipped by the root-element fingerprint, see `is_mspdi` in `examples/sample_survey.rs`)
- Comment-language guard: only intentional non-ASCII test data remains (`src/mpx/locale_data_zh.rs`, `src/mpx/localized_names.rs`, and a handful of encoding/codepage/multi-byte assertions such as `decode_lpstr` in `src/mpp/project_properties_reader.rs` and `src/common/charset_helper.rs`)

> ⚠️ **The baseline only proves "parsing does not crash and produces data"; it does not prove field-by-field correctness.**
> "100% of samples pass" once masked real defects (the five name tables in `sample.de.mpx` used the English tables, and the `availableTo` constant was mixed up).
> Do not rely on these three items alone when changing semantics; you must add tests that distinguish specific values.

## 8. Technical Debt

`TECH_DEBT.md` is the sole authoritative inventory. The "Remediation Progress" at the top records:

- which items have been fixed (with commits);
- which, after verification, **cannot be reproduced** (with reasons, so nobody retries them);
- which original judgments were themselves wrong (including the two disproven in the audit body).

**Read it before starting work**, to avoid re-fixing or treating an intentionally preserved design as a defect.

## 9. Committing

- Commit messages are in **English**, one commit per concern; a single change commits "code + corresponding tests + file-header differences table" together.
- Do not commit `target/`, `Cargo.lock` (currently in `.gitignore`), or `_tmp_*/`.
- Do not take the opportunity to do unrelated refactoring or formatting.
- Do not deviate from Java behavior just to be "more Rust"; when the two conflict, Java wins, and the deviation must be written into the differences table.
