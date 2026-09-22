# mpxj-rs

[MPXJ](https://github.com/joniles/mpxj) **reader 部分**的 Rust 重写/移植：只读，逐方法对照 Java
源码实现，解析结果落在与 Java 一致的 `ProjectFile` 领域模型上。

> 本 crate 是 `mpxj` 的**非官方**移植，**只覆盖 reader**：不做 writer（`MPXWriter` / MSPDI），
> 不含完整 CPM 排程，也不含 P6 / Primavera 专用数据。包名取 `mpxj-rs` 以标注「Rust 移植」并与
> 上游 Java 项目区分——它**不由 MPXJ 作者维护**。上游以 git submodule 固定在
> `joniles/mpxj` @ `c5e1320cde0acd404031495aa66c17639a57239b`（`v16.7.0-14-gc5e1320cde`）。

## 支持的格式

| 格式 | 版本 | 说明 |
|---|---|---|
| **MPP** | MPP8 / MPP9 / MPP12 / MPP14 | Microsoft Project 二进制（Project 98 / 2000–2003 / 2007 / 2010 及以后），`MPPReader` |
| **MPX** | MPX 3.0 / 4.0 | Microsoft Project Exchange 文本，`MPXReader`；含本地化（如德语）名表 |

**不支持**：MSPDI（XML）、MPD、P6 / Primavera（XER / PMXML / P6 数据库）、Planner、Suretrak、
Asta、Phoenix、FastTrack、Ganttproject、Synchro、SDEF、Project Commander、Open Plan 等 MPXJ 支持的
其他格式；也不提供 `UniversalProjectReader`（Java 靠反射分发，Rust 无反射）。

## 安装

```toml
[dependencies]
mpxj-rs = "0.1"
```

库名仍为 `mpxj`：

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

MPX 同理，改用 `mpxj::mpx::mpx_reader::MPXReader`；非英语区域的 MPX 文件需先
`reader.set_locale(..)`（例如德语样本用 `MpxLocale::De`）。

## 特性与限制

- **只读**：不做 `MPXWriter` / MSPDI 等写入路径。
- **CPM 排程**：仅移植 `MicrosoftSlackCalculator`（总时差计算），不提供完整排程。
- 领域模型、字段号映射与文件头注释均逐一对照 Java 源码；每处与 Java 的差异都登记在对应
  文件头的差异表与 `TECH_DEBT.md` 中。
- 库代码不 panic：非法输入返回 `MpxjError`，越界/缺失字段退化为 `None` / `0` / 跳过该记录。

## 开发

```bash
git clone --recurse-submodules <repo-url>   # tests 依赖 mpxj/junit/data 下的真实样本
cargo build --all-targets
cargo test
cargo run --release --example sample_survey
```

## 许可

移植自 [MPXJ](https://github.com/joniles/mpxj)，沿用上游许可
[LGPL-2.1-or-later](https://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html)。
