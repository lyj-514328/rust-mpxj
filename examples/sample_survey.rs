//! 批量解析仓库内全部真实样本，统计成功率。
//!
//! 用法：`cargo run --release --example sample_survey`
//!
//! 遍历 `mpxj/junit/data/` 下的全部 `.mpp` 与 `.mpx` 样本，逐个交给
//! [`MPPReader`] / [`MPXReader`] 解析，最后打印：
//! - 总数 / 成功 / 失败 / 成功率
//! - 失败原因归类（按去掉路径与数字后的错误文本聚合）
//! - 失败样本清单
//!
//! 说明：样本目录里有少量**故意损坏**的文件（MPXJ 自己的测试用，见 `corrupt` 等目录名），
//! 它们解析失败是预期行为。报告里会把这类样本单独标注。

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;

use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::mpx::locale_utility::MpxLocale;
use mpxj::mpx::mpx_reader::MPXReader;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::ProjectFile;

/// 最近一次 panic 的原因（供 `catch_unwind` 之后取用）。
static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

/// 一个样本的解析结果。
enum Outcome {
    Ok,
    /// 解析返回错误
    Error(String),
}

/// 把错误文本归一化，便于按原因归类：
/// 去掉绝对路径与具体数字，只保留错误的结构。
fn normalise(message: &str) -> String {
    let mut result = String::with_capacity(message.len());
    let mut last_was_digit = false;

    for ch in message.chars() {
        if ch.is_ascii_digit() {
            if !last_was_digit {
                result.push('#');
            }
            last_was_digit = true;
        } else {
            result.push(ch);
            last_was_digit = false;
        }
    }

    // 单引号内的内容多半是文件名/条目名，抹掉以免同类原因被拆散
    let mut collapsed = String::with_capacity(result.len());
    let mut in_quote = false;
    for ch in result.chars() {
        if ch == '\'' || ch == '"' {
            in_quote = !in_quote;
            collapsed.push(ch);
        } else if in_quote {
            continue;
        } else {
            collapsed.push(ch);
        }
    }

    collapsed
}

/// 判断路径是否位于「故意损坏」的样本目录下。
fn is_intentionally_corrupt(path: &Path) -> bool {
    let text = path.to_string_lossy().to_ascii_lowercase();
    text.contains("corrupt")
        || text.contains("password")
        || text.contains("encrypted")
        || text.contains("invalid")
}

/// 递归收集指定扩展名的样本文件。
fn collect(root: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, extension, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            out.push(path);
        }
    }
}

/// 解析单个 MPP 样本。
fn read_mpp(path: &Path) -> Outcome {
    let mut reader = MPPReader::new();
    match reader.read_file(path) {
        Ok(_) => Outcome::Ok,
        Err(error) => Outcome::Error(error.message().to_owned()),
    }
}

/// 从样本文件名推断 MPX 文件所用的区域设置。
///
/// MPX 的文本内容随微软项目的语言版本本地化，读取非英语版本必须先指定区域设置
/// （见 `mpxj/mkdocs/docs/howto-read-mpx.md` 的 “Locale” 一节）。样本文件名形如
/// `sample.de.mpx`，语言后缀即区域设置；没有语言后缀时用英文（`MPXReader` 的默认值）。
fn locale_for(path: &Path) -> MpxLocale {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");

    stem.rsplit('.')
        .next()
        .and_then(MpxLocale::get_instance)
        .unwrap_or(MpxLocale::En)
}

/// 解析单个 MPX 样本。
fn read_mpx(path: &Path) -> Outcome {
    let mut reader = MPXReader::new();
    // 对应 Java `LocaleTest.testReadGerman`：`reader.setLocale(Locale.GERMAN)`
    reader.set_locale(locale_for(path));
    match reader.read_file(path) {
        Ok(_) => Outcome::Ok,
        Err(error) => Outcome::Error(error.message().to_owned()),
    }
}

/// 统计一组样本。
fn survey(label: &str, root: &Path, extension: &str, reader: fn(&Path) -> Outcome) {
    let mut files = Vec::new();
    collect(root, extension, &mut files);
    files.sort();

    let (mut ok, mut corrupt_ok) = (0usize, 0usize);
    let mut reasons: HashMap<String, usize> = HashMap::new();
    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    let mut panics = 0usize;

    for path in &files {
        let outcome = catch_unwind(AssertUnwindSafe(|| reader(path)));

        match outcome {
            Ok(Outcome::Ok) => {
                ok += 1;
                if is_intentionally_corrupt(path) {
                    corrupt_ok += 1;
                }
            }
            Ok(Outcome::Error(message)) => {
                *reasons.entry(normalise(&message)).or_default() += 1;
                failures.push((path.clone(), message));
            }
            Err(_) => {
                panics += 1;
                let message = LAST_PANIC
                    .lock()
                    .ok()
                    .and_then(|mut slot| slot.take())
                    .unwrap_or_else(|| "<panic>".to_owned());
                *reasons.entry(message.clone()).or_default() += 1;
                failures.push((path.clone(), message));
            }
        }
    }

    let total = files.len();
    let failed = total - ok;
    println!();
    println!("=========== {label} ===========");
    println!("样本总数 : {total}");
    println!("解析成功 : {ok}");
    println!("解析失败 : {failed}  (其中 panic {panics})");
    if total > 0 {
        println!(
            "成功率   : {:.2}%",
            (ok as f64) * 100.0 / (total as f64)
        );
    }
    if corrupt_ok > 0 {
        println!("（成功样本中有 {corrupt_ok} 个位于 corrupt/password/encrypted/invalid 目录）");
    }

    if !reasons.is_empty() {
        println!();
        println!("--- 失败原因归类（按出现次数降序）---");
        let mut grouped: Vec<(String, usize)> = reasons.into_iter().collect();
        grouped.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (reason, count) in &grouped {
            println!("{count:>5}  {reason}");
        }

        println!();
        println!("--- 失败样本清单（最多 40 条）---");
        let base = root.parent().unwrap_or(root);
        for (path, message) in failures.iter().take(40) {
            let relative = path.strip_prefix(base).unwrap_or(path);
            println!("  {} :: {}", relative.display(), message);
        }
        if failures.len() > 40 {
            println!("  ... 其余 {} 条略", failures.len() - 40);
        }
    }
}

fn main() {
    // 批量跑样本时只记录 panic 原因、不打印回溯，避免刷屏
    std::panic::set_hook(Box::new(|info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<非字符串 panic 载荷>".to_owned());

        let location = info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "<未知位置>".to_owned());

        if let Ok(mut slot) = LAST_PANIC.lock() {
            *slot = Some(format!("{payload}  @ {location}"));
        }
    }));

    let data = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data");

    survey("MPP 样本", &data, "mpp", read_mpp);
    survey("MPX 样本", &data, "mpx", read_mpx);

    // 还原 panic hook，保持退出时行为正常
    let _ = std::panic::take_hook();

    println!();
    println!("统计完成。");
    let _: Option<Rc<RefCell<ProjectFile>>> = None;
}
