# ProjectFile JSON Comparison

Concurrency orchestration is handled by `tools/run_json_compare.py`: Java and Rust each start **a single long-lived server process**, sample paths are fed in bulk over stdin, snapshots are written to disk per sample, and the comparator then compares them in parallel inside the Python process. JSON is saved as a UTF-8 byte stream so the Windows console code page cannot rewrite non-ASCII characters.

The long-lived server removes the fixed cost of starting a JVM per sample (about 1.8 s per run for small samples on this machine, most of it class loading). Server mode: `--server <output-dir>`, one `<stem>\t<sample-path>` per stdin line, results written to `<output-dir>/<stem>.json` with the content `{"path": <sample-path>, "result": <ProjectFile snapshot>}`; on read failure it becomes `{"path": ..., "error": ...}`. `--threads N` controls the server's internal thread pool (default: number of CPU cores). Throughout its run the server redirects `System.out` to stderr so upstream diagnostics cannot pollute the protocol channel. `--parallelism` now means the number of threads inside the server, not the number of concurrent processes.

```powershell
python -X utf8 -B tools/run_json_compare.py --all-fields --parallelism 8
```

The existing PowerShell entry point is still retained and forwards its arguments to Python:

```powershell
./tools/run_json_compare.ps1 -AllFields -Parallelism 8 -Quiet
```

Run from the repository root:

```powershell
./tools/run_json_compare.ps1
./tools/run_json_compare.ps1 -Samples mpxj/junit/data/sample.de.mpx
```

Full comparison (excluding the 3 known password-protected MPP files; 19 MPX + 606 MPP in total):

```powershell
$mpxSamples = @(rg --files mpxj/junit/data -g '*.mpx')
$mppSamples = @(rg --files mpxj/junit/data -g '*.mpp' |
    Where-Object { $_ -notmatch '(bothpassword9|readpassword9|password-protected-mpp9)\.mpp$' })
$samples = @($mpxSamples) + @($mppSamples)
if ($samples.Count -ne 625) { throw "sample count mismatch: $($samples.Count)" }
./tools/run_json_compare.ps1 -Samples $samples
```

Field-level full mode:

```powershell
./tools/run_json_compare.ps1 -AllFields
./tools/run_json_compare.ps1 -Samples mpxj/junit/data/sample.de.mpx -AllFields
# For large sample sets keep only the final summary
./tools/run_json_compare.ps1 -Samples $samples -AllFields -Quiet
```

`-AllFields` adds to both sides' snapshots the `field_catalog` and explicit `field_exclusions` for the four entity kinds, plus all non-null `field_values` of every project property, task, resource, and assignment. The catalog comes from each side's built-in Java/Rust `FieldType.values()` / `ALL` and includes `CUSTOM:<uniqueID>` custom fields; the comparator rejects a missing catalog, a missing entity field list, duplicate field names, unregistered fields, and excluded fields, so that both sides missing a value at once cannot be misreported as matching. `CURRENT_DATE` is marked `<dynamic>` when it is not explicitly cached, to avoid spurious differences caused by reading at different moments.

The current exclusions are MPXJ's explicitly P6-specific fields: `PROJECT_CODE_VALUES`, `RESOURCE_CODE_VALUES`, `ROLE_CODE_VALUES`, `RESOURCE_ASSIGNMENT_CODE_VALUES`, `ACTIVITY_CODE_VALUES`, `EXPENSE_ITEMS`. This repository's reader does not implement P6/Primavera data, so the exclusions are listed explicitly in the snapshot rather than skipped silently.

When `-Samples` is not passed, the fixed list in `tools/json_compare_samples.txt` is used: all 19 MPX samples plus 13 MPP samples covering the MPP8/9/12/14 calendar, relation, assignment, and availability paths. `-Samples` can compare only the specified files; explicitly passing an empty list raises an error and does not produce a spurious pass. When updating the list, first verify the specific differences between the Java/Rust snapshots before adding anything to the default set.

Requires Java/Javac, Python 3, Cargo, and Maven (either `mvn.cmd` on PATH, or Scoop's `~/scoop/apps/maven/current/bin/mvn.cmd`). Each run first uses Maven to `package` `mpxj/` (skipping tests and Javadoc), then uses the jar built in that run, `mpxj/target/mpxj-16.7.0.jar`, together with `mpxj/lib/*.jar`. Maven only produces build artifacts that the submodule already ignores; **the comparison tool's source is not written into the submodule**. The Java snapshot tool lives at the repository root in `tools/JsonSnapshot.java`, and both its compiled output and the two sides' JSON are written under the version-control-ignored `_tmp_json_compare/` at the root (Java and Rust snapshots land in `_tmp_json_compare/java/` and `_tmp_json_compare/rust/` respectively). The Rust entry point is `examples/json_snapshot.rs`; both sides support a single-file mode (JSON to stdout) and the `--server` long-lived mode. You can also run `python -X utf8 -B tools/compare_json.py <java.json> <rust.json>` on its own (the two snapshots from single-file mode can still be compared directly). A comparison match returns exit code 0, field differences return 1, and tool or snapshot errors return 2.

The Java reference implementation is taken from a **locally built jar** of this repository's pinned submodule commit `c5e1320`, not from a same-version jar downloaded from the Maven repository. After source changes, the next comparison rebuilds automatically, ensuring the jar matches the source version.

The default mode still keeps a lean basic snapshot. `-AllFields` covers all built-in fields and declared custom field values of the four entity kinds, and additionally attaches objects such as calendar work weeks/exceptions/weekday periods, resource availability and the five rate tables, views/tables/columns, filters/groups, user field definitions, and data links. Relations and other presentation details not listed still fall outside the schema; MPXJ's ten built-in `WorkContour`s are static constants and are not output as container members. This mode is still not a full per-object serialization like MPXJ's `JsonWriter`; objects not covered continue to be listed in this description.

Dates are local time and keep nanoseconds, with no time-zone conversion; durations store the raw value and the unit enum integer; JSON `null` differs from a missing key. Object key order does not participate in comparison; entity lists are matched by unique ID (falling back to a secondary key and an occurrence index when missing), and duplicate IDs are not swallowed. All differences show the JSON path; a parse or tool error stops the run and is not treated as a match.

Java snapshots write non-ASCII characters as JSON Unicode escapes, so the Windows default GBK encoding does not replace accented calendar and resource names with `?`; after parsing the JSON, the comparator still compares by original string semantics.

The first comparison round found 11 calendar differences each in `legacy/sample.mpx` and `sample.de.mpx`: creating a calendar re-entered the borrowed context, so the automatic unique ID was missing, which in turn lost the parent ID/type of resource calendars and the references to the default calendar and resources. Later comparison rounds found further differences in MPP task durations, hyperlink null values, FILETIME time zone/unit, custom property code page and binary values, notes terminators, and computed field default values; all were fixed to match Java behavior and covered by regression tests. In the end, using the locally built Maven Java jar, the `-AllFields` mode compared **19 MPX + 606 readable MPP** files; the other 3 MPP files are password-protected and were not included. The conclusions cover only the field set explicitly listed in the paragraph above.
