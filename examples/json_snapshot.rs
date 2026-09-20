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
use mpxj::assignment_field::AssignmentField;
use mpxj::duration::Duration;
use mpxj::field_container::FieldContainer;
use mpxj::field_key::{CustomFieldKey, FieldKey};
use mpxj::field_type::FieldType;
use mpxj::field_value::FieldValue;
use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::mpx::locale_utility::MpxLocale;
use mpxj::mpx::mpx_reader::MPXReader;
use mpxj::project_field::ProjectField;
use mpxj::project_entity_with_id::ProjectEntityWithID;
use mpxj::project_entity_with_unique_id::ProjectEntityWithUniqueID;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::resource_field::ResourceField;
use mpxj::task_field::TaskField;
use mpxj::user_defined_field::UserDefinedField;

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

/// 将字段表中的具体值转换为两侧共有的类型表示；未实现的类型必须报错。
fn field_value(value: FieldValue) -> Result<Json, String> {
    Ok(match value {
        FieldValue::Integer(v) => Json::Int(v),
        FieldValue::Short(v) => Json::Int(i32::from(v)),
        FieldValue::Double(v) => Json::Number(v),
        FieldValue::Boolean(v) => Json::Bool(v),
        FieldValue::String(v) => Json::Text(v),
        FieldValue::Bytes(v) => Json::Text(v.iter().map(|byte| format!("{byte:02x}")).collect()),
        FieldValue::Date(v) => Json::Text(v.format("%Y-%m-%d").to_string()),
        FieldValue::Time(v) => Json::Text(v.format("%H:%M:%S%.9f").to_string()),
        FieldValue::DateTime(v) => date(Some(v)),
        FieldValue::Duration(v) => duration(Some(v)),
        FieldValue::Rate(v) => Json::Object(vec![
            ("amount", Json::Number(v.get_amount())),
            ("unit", Json::Int(v.get_units() as i32)),
        ]),
        FieldValue::Uuid(v) => Json::Text(v.to_string()),
        FieldValue::TimeUnit(v) => Json::Int(v as i32),
        FieldValue::RateSource(v) => Json::Text(format!("{v:?}").to_ascii_lowercase()),
        FieldValue::ResourceRequestType(v) => Json::Int(v as i32),
        FieldValue::WorkContour(v) => Json::Object(vec![
            ("unique_id", integer(v.get_unique_id())),
            ("name", string(v.get_name())),
        ]),
        FieldValue::Notes(v) => Json::Text(v.to_string()),
        FieldValue::Steps(v) => Json::Array(v.try_borrow().map_err(|_| "步骤列表借用冲突")?.iter().map(|step| {
            Json::Object(vec![
                ("unique_id", integer(step.get_unique_id())),
                ("name", string(step.get_name())),
                ("percent_complete", number(step.get_percent_complete())),
                ("sequence_number", integer(step.get_sequence_number())),
                ("weight", number(step.get_weight())),
                ("description", Json::Text(step.get_description())),
            ])
        }).collect()),
        FieldValue::Relations(v) => Json::Array(v.iter().map(|relation| {
            let relation = relation.try_borrow().map_err(|_| "关系借用冲突")?;
            Ok(Json::Object(vec![
                ("unique_id", integer(relation.get_unique_id())),
                ("predecessor_unique_id", integer(relation.get_predecessor_task().and_then(|t| t.try_borrow().ok().and_then(|t| t.get_unique_id())))),
                ("successor_unique_id", integer(relation.get_successor_task().and_then(|t| t.try_borrow().ok().and_then(|t| t.get_unique_id())))),
                ("type", Json::Int(relation.get_type() as i32)),
                ("lag", duration(relation.get_lag())),
            ]))
        }).collect::<Result<Vec<_>, String>>()?),
        FieldValue::Resource(v) => Json::Object(vec![("unique_id", integer(v.try_borrow().map_err(|_| "资源借用冲突")?.get_unique_id()))]),
        FieldValue::TimeRanges(v) => Json::Array(v.iter().map(|range| Json::Object(vec![
            ("start", date(range.get_start())),
            ("end", date(range.get_end())),
        ])).collect()),
        FieldValue::TimephasedWork(v) => Json::Array(v.iter().map(|item| Json::Object(vec![
            ("start", date(item.get_start())),
            ("finish", date(item.get_finish())),
            ("total_amount", duration(item.get_total_amount())),
            ("amount_per_hour", duration(item.get_amount_per_hour())),
        ])).collect()),
        FieldValue::TimephasedCost(v) => Json::Array(v.iter().map(|item| Json::Object(vec![
            ("start", date(item.get_start())),
            ("finish", date(item.get_finish())),
            ("total_amount", number(item.get_total_amount())),
            ("amount_per_hour", number(item.get_amount_per_hour())),
        ])).collect()),
    })
}

