//! 对拍用的 ProjectFile JSON 快照（只读；字段范围见 `tools/compare_json.md`）。
//!
//! # 与 Java 的差异
//! | Java | Rust | 说明 |
//! |---|---|---|
//! | `tools/JsonSnapshot.java` | 本示例 | Rust 不增加 JSON crate，使用本地的最小 JSON 编码器；字段和值类型一致 |
//!
//! # 跳过的 Java 成员及原因
//! 本示例不是 Java 类的移植；未列入快照的字段统一在对拍说明中登记。

use std::cell::RefCell;
use std::env;
use std::path::Path;
use std::rc::Rc;

use chrono::NaiveDateTime;
use mpxj::ProjectFile;
use mpxj::duration::Duration;
use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::mpx::locale_utility::MpxLocale;
use mpxj::mpx::mpx_reader::MPXReader;
use mpxj::project_entity_with_id::ProjectEntityWithID;
use mpxj::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use mpxj::reader::project_reader::ProjectReader;

enum Json {
    Null,
    Text(String),
    Int(i32),
    Number(f64),
    Bool(bool),
    Array(Vec<Json>),
    Object(Vec<(&'static str, Json)>),
}

impl Json {
    /// 将快照值编码成标准 JSON；非有限浮点数使用 JSON 的 `null`。
    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Text(value) => {
                out.push('"');
                for ch in value.chars() {
                    match ch {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        '\u{0000}'..='\u{001f}' => out.push_str(&format!("\\u{:04x}", ch as u32)),
                        _ => out.push(ch),
                    }
                }
                out.push('"');
            }
            Self::Int(value) => out.push_str(&value.to_string()),
            Self::Number(value) if value.is_finite() => out.push_str(&value.to_string()),
            Self::Number(_) => out.push_str("null"),
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index != 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Object(fields) => {
                out.push('{');
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index != 0 {
                        out.push(',');
                    }
                    Self::Text((*key).to_owned()).write(out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// 将 Java 的 nullable Integer 映射为 JSON number/null。
fn integer(value: Option<i32>) -> Json {
    value.map(Json::Int).unwrap_or(Json::Null)
}

/// 将 Java 的 nullable Number 映射为 JSON number/null。
fn number(value: Option<f64>) -> Json {
    value.map(Json::Number).unwrap_or(Json::Null)
}

/// 将 Java 的 nullable String 映射为 JSON string/null。
fn string(value: Option<&str>) -> Json {
    value
        .map(|s| Json::Text(s.to_owned()))
        .unwrap_or(Json::Null)
}

/// 将 Java 的 LocalDateTime 映射为固定九位小数的 ISO 本地时间。
fn date(value: Option<NaiveDateTime>) -> Json {
    string(
        value
            .as_ref()
            .map(|v| v.format("%Y-%m-%dT%H:%M:%S%.9f").to_string())
            .as_deref(),
    )
}

/// 将 Java 的 Duration 映射为数量与枚举值，保留原始单位。
fn duration(value: Option<Duration>) -> Json {
    match value {
        Some(v) => Json::Object(vec![
            ("value", Json::Number(v.get_duration())),
            ("unit", Json::Int(v.get_units() as i32)),
        ]),
        None => Json::Null,
    }
}

/// 与 Java 示例按同一命名规则选择 MPX 区域设置。
fn locale(path: &Path) -> MpxLocale {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .rsplit('.')
        .next()
        .and_then(MpxLocale::get_instance)
        .unwrap_or(MpxLocale::En)
}

/// 从文件扩展名选择已有的 MPX/MPP reader。
fn read(path: &Path) -> Result<Rc<RefCell<ProjectFile>>, String> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mpp") => MPPReader::new()
            .read_file(path)
            .map_err(|e| e.message().to_owned()),
        Some("mpx") => {
            let mut reader = MPXReader::new();
            reader.set_locale(locale(path));
            reader.read_file(path).map_err(|e| e.message().to_owned())
        }
        _ => Err("仅支持 .mpp 和 .mpx".to_owned()),
    }
}

/// 生成与 Java 示例同结构的完整容器快照。
fn snapshot(file: &Rc<RefCell<ProjectFile>>) -> Json {
    let file = file.borrow();
    let p = file.get_project_properties();
    let properties = Json::Object(vec![
        ("unique_id", integer(p.get_unique_id())),
        ("name", string(p.get_name().as_deref())),
        ("author", string(p.get_author().as_deref())),
        ("company", string(p.get_company().as_deref())),
        ("start", date(p.get_start_date())),
        ("finish", date(p.get_finish_date())),
        (
            "default_calendar_unique_id",
            integer(p.get_default_calendar_unique_id()),
        ),
    ]);
    let tasks: Vec<_> = file.get_tasks().iter().cloned().collect();
    let resources: Vec<_> = file.get_resources().iter().cloned().collect();
    let assignments: Vec<_> = file.get_resource_assignments().iter().cloned().collect();
    let relations: Vec<_> = file.get_relations().iter().cloned().collect();
    let calendars: Vec<_> = file.get_calendars().iter().cloned().collect();
    drop(file);

    Json::Object(vec![
        ("properties", properties),
        (
            "tasks",
            Json::Array(
                tasks
                    .iter()
                    .map(|item| {
                        let t = item.borrow();
                        Json::Object(vec![
                            ("id", integer(t.get_id())),
                            ("unique_id", integer(t.get_unique_id())),
                            ("parent_unique_id", integer(t.get_parent_task_unique_id())),
                            ("name", string(t.get_name().as_deref())),
                            ("wbs", string(t.get_wbs().as_deref())),
                            ("outline_level", integer(t.get_outline_level())),
                            ("start", date(t.get_start())),
                            ("finish", date(t.get_finish())),
                            ("duration", duration(t.get_duration())),
                            ("work", duration(t.get_work())),
                            ("cost", number(t.get_cost())),
                            ("percentage_complete", number(t.get_percentage_complete())),
                            ("summary", Json::Bool(t.get_summary())),
                            ("milestone", Json::Bool(t.get_milestone())),
                            ("calendar_unique_id", integer(t.get_calendar_unique_id())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "resources",
            Json::Array(
                resources
                    .iter()
                    .map(|item| {
                        let r = item.borrow();
                        Json::Object(vec![
                            ("id", integer(r.get_id())),
                            ("unique_id", integer(r.get_unique_id())),
                            ("name", string(r.get_name().as_deref())),
                            ("calendar_unique_id", integer(r.get_calendar_unique_id())),
                            ("work", duration(r.get_work())),
                            ("cost", number(r.get_cost())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "assignments",
            Json::Array(
                assignments
                    .iter()
                    .map(|item| {
                        let a = item.borrow();
                        Json::Object(vec![
                            ("unique_id", integer(a.get_unique_id())),
                            ("task_unique_id", integer(a.get_task_unique_id())),
                            ("resource_unique_id", integer(a.get_resource_unique_id())),
                            ("start", date(a.get_start())),
                            ("finish", date(a.get_finish())),
                            ("work", duration(a.get_work())),
                            ("cost", number(a.get_cost())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "relations",
            Json::Array(
                relations
                    .iter()
                    .map(|item| {
                        let r = item.borrow();
                        Json::Object(vec![
                            ("unique_id", integer(r.get_unique_id())),
                            (
                                "predecessor_unique_id",
                                integer(r.get_predecessor_task().and_then(|t| {
                                    t.try_borrow().ok().and_then(|t| t.get_unique_id())
                                })),
                            ),
                            (
                                "successor_unique_id",
                                integer(r.get_successor_task().and_then(|t| {
                                    t.try_borrow().ok().and_then(|t| t.get_unique_id())
                                })),
                            ),
                            ("type", Json::Int(r.get_type() as i32)),
                            ("lag", duration(r.get_lag())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "calendars",
            Json::Array(
                calendars
                    .iter()
                    .map(|item| {
                        let c = item.borrow();
                        Json::Object(vec![
                            ("unique_id", integer(c.get_unique_id())),
                            ("name", string(c.get_name())),
                            ("parent_unique_id", integer(c.get_parent_unique_id())),
                            ("project_unique_id", integer(c.get_project_unique_id())),
                            ("type", Json::Int(c.get_type() as i32)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// 命令行入口：仅向 stdout 输出 JSON，错误写到 stderr。
fn main() {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("用法: cargo run --example json_snapshot -- <file.mpx|file.mpp>");
        std::process::exit(2);
    };
    match read(Path::new(&path)) {
        Ok(file) => {
            let mut output = String::new();
            snapshot(&file).write(&mut output);
            println!("{output}");
        }
        Err(error) => {
            eprintln!("读取失败: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 断言转义、空值、布尔及非有限浮点符合 JSON 语义。
    #[test]
    fn json_encoder_preserves_values() {
        let mut out = String::new();
        Json::Array(vec![
            Json::Text("引号\"\\\n\u{0001}".into()),
            Json::Null,
            Json::Bool(false),
            Json::Number(f64::NAN),
            Json::Int(42),
        ])
        .write(&mut out);
        assert_eq!(out, "[\"引号\\\"\\\\\\n\\u0001\",null,false,null,42]");
    }
}
