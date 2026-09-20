# ProjectFile JSON 对拍

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

不传 `-Samples` 时使用 `tools/json_compare_samples.txt` 中的固定清单：全部 19 份 MPX 样本，加 13 份覆盖 MPP8/9/12/14 日历、关系、分配及可用性路径的 MPP 样本。`-Samples` 可只对拍指定文件；显式传入空列表会报错，不会产生虚假的通过结果。更新清单时应先核对 Java/Rust 快照的具体差异，再纳入默认集。

依赖 Java/Javac、Python 3、Cargo 和 Maven（PATH 中的 `mvn.cmd`，或 Scoop 的 `~/scoop/apps/maven/current/bin/mvn.cmd`）。脚本每次先用 Maven 对 `mpxj/` 执行 `package`（跳过测试与 Javadoc），再使用本次构建的 `mpxj/target/mpxj-16.7.0.jar` 与 `mpxj/lib/*.jar`。Maven 只生成子模块已忽略的构建产物；**对拍工具的源码没有写入子模块**。Java 快照工具留在仓库根目录的 `tools/JsonSnapshot.java`，其编译产物与两侧 JSON 均写入根目录下忽略版本控制的 `_tmp_json_compare/`。Rust 入口是 `examples/json_snapshot.rs`。也可以单独运行 `python -X utf8 -B tools/compare_json.py <java.json> <rust.json>`。对拍一致返回码为 0，字段差异为 1，工具或快照错误为 2。

Java 参考实现取自本仓库固定的子模块提交 `c5e1320` 的**本地构建 jar**，而不是 Maven 仓库下载的同版本 jar。源码变动后，下一次对拍会自动重新构建，确保 jar 与源码版本一致。

当前快照是**明确的部分覆盖**，不是 MPXJ `JsonWriter` 的全字段输出：项目属性（ID、名称、作者、公司、起止、默认日历 ID），全部任务（标识、层级、名称、WBS、起止、工期、工时、成本、完成率、摘要/里程碑、日历 ID），全部资源（标识、名称、日历 ID、工时、成本），全部分配（标识、任务/资源 ID、起止、工时、成本），全部关系（标识、两端任务 ID、类型、滞后），全部日历（标识、名称、父 ID、项目 ID、类型）。未纳入字段、日历工作时段、展示数据、时间分段数据及自定义字段**不在一致性结论范围内**，后续可按同一 schema 逐步扩充。

日期是本地时间并保留纳秒，不做时区变换；工期保存原始数值和单位枚举整数；JSON 的 `null` 与缺少键不同。对象键顺序不参与比较；实体列表用 unique ID（缺失时备用键和出现序号）匹配，重复 ID 不会被吞掉。所有差异均显示 JSON 路径；解析或工具出错时停止，不当作「一致」。

Java 快照将非 ASCII 字符写为 JSON Unicode 转义，避免 Windows 系统默认 GBK 编码把带重音的日历、资源名称替换为 `?`；比较器解析 JSON 后仍按原始字符串语义比较。

首轮对拍发现 `legacy/sample.mpx` 和 `sample.de.mpx` 各有 11 处日历差异：日历创建时重入借用上下文，导致自动 unique ID 缺失，继而丢失资源日历的父 ID/类型及默认日历和资源的引用。后续全量对拍又发现 19 份 MPP 的任务工期差异：项目属性赋值时持续借用 `ProjectFile`，导致每天分钟数未同步至 `ProjectContext`；`sf259.mpp` 的非 ASCII 名称差异则是 Java 输出按系统 GBK 编码时被替换造成的。修复后，使用 Maven 构建的本地 Java jar 对拍 **19 份 MPX + 606 份可读 MPP**，上述快照字段范围内全部一致。另 3 份 MPP 受密码保护，未纳入对拍；快照未覆盖的字段不在此结论内。
