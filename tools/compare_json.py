"""按结构比较两侧 ProjectFile 快照，并报告精确的差异路径。"""

import argparse
import json
import sys
from collections import Counter
from decimal import Decimal
from pathlib import Path


KEYS = {
    "tasks": (("unique_id",), ("id",)),
    "resources": (("unique_id",), ("id",)),
    "assignments": (("unique_id",), ("task_unique_id", "resource_unique_id")),
    "relations": (("unique_id",), ("predecessor_unique_id", "successor_unique_id", "type")),
    "calendars": (("unique_id",), ("name",)),
    "field_values": (("name",),),
}
MISSING = object()
FIELD_GROUPS = ("properties", "tasks", "resources", "assignments")


def validate_snapshot(snapshot, side, require_all_fields=False):
    """拒绝不完整的字段快照，避免双方同时漏字段时误报一致。"""
    required = {"properties", "tasks", "resources", "assignments", "relations", "calendars"}
    if not isinstance(snapshot, dict) or not required <= snapshot.keys():
        missing = sorted(required - snapshot.keys()) if isinstance(snapshot, dict) else "根节点不是对象"
        raise ValueError(f"{side} 快照缺少必需容器: {missing}")
    if not require_all_fields:
        return
    catalogs = snapshot.get("field_catalog")
    exclusions = snapshot.get("field_exclusions")
    if not isinstance(catalogs, dict) or not isinstance(exclusions, dict):
        raise ValueError(f"{side} 缺少全字段目录或排除项")
    for group in FIELD_GROUPS:
        catalog = catalogs.get(group)
        excluded = exclusions.get(group)
        if not isinstance(catalog, list) or not catalog or any(not isinstance(name, str) or not name for name in catalog):
            raise ValueError(f"{side}.{group} 字段目录无效")
        if len(set(catalog)) != len(catalog):
            raise ValueError(f"{side}.{group} 字段目录包含重复名称")
        if not isinstance(excluded, list) or any(not isinstance(name, str) for name in excluded):
            raise ValueError(f"{side}.{group} 排除项无效")
        if len(set(excluded)) != len(excluded) or not set(excluded) <= set(catalog):
            raise ValueError(f"{side}.{group} 排除项重复或不在字段目录中")
        entities = [snapshot[group]] if group == "properties" else snapshot[group]
        if not isinstance(entities, list) or any(not isinstance(entity, dict) for entity in entities):
            raise ValueError(f"{side}.{group} 实体容器无效")
        for index, entity in enumerate(entities):
            values = entity.get("field_values")
            if not isinstance(values, list):
                raise ValueError(f"{side}.{group}[{index}] 缺少字段值")
            names = []
            for item in values:
                if not isinstance(item, dict) or "name" not in item or "value" not in item or not isinstance(item["name"], str):
                    raise ValueError(f"{side}.{group}[{index}] 字段条目无效")
                names.append(item["name"])
            if len(set(names)) != len(names) or not set(names) <= set(catalog) - set(excluded):
                raise ValueError(f"{side}.{group}[{index}] 字段值重复、被排除或不在目录中")


def matching_fields(java, rust, collection):
    """优先用双方都填充的唯一键；一侧缺 ID 时退到名称等备用键。"""
    for fields in KEYS[collection]:
        left = [tuple(item.get(field) for field in fields) for item in java]
        right = [tuple(item.get(field) for field in fields) for item in rust]
        if all(all(value is not None for value in key) for key in left + right):
            if len(set(left)) == len(left) and len(set(right)) == len(right):
                return fields
    return KEYS[collection][-1]


def align(items, fields):
    """重复标识保留原始顺序，并显式附加序号以免静默覆盖。"""
    seen = Counter()
    result = {}
    for item in items:
        key = tuple((field, item.get(field)) for field in fields)
        seen[key] += 1
        result[(key, seen[key])] = item
    return result


def compare_values(java, rust, path="$", differences=None, limit=100):
    """递归比较 JSON 语义：对象键无序、数字按数值、列表按实体身份。"""
    if differences is None:
        differences = []
    if len(differences) >= limit:
        return differences
    if java is MISSING or rust is MISSING:
        differences.append((path, java, rust))
    elif isinstance(java, dict) and isinstance(rust, dict):
        for key in sorted(java.keys() | rust.keys()):
            compare_values(java.get(key, MISSING), rust.get(key, MISSING),
                           f"{path}.{key}", differences, limit)
    elif isinstance(java, list) and isinstance(rust, list):
        collection = path.rsplit(".", 1)[-1]
        if collection in KEYS and all(isinstance(x, dict) for x in java + rust):
            fields = matching_fields(java, rust, collection)
            left, right = align(java, fields), align(rust, fields)
            for key in sorted(left.keys() | right.keys(), key=str):
                identity, occurrence = key
                suffix = ",".join(f"{k}={v}" for k, v in identity)
                compare_values(left.get(key, MISSING), right.get(key, MISSING),
                               f"{path}[{suffix}#{occurrence}]", differences, limit)
        else:
            for index in range(max(len(java), len(rust))):
                compare_values(java[index] if index < len(java) else MISSING,
                               rust[index] if index < len(rust) else MISSING,
                               f"{path}[{index}]", differences, limit)
    elif type(java) is bool or type(rust) is bool:
        if type(java) is not type(rust) or java != rust:
            differences.append((path, java, rust))
    elif isinstance(java, (int, Decimal)) and isinstance(rust, (int, Decimal)):
        if java != rust:
            differences.append((path, java, rust))
    elif type(java) is not type(rust) or java != rust:
        differences.append((path, java, rust))
    return differences


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("java", type=Path)
    parser.add_argument("rust", type=Path)
    parser.add_argument("--limit", type=int, default=40)
    parser.add_argument("--require-all-fields", action="store_true")
    args = parser.parse_args()
    try:
        with args.java.open(encoding="utf-8-sig") as source:
            java = json.load(source, parse_float=Decimal)
        with args.rust.open(encoding="utf-8-sig") as source:
            rust = json.load(source, parse_float=Decimal)
        for side, value in (("Java", java), ("Rust", rust)):
            validate_snapshot(value, side, args.require_all_fields)
    except (OSError, ValueError) as error:
        print(f"快照读取失败: {error}", file=sys.stderr)
        return 2
    differences = compare_values(java, rust, limit=args.limit + 1)
    if not differences:
        print("一致：当前快照范围内的 JSON 语义相同")
        return 0
    print(f"不一致：以下列出前 {min(args.limit, len(differences))} 处字段差异" +
          ("（尚有更多）" if len(differences) > args.limit else ""))
    for path, left, right in differences[:args.limit]:
        format_value = lambda v: "<缺失>" if v is MISSING else repr(v)
        print(f"  {path}\n    Java: {format_value(left)}\n    Rust: {format_value(right)}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