/// 枚举清单独立于实际值输出，避免两边都漏掉同一个空字段时误判一致。
fn field_catalog<F: FieldType>(fields: &[F], custom: &[(String, FieldKey)]) -> Json {
    let mut names: Vec<_> = fields.iter().map(FieldType::name).collect();
    names.extend(custom.iter().map(|(name, _)| name.as_str()));
    names.sort_unstable();
    Json::Array(names.into_iter().map(|name| Json::Text(name.to_owned())).collect())
}

/// P6 专用的 Code/Expense 字段不在本仓库的 MPX/MPP reader 范围内。
fn excluded_field(name: &str) -> bool {
    matches!(name,
        "PROJECT_CODE_VALUES" | "RESOURCE_CODE_VALUES" | "ROLE_CODE_VALUES" |
        "RESOURCE_ASSIGNMENT_CODE_VALUES" | "ACTIVITY_CODE_VALUES" | "EXPENSE_ITEMS")
}

/// 将明确排除的字段写入快照，防止只靠缺失值误报一致。
fn field_exclusions<F: FieldType>(fields: &[F]) -> Json {
    let mut names: Vec<_> = fields.iter().map(FieldType::name).filter(|name| excluded_field(name)).collect();
    names.sort_unstable();
    Json::Array(names.into_iter().map(|name| Json::Text(name.to_owned())).collect())
}

/// 只输出非空字段，空字段由双方相同的枚举清单约束。
fn field_values<T: FieldContainer, F: FieldType + Copy + Into<FieldKey>>(
    container: &T,
    fields: &[F],
    custom: &[(String, FieldKey)],
) -> Result<Json, String> {
    let mut values: Vec<(String, Json)> = Vec::new();
    for field in fields {
        if excluded_field(field.name()) { continue; }
        let key = (*field).into();
        if key == FieldKey::Project(ProjectField::CurrentDate)
            && container.get_cached_value(key).is_none()
        {
            values.push((field.name().to_owned(), Json::Text("<dynamic>".to_owned())));
            continue;
        }
        if let Some(value) = container.get(key) {
            values.push((field.name().to_owned(), field_value(value).map_err(|e| format!("{}: {e}", field.name()))?));
        }
    }
    for (name, key) in custom {
        if let Some(value) = container.get(*key) {
            values.push((name.clone(), field_value(value).map_err(|e| format!("{name}: {e}"))?));
        }
    }
    values.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    Ok(Json::Array(values.into_iter().map(|(name, value)| {
        Json::Object(vec![("name", Json::Text(name)), ("value", value)])
    }).collect()))
}

/// 自定义字段以 unique ID 命名，避免本地化名称及重名影响对齐。
fn user_fields(fields: Vec<Rc<RefCell<UserDefinedField>>>) -> Vec<(String, FieldKey)> {
    fields.into_iter().filter_map(|field| {
        let id = field.try_borrow().ok()?.get_unique_id();
        Some((format!("CUSTOM:{id}"), FieldKey::Custom(CustomFieldKey::from_field(&field))))
    }).collect()
}

