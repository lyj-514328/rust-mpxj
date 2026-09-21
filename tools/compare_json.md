# ProjectFile JSON 对拍

并发编排由 `tools/run_json_compare.py` 负责：Java 与 Rust 各启动**一个常驻 server 进程**，样本路径经 stdin 批量送入，快照按样本落盘，随后比较器在 Python 进程内并行比对。JSON 以 UTF-8 字节流保存，避免 Windows 控制台代码页改写非 ASCII 字符。

常驻 server 省掉了逐样本启动 JVM 的固定开销（本机小样本约 1.8 s/次，其中大部分是类加载）。server 模式：`--server <输出目录>`，stdin 每行 `<stem>\t<样本路径>`，结果写入 `<输出目录>/<stem>.json`，内容为 `{"path": <样本路径>, "result": <ProjectFile 快照>}`；读取失败时改为 `{"path": ..., "error": ...}`。`--threads N` 控制 server 内部线程池（默认 CPU 核数）。server 全程把 `System.out` 重定向到 stderr，避免上游诊断污染协议通道。`--parallelism` 现在表示 server 内部线程数，而不是并发进程数。

```powershell
python -X utf8 -B tools/run_json_compare.py --all-fields --parallelism 8
```

已有的 PowerShell 入口仍然保留，会把参数转发给 Python：

```powershell
./tools/run_json_compare.ps1 -AllFields -Parallelism 8 -Quiet
```

在仓库根目录执行：

```powershell
./tools/run_json_compare.ps1
./tools/run_json_compare.ps1 -Samples mpxj/junit/data/sample.de.mpx
```

全量对拍（排除 3 份已知受密码保护的 MPP 文件，共 19 份 MPX + 606 份 MPP）：

```powershell
$mpxSamples = @(rg --files mpxj/junit/data -g '*.mpx')
$mppSamples = @(rg --files mpxj/junit/data -g '*.mpp' |
    Where-Object { $_ -notmatch '(bothpassword9|readpassword9|password-protected-mpp9)\.mpp$' })
$samples = @($mpxSamples) + @($mppSamples)
if ($samples.Count -ne 625) { throw "样本总数不符: $($samples.Count)" }
./tools/run_json_compare.ps1 -Samples $samples
```

字段级全量模式：

```powershell
./tools/run_json_compare.ps1 -AllFields
./tools/run_json_compare.ps1 -Samples mpxj/junit/data/sample.de.mpx -AllFields
# 大样本集只保留最终汇总
./tools/run_json_compare.ps1 -Samples $samples -AllFields -Quiet
```

`-AllFields` 会在两侧快照中附加四类实体的 `field_catalog`、明确的 `field_exclusions`，以及每个项目属性、任务、资源和分配的全部非空 `field_values`。目录来自 Java/Rust 各自的内置 `FieldType.values()` / `ALL`，并包含 `CUSTOM:<uniqueID>` 自定义字段；比较器会拒绝缺目录、缺实体字段列表、重复字段名、未登记字段和被排除字段，避免两侧同时漏值而误报一致。`CURRENT_DATE` 在未显式缓存时标记为 `<dynamic>`，避免读取时刻不同造成伪差异。

当前排除项为 MPXJ 明确的 P6 专用字段：`PROJECT_CODE_VALUES`、`RESOURCE_CODE_VALUES`、`ROLE_CODE_VALUES`、`RESOURCE_ASSIGNMENT_CODE_VALUES`、`ACTIVITY_CODE_VALUES`、`EXPENSE_ITEMS`。本仓库 reader 不实现 P6/Primavera 数据，排除项会在快照中显式列出而不会静默跳过。

不传 `-Samples` 时使用 `tools/json_compare_samples.txt` 中的固定清单：全部 19 份 MPX 样本，加 13 份覆盖 MPP8/9/12/14 日历、关系、分配及可用性路径的 MPP 样本。`-Samples` 可只对拍指定文件；显式传入空列表会报错，不会产生虚假的通过结果。更新清单时应先核对 Java/Rust 快照的具体差异，再纳入默认集。

依赖 Java/Javac、Python 3、Cargo 和 Maven（PATH 中的 `mvn.cmd`，或 Scoop 的 `~/scoop/apps/maven/current/bin/mvn.cmd`）。脚本每次先用 Maven 对 `mpxj/` 执行 `package`（跳过测试与 Javadoc），再使用本次构建的 `mpxj/target/mpxj-16.7.0.jar` 与 `mpxj/lib/*.jar`。Maven 只生成子模块已忽略的构建产物；**对拍工具的源码没有写入子模块**。Java 快照工具留在仓库根目录的 `tools/JsonSnapshot.java`，其编译产物与两侧 JSON 均写入根目录下忽略版本控制的 `_tmp_json_compare/`（Java 与 Rust 快照分别落在 `_tmp_json_compare/java/`、`_tmp_json_compare/rust/`）。Rust 入口是 `examples/json_snapshot.rs`，两侧都支持单文件模式（stdout 输出 JSON）与 `--server` 常驻模式。也可以单独运行 `python -X utf8 -B tools/compare_json.py <java.json> <rust.json>`（单文件模式的两份快照仍可直接比较）。对拍一致返回码为 0，字段差异为 1，工具或快照错误为 2。

Java 参考实现取自本仓库固定的子模块提交 `c5e1320` 的**本地构建 jar**，而不是 Maven 仓库下载的同版本 jar。源码变动后，下一次对拍会自动重新构建，确保 jar 与源码版本一致。

默认模式仍保持精简的基础快照。`-AllFields` 覆盖四类实体的全部内置字段和已声明的自定义字段值，并附加日历工作周/例外/星期时段、资源可用性与五张费率表、视图/表格/列、筛选/分组、用户字段定义、数据链接等对象。关系及其它未列出的展示细节仍不属于 schema；MPXJ 的十个内置 `WorkContour` 是静态常量，不作为容器成员输出。该模式仍不是 MPXJ `JsonWriter` 的逐对象完全序列化，未覆盖的对象会继续列在本说明中。

日期是本地时间并保留纳秒，不做时区变换；工期保存原始数值和单位枚举整数；JSON 的 `null` 与缺少键不同。对象键顺序不参与比较；实体列表用 unique ID（缺失时备用键和出现序号）匹配，重复 ID 不会被吞掉。所有差异均显示 JSON 路径；解析或工具出错时停止，不当作「一致」。

Java 快照将非 ASCII 字符写为 JSON Unicode 转义，避免 Windows 系统默认 GBK 编码把带重音的日历、资源名称替换为 `?`；比较器解析 JSON 后仍按原始字符串语义比较。

首轮对拍发现 `legacy/sample.mpx` 和 `sample.de.mpx` 各有 11 处日历差异：日历创建时重入借用上下文，导致自动 unique ID 缺失，继而丢失资源日历的父 ID/类型及默认日历和资源的引用。后续对拍又发现 MPP 任务工期、超链接空值、FILETIME 时区/单位、自定义属性代码页与二进制值、备注终止符及计算字段默认值差异，均已按 Java 行为修复并补回归测试。最终使用 Maven 构建的本地 Java jar，在 `-AllFields` 模式下对拍 **19 份 MPX + 606 份可读 MPP**；另 3 份 MPP 受密码保护，未纳入对拍。结论只覆盖上段明确列出的字段集合。
