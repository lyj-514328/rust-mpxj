#!/usr/bin/env python3
"""构建 Java/Rust 快照工具，并行执行样本语义对拍。"""

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


def run_sample(
    sample: str,
    repo: Path,
    classpath: str,
    rust_exe: str,
    output: Path,
    comparator: Path,
    all_fields: bool,
) -> Result:
    sample_path = Path(sample)
    if not sample_path.is_absolute():
        sample_path = repo / sample_path
    try:
        sample_path = sample_path.resolve(strict=True)
    except FileNotFoundError:
        return Result(sample, 2, f"样本不存在: {sample}")

    stem = output_stem(sample_path, repo)
    java_json = output / f"{stem}.java.json"
    rust_json = output / f"{stem}.rust.json"
    compare_log = output / f"{stem}.compare.log"
    snapshot_option = ["--all-fields"] if all_fields else []
    try:
        java = subprocess.run(
            ["java", "-cp", classpath, "JsonSnapshot", str(sample_path), *snapshot_option],
            cwd=repo,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if java.returncode != 0:
            return Result(sample, 2, f"Java 读取失败，退出码 {java.returncode}")
        java_json.write_bytes(java.stdout)

        rust = subprocess.run(
            [rust_exe, str(sample_path), *snapshot_option],
            cwd=repo,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if rust.returncode != 0:
            return Result(sample, 2, f"Rust 读取失败，退出码 {rust.returncode}")
        rust_json.write_bytes(rust.stdout)

        compare_option = ["--require-all-fields"] if all_fields else []
        comparison = subprocess.run(
            [sys.executable, "-X", "utf8", "-B", str(comparator), str(java_json), str(rust_json), *compare_option],
            cwd=repo,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        message = comparison.stdout.decode("utf-8", errors="replace").strip()
        compare_log.write_text(message + ("\n" if message else ""), encoding="utf-8")
        return Result(sample, comparison.returncode, message)
    except OSError as error:
        return Result(sample, 2, str(error))


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
    parser.add_argument("--parallelism", type=int, default=1, help="并发样本数，默认 1")
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

    comparator = repo / "tools" / "compare_json.py"
    failed = 0
    tool_errors = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallelism) as executor:
        jobs = [
            executor.submit(run_sample, sample, repo, classpath, rust_exe, output, comparator, args.all_fields)
            for sample in samples
        ]
        for future in concurrent.futures.as_completed(jobs):
            result = future.result()
            if result.code == 1:
                failed += 1
                if not args.quiet:
                    print(f"差异: {result.sample}\n{result.message}")
            elif result.code != 0:
                tool_errors += 1
                print(f"失败: {result.sample} {result.message}")
            elif not args.quiet:
                print(f"一致: {result.sample}")

    print(f"原始 JSON: {output}")
    print(f"已对拍样本: {len(samples)}，存在差异: {failed}，工具错误: {tool_errors}，并行度: {args.parallelism}")
    return 1 if failed or tool_errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