/// 给现有 ProjectFile 快照附加完整的内置字段清单及非空字段值。
fn append_all_fields(file: &Rc<RefCell<ProjectFile>>, snapshot: &mut Json) -> Result<(), String> {
    let Json::Object(root) = snapshot else { return Err("快照根节点不是对象".to_owned()); };
    let (project_custom, task_custom, resource_custom, assignment_custom) = {
        let file_ref = file.borrow();
        let custom = file_ref.get_user_defined_fields();
        (
            user_fields(custom.get_project_fields()),
            user_fields(custom.get_task_fields()),
            user_fields(custom.get_resource_fields()),
            user_fields(custom.get_assignment_fields()),
        )
    };
    root.push(("field_catalog", Json::Object(vec![
        ("properties", field_catalog(&ProjectField::ALL, &project_custom)),
        ("tasks", field_catalog(&TaskField::ALL, &task_custom)),
        ("resources", field_catalog(&ResourceField::ALL, &resource_custom)),
        ("assignments", field_catalog(&AssignmentField::ALL, &assignment_custom)),
    ])));
    root.push(("field_exclusions", Json::Object(vec![
        ("properties", field_exclusions(&ProjectField::ALL)),
        ("tasks", field_exclusions(&TaskField::ALL)),
        ("resources", field_exclusions(&ResourceField::ALL)),
        ("assignments", field_exclusions(&AssignmentField::ALL)),
    ])));

    let file_ref = file.borrow();
    let mut properties = Some(field_values(file_ref.get_project_properties(), &ProjectField::ALL, &project_custom)?);
    if let Some(Json::Array(values)) = &mut properties {
        let mut custom: Vec<_> = file_ref.get_project_properties().get_custom_properties().iter().collect();
        custom.sort_unstable_by_key(|(name, _)| *name);
        let entries = custom.into_iter().map(|(name, value)| {
            Ok(Json::Object(vec![
                ("name", Json::Text(name.clone())),
                ("value", field_value(value.clone())?),
            ]))
        }).collect::<Result<Vec<_>, String>>()?;
        values.push(Json::Object(vec![
            ("name", Json::Text("CUSTOM_PROPERTIES".to_owned())),
            ("value", Json::Array(entries)),
        ]));
    }
    if let Some(id) = file_ref.get_project_properties().get_default_calendar_unique_id() {
        if let Some(Json::Array(values)) = &mut properties {
            let present = values.iter().any(|value| match value {
                Json::Object(fields) => matches!(fields.first(), Some(("name", Json::Text(name))) if name == "DEFAULT_CALENDAR_UNIQUE_ID"),
                _ => false,
            });
            if !present {
                values.push(Json::Object(vec![
                    ("name", Json::Text("DEFAULT_CALENDAR_UNIQUE_ID".to_owned())),
                    ("value", Json::Int(id)),
                ]));
            }
        }
    }
    let tasks: Vec<_> = file_ref.get_tasks().iter().cloned().collect();
    let resources: Vec<_> = file_ref.get_resources().iter().cloned().collect();
    let assignments: Vec<_> = file_ref.get_resource_assignments().iter().cloned().collect();
    drop(file_ref);

    for (key, data) in root.iter_mut() {
        match (*key, data) {
            ("properties", Json::Object(entry)) => entry.push((
                "field_values",
                properties.take().ok_or("项目字段已写入")?,
            )),
            ("tasks", Json::Array(entries)) => {
                for (entry, task) in entries.iter_mut().zip(tasks.iter()) {
                    let Json::Object(entry) = entry else { return Err("任务快照不是对象".to_owned()); };
                    entry.push(("field_values", field_values(&*task.borrow(), &TaskField::ALL, &task_custom)?));
                }
            }
            ("resources", Json::Array(entries)) => {
                for (entry, resource) in entries.iter_mut().zip(resources.iter()) {
                    let Json::Object(entry) = entry else { return Err("资源快照不是对象".to_owned()); };
                    entry.push(("field_values", field_values(&*resource.borrow(), &ResourceField::ALL, &resource_custom)?));
                }
            }
            ("assignments", Json::Array(entries)) => {
                for (entry, assignment) in entries.iter_mut().zip(assignments.iter()) {
                    let Json::Object(entry) = entry else { return Err("分配快照不是对象".to_owned()); };
                    entry.push(("field_values", field_values(&*assignment.borrow(), &AssignmentField::ALL, &assignment_custom)?));
                }
            }
            _ => {}
        }
    }
    Ok(())
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
    let all_fields = match env::args_os().nth(2).as_deref() {
        None => false,
        Some(value) if value == "--all-fields" => true,
        Some(_) => {
            eprintln!("用法: json_snapshot <file.mpx|file.mpp> [--all-fields]");
            std::process::exit(2);
        }
    };
    match read(Path::new(&path)) {
        Ok(file) => {
            let mut output = String::new();
            let mut value = snapshot(&file);
            if all_fields {
                if let Err(error) = append_all_fields(&file, &mut value) {
                    eprintln!("字段快照失败: {error}");
                    std::process::exit(2);
                }
            }
            value.write(&mut output);
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
