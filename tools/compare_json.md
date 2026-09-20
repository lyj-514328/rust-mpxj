# ProjectFile JSON 对拍

在仓库根目录执行：

```powershell
./tools/run_json_compare.ps1
./tools/run_json_compare.ps1 -Samples mpxj/junit/data/sample.de.mpx
```

依赖 Java/Javac、Python 3、Cargo 和 Maven（PATH 中的 `mvn.cmd`，或 Scoop 的 `~/scoop/apps/maven/current/bin/mvn.cmd`）。脚本每次先用 Maven 对 `mpxj/` 执行 `package`（跳过测试与 Javadoc），再使用本次构建的 `mpxj/target/mpxj-16.7.0.jar` 与 `mpxj/lib/*.jar`。Maven 只生成子模块已忽略的构建产物；**对拍工具的源码没有写入子模块**。Java 快照工具留在仓库根目录的 `tools/JsonSnapshot.java`，其编译产物与两侧 JSON 均写入根目录下忽略版本控制的 `_tmp_json_compare/`。Rust 入口是 `examples/json_snapshot.rs`。也可以单独运行 `python -X utf8 -B tools/compare_json.py <java.json> <rust.json>`。对拍一致返回码为 0，字段差异为 1，工具或快照错误为 2。

Java 参考实现取自本仓库固定的子模块提交 `c5e1320` 的**本地构建 jar**，而不是 Maven 仓库下载的同版本 jar。源码变动后，下一次对拍会自动重新构建，确保 jar 与源码版本一致。

当前快照是**明确的部分覆盖**，不是 MPXJ `JsonWriter` 的全字段输出：项目属性（ID、名称、作者、公司、起止、默认日历 ID），全部任务（标识、层级、名称、WBS、起止、工期、工时、成本、完成率、摘要/里程碑、日历 ID），全部资源（标识、名称、日历 ID、工时、成本），全部分配（标识、任务/资源 ID、起止、工时、成本），全部关系（标识、两端任务 ID、类型、滞后），全部日历（标识、名称、父 ID、项目 ID、类型）。未纳入字段、日历工作时段、展示数据、时间分段数据及自定义字段**不在一致性结论范围内**，后续可按同一 schema 逐步扩充。

日期是本地时间并保留纳秒，不做时区变换；工期保存原始数值和单位枚举整数；JSON 的 `null` 与缺少键不同。对象键顺序不参与比较；实体列表用 unique ID（缺失时备用键和出现序号）匹配，重复 ID 不会被吞掉。所有差异均显示 JSON 路径；解析或工具出错时停止，不当作「一致」。

首轮结果（仅限上述字段）：`legacy/sample.mpp` 和 `mpp14availability.mpp` 一致；`legacy/sample.mpx` 和 `sample.de.mpx` 均报告相同的 11 处差异，集中在四个日历的 unique ID、两个资源日历的父 ID/类型，以及默认日历和资源的日历 ID。`ProjectCalendarContainer::add` 经上下文可变借用进入 `ProjectCalendar::new`，后者的 `try_borrow` 因重入失败而不生成自动 ID；这是 MPX 差异的已确认起点，需在模型层单独修复并补跨格式回归测试。目前不能宣称 MPX 语义一致。
