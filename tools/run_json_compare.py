#!/usr/bin/env python3
"""构建 Java/Rust 快照工具，用常驻 server 进程批量对拍样本。"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
from dataclasses import dataclass

import compare_json


@dataclass
class Result:
    sample: str
    code: int
    message: str = ""


def command(name: str, fallback: Path | None = None) -> str:
    found = shutil.which(name)
    if found:
        return found
    if fallback is not None and fallback.exists():
        return str(fallback)
    raise RuntimeError(f"未找到工具: {name}")


def run_checked(args: list[str], cwd: Path, description: str) -> None:
    result = subprocess.run(args, cwd=cwd)
    if result.returncode != 0:
        raise RuntimeError(f"{description}失败，退出码 {result.returncode}")


def build_tools(repo: Path, output: Path) -> tuple[str, str]:
    output.mkdir(parents=True, exist_ok=True)
    classes = output / "classes"
    classes.mkdir(parents=True, exist_ok=True)

    user_profile = os.environ.get("USERPROFILE", "")
    maven_fallback = Path(user_profile) / "scoop" / "apps" / "maven" / "current" / "bin" / "mvn.cmd"
    maven = command("mvn.cmd", maven_fallback)
    upstream = repo / "mpxj"
    run_checked(
        [maven, "-q", "-DskipTests", "-Dmaven.javadoc.skip=true", "-f", str(upstream / "pom.xml"), "package"],
        repo,
        "Java 子模块源码构建",
    )

    upstream_jar = upstream / "target" / "mpxj-16.7.0.jar"
    lib = upstream / "lib"
    dependencies = sorted(lib.glob("*.jar"))
    if not upstream_jar.exists() or not lib.exists() or not dependencies:
        raise RuntimeError("Maven 构建后缺少 MPXJ jar 或 lib/ 依赖")
    classpath = os.pathsep.join(str(path) for path in [classes, upstream_jar, *dependencies])
    javac = command("javac")
    run_checked(
        [javac, "-encoding", "UTF-8", "-cp", classpath, "-d", str(classes), str(repo / "tools" / "JsonSnapshot.java")],
        repo,
        "Java 快照编译",
    )

    cargo = command("cargo")
    run_checked([cargo, "build", "--example", "json_snapshot", "--manifest-path", str(repo / "Cargo.toml")], repo, "Rust 快照编译")
    rust_exe = repo / "target" / "debug" / "examples" / ("json_snapshot.exe" if os.name == "nt" else "json_snapshot")
    if not rust_exe.exists():
        raise RuntimeError(f"Rust 快照程序不存在: {rust_exe}")
    return classpath, str(rust_exe)


def output_stem(sample: Path, repo: Path) -> str:
    try:
        display = sample.relative_to(repo).as_posix()
    except ValueError:
        display = sample.as_posix()
    safe = "".join("_" if char in ":/\\" else char for char in display)
    digest = hashlib.sha1(display.encode("utf-8")).hexdigest()[:10]
    return f"{safe}.{digest}"


def resolve_samples(repo: Path, samples: list[str]) -> tuple[list[tuple[str, str]], list[Result]]:
    """把样本解析成 (stem, 绝对路径)，无法定位的样本作为工具错误返回。"""
    tasks: list[tuple[str, str]] = []
    missing: list[Result] = []
    for sample in samples:
        sample_path = Path(sample)
        if not sample_path.is_absolute():
            sample_path = repo / sample_path
        try:
            sample_path = sample_path.resolve(strict=True)
        except FileNotFoundError:
            missing.append(Result(sample, 2, f"样本不存在: {sample}"))
            continue
        tasks.append((output_stem(sample_path, repo), str(sample_path)))
    return tasks, missing


def run_server(
    argv: list[str],
    tasks_file: Path,
    repo: Path,
    description: str,
) -> list[Result]:
    """启动常驻 server，把任务清单从 stdin 送入，等待其写完所有输出。"""
    try:
        with tasks_file.open("rb") as stdin:
            server = subprocess.run(
                argv,
                cwd=repo,
                stdin=stdin,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                check=False,
            )
    except OSError as error:
        return [Result(description, 2, str(error))]
    message = server.stdout.decode("utf-8", errors="replace").strip()
    if server.returncode != 0:
        return [Result(description, 2, f"{description}退出码 {server.returncode}: {message}")]
    if message:
        print(message)
    return []


def same_file(left, right) -> bool:
    """判断两个路径字符串是否指向同一个文件；用于校验 server 回写的 path。"""
    if not isinstance(left, str) or not isinstance(right, str) or not left or not right:
        return False
    try:
        left_path = str(Path(left).resolve())
        right_path = str(Path(right).resolve())
    except OSError:
        return False
    return os.path.normcase(left_path) == os.path.normcase(right_path)


def compare_sample(stem: str, sample: str, java_dir: Path, rust_dir: Path, all_fields: bool) -> Result:
    java_file = java_dir / f"{stem}.json"
    rust_file = rust_dir / f"{stem}.json"
    try:
        java_entry = compare_json.load_snapshot(java_file)
        rust_entry = compare_json.load_snapshot(rust_file)
    except (OSError, ValueError) as error:
        return Result(sample, 2, f"快照读取失败: {error}")
    for side, entry in (("Java", java_entry), ("Rust", rust_entry)):
        if not isinstance(entry, dict) or "path" not in entry:
            return Result(sample, 2, f"{side} 快照缺少 path 字段")
        if "error" in entry:
            return Result(sample, 2, f"{side} {entry['error']}")
    if not same_file(java_entry["path"], rust_entry["path"]):
        return Result(sample, 2, f"两侧 path 不一致: Java={java_entry['path']} Rust={rust_entry['path']}")
    if not same_file(java_entry["path"], sample):
        return Result(sample, 2, f"快照 path 与本轮样本不一致: {java_entry['path']} != {sample}")
    try:
        differences = compare_json.diff(java_entry["result"], rust_entry["result"], require_all_fields=all_fields)
    except ValueError as error:
        return Result(sample, 2, str(error))
    return Result(sample, 0 if not differences else 1, compare_json.format_differences(differences, 40))


def read_samples(repo: Path, values: list[str] | None, sample_list: Path | None) -> list[str]:
    if values and sample_list:
        raise RuntimeError("--samples 与 --sample-list 不能同时使用")
    if sample_list is not None:
        source = sample_list if sample_list.is_absolute() else repo / sample_list
        lines = source.read_text(encoding="utf-8-sig").splitlines()
    elif values:
        lines = values
    else:
        lines = (repo / "tools" / "json_compare_samples.txt").read_text(encoding="utf-8-sig").splitlines()
    samples = [line.strip() for line in lines if line.strip() and not line.lstrip().startswith("#")]
    if not samples:
        raise RuntimeError("没有待对拍的样本")
    return samples


def main() -> int:
    parser = argparse.ArgumentParser(description="并行执行 Java/Rust ProjectFile JSON 语义对拍")
    parser.add_argument("--samples", nargs="*", help="样本路径列表")
    parser.add_argument("--sample-list", type=Path, help="每行一个样本路径的文件")
    parser.add_argument("--all-fields", action="store_true", help="比较完整字段与扩展容器")
    parser.add_argument("--parallelism", type=int, default=1, help="每个 server 的内部线程数，默认 1")
    parser.add_argument("--quiet", action="store_true", help="只输出汇总")
    args = parser.parse_args()
    if not 1 <= args.parallelism <= 32:
        parser.error("--parallelism 必须在 1 到 32 之间")

    repo = Path(__file__).resolve().parent.parent
    samples = read_samples(repo, args.samples, args.sample_list)
    output = repo / "_tmp_json_compare"
    try:
        classpath, rust_exe = build_tools(repo, output)
    except RuntimeError as error:
        print(str(error), file=sys.stderr)
        return 2

    tasks, missing = resolve_samples(repo, samples)
    server_errors: list[Result] = []
    results: list[Result] = []
    if tasks:
        tasks_file = output / "tasks.txt"
        tasks_file.write_text("".join(f"{stem}\t{path}\n" for stem, path in tasks), encoding="utf-8")
        java_dir = output / "java"
        rust_dir = output / "rust"
        for directory in (java_dir, rust_dir):
            shutil.rmtree(directory, ignore_errors=True)
        common = ["--threads", str(args.parallelism)]
        snapshot_option = ["--all-fields"] if args.all_fields else []
        java_argv = ["java", "-cp", classpath, "JsonSnapshot", "--server", str(java_dir), *snapshot_option, *common]
        rust_argv = [rust_exe, "--server", str(rust_dir), *snapshot_option, *common]
        server_errors += run_server(java_argv, tasks_file, repo, "Java server")
        server_errors += run_server(rust_argv, tasks_file, repo, "Rust server")

        if not server_errors:
            with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, min(8, args.parallelism))) as executor:
                jobs = [
                    executor.submit(compare_sample, stem, sample, java_dir, rust_dir, args.all_fields)
                    for stem, sample in tasks
                ]
                results = [future.result() for future in jobs]
        else:
            results = [Result(sample, 2, "server 未生成快照") for _, sample in tasks]

    all_results = missing + server_errors + results
    for result in all_results:
        if result.code == 1:
            if not args.quiet:
                print(f"差异: {result.sample}\n{result.message}")
        elif result.code != 0:
            print(f"失败: {result.sample} {result.message}")
        elif not args.quiet:
            print(f"一致: {result.sample}")
    failed = sum(1 for result in all_results if result.code == 1)
    tool_errors = sum(1 for result in all_results if result.code not in (0, 1))
    print(f"原始 JSON: {output}")
    print(f"已对拍样本: {len(samples)}，存在差异: {failed}，工具错误: {tool_errors}，并行度: {args.parallelism}")
    return 1 if failed or tool_errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
