"""快照比较器的结构、类型和实体对齐回归测试。"""

import json
import tempfile
import unittest
from decimal import Decimal
from pathlib import Path

from compare_json import MISSING, compare_values, diff, format_differences, unwrap_snapshot, validate_snapshot
from run_json_compare import compare_sample, output_stem, resolve_samples, same_file


class CompareJsonTest(unittest.TestCase):
    def test_object_order_and_number_representation(self):
        self.assertEqual(compare_values({"a": 1, "b": Decimal("1.5")},
                                        {"b": Decimal("1.50"), "a": Decimal("1.0")}), [])

    def test_order_independent_entities(self):
        left = {"tasks": [{"unique_id": 1, "name": "甲"}, {"unique_id": 2, "name": "乙"}]}
        right = {"tasks": list(reversed(left["tasks"]))}
        self.assertEqual(compare_values(left, right), [])

    def test_missing_is_not_null(self):
        differences = compare_values({"a": None}, {})
        self.assertIs(differences[0][2], MISSING)

    def test_bool_is_not_number(self):
        self.assertEqual(len(compare_values({"a": True}, {"a": 1})), 1)

    def test_duplicate_ids_are_not_dropped(self):
        left = {"tasks": [{"unique_id": 1, "name": "甲"}, {"unique_id": 1, "name": "乙"}]}
        right = {"tasks": left["tasks"][:1]}
        self.assertIn("#2", compare_values(left, right)[0][0])

    def test_missing_id_uses_common_name_and_reports_id_difference(self):
        left = {"calendars": [{"unique_id": 1, "name": "Standard", "type": 2}]}
        right = {"calendars": [{"unique_id": None, "name": "Standard", "type": 2}]}
        differences = compare_values(left, right)
        self.assertEqual(len(differences), 1)
        self.assertEqual(differences[0][0], "$.calendars[name=Standard#1].unique_id")

    def test_all_fields_requires_every_entity_value_list(self):
        snapshot = self.full_snapshot()
        del snapshot["tasks"][0]["field_values"]
        with self.assertRaisesRegex(ValueError, "缺少字段值"):
            validate_snapshot(snapshot, "Java", True)

    def test_all_fields_requires_catalog_and_exclusions(self):
        snapshot = self.full_snapshot()
        del snapshot["field_catalog"]
        with self.assertRaisesRegex(ValueError, "扩展容器"):
            validate_snapshot(snapshot, "Rust", True)

    def test_all_fields_requires_extended_containers(self):
        snapshot = self.full_snapshot()
        del snapshot["data_links"]
        with self.assertRaisesRegex(ValueError, "扩展容器"):
            validate_snapshot(snapshot, "Java", True)

    def test_all_fields_rejects_duplicate_or_unlisted_values(self):
        snapshot = self.full_snapshot()
        snapshot["tasks"][0]["field_values"].append({"name": "UNLISTED", "value": 1})
        with self.assertRaisesRegex(ValueError, "不在目录"):
            validate_snapshot(snapshot, "Rust", True)
        snapshot["tasks"][0]["field_values"] = [{"name": "ID", "value": 1}] * 2
        with self.assertRaisesRegex(ValueError, "重复"):
            validate_snapshot(snapshot, "Rust", True)

    def test_all_fields_rejects_excluded_field_value(self):
        snapshot = self.full_snapshot()
        snapshot["field_exclusions"]["tasks"] = ["ID"]
        with self.assertRaisesRegex(ValueError, "被排除"):
            validate_snapshot(snapshot, "Java", True)

    def test_all_fields_requires_calendar_and_resource_details(self):
        snapshot = self.full_snapshot()
        snapshot["calendars"].append({"details": {"days": [None] * 7, "work_weeks": [], "exceptions": []}})
        snapshot["resources"].append({"details": {"availability": [], "cost_rate_tables": [None] * 5}, "field_values": []})
        validate_snapshot(snapshot, "Rust", True)
        del snapshot["calendars"][0]["details"]
        with self.assertRaisesRegex(ValueError, "缺少对象详情"):
            validate_snapshot(snapshot, "Rust", True)

    def test_all_fields_requires_display_containers(self):
        snapshot = self.full_snapshot()
        del snapshot["tables"]
        with self.assertRaisesRegex(ValueError, "扩展容器"):
            validate_snapshot(snapshot, "Java", True)

    @staticmethod
    def full_snapshot():
        groups = ("properties", "tasks", "resources", "assignments")
        snapshot = {"properties": {"field_values": [{"name": "ID", "value": 1}]},
                    "tasks": [{"field_values": [{"name": "ID", "value": 1}]}],
                    "resources": [], "assignments": [], "relations": [], "calendars": []}
        snapshot["field_catalog"] = {group: ["ID"] for group in groups}
        snapshot["field_exclusions"] = {group: [] for group in groups}
        snapshot.update({"views": [], "tables": [], "groups": [], "task_filters": [],
                         "resource_filters": [], "view_state": None, "user_defined_fields": [],
                         "work_contours": [], "data_links": []})
        return snapshot


class RunnerTest(unittest.TestCase):
    def test_diff_and_format_reuse_cli_semantics(self):
        snapshot = {"properties": [], "tasks": [], "resources": [], "assignments": [],
                    "relations": [], "calendars": []}
        self.assertEqual(diff(snapshot, snapshot), [])
        other = dict(snapshot, properties={"unique_id": 2, "name": None, "author": None, "company": None,
                                           "start": None, "finish": None, "default_calendar_unique_id": None})
        baseline = dict(snapshot, properties={"unique_id": 1, "name": None, "author": None, "company": None,
                                              "start": None, "finish": None, "default_calendar_unique_id": None})
        differences = diff(baseline, other)
        self.assertEqual(len(differences), 1)
        self.assertIn("$.properties.unique_id", format_differences(differences, 40))

    def test_output_stem_is_unique_and_filesystem_safe(self):
        repo = Path("D:/repo")
        stem = output_stem(Path("D:/repo/mpxj/junit/data/sample.de.mpx"), repo)
        self.assertNotIn(":", stem)
        self.assertNotIn("/", stem)
        self.assertNotEqual(stem, output_stem(Path("D:/repo/mpxj/junit/data/other.mpx"), repo))

    def test_resolve_samples_reports_missing_file(self):
        tasks, missing = resolve_samples(Path("D:/repo"), ["D:/repo/no-such-sample.mpp"])
        self.assertEqual(tasks, [])
        self.assertEqual(missing[0].code, 2)

    def test_unwrap_snapshot_accepts_both_contracts(self):
        snapshot = {"tasks": []}
        self.assertIs(unwrap_snapshot(snapshot), snapshot)
        self.assertEqual(unwrap_snapshot({"path": "x.mpp", "result": snapshot}), snapshot)
        with self.assertRaisesRegex(ValueError, "boom"):
            unwrap_snapshot({"path": "x.mpp", "error": "读取失败: boom"})

    def test_same_file_normalizes_separators_and_case(self):
        self.assertTrue(same_file("D:/repo/a.mpp", "D:\\repo\\a.mpp"))
        self.assertTrue(same_file("D:\\repo\\A.MPP", "D:\\repo\\a.mpp"))
        self.assertFalse(same_file("D:/repo/a.mpp", "D:/repo/b.mpp"))
        self.assertFalse(same_file(None, "D:/repo/a.mpp"))

    def test_compare_sample_rejects_path_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            java_dir = root / "java"
            rust_dir = root / "rust"
            java_dir.mkdir()
            rust_dir.mkdir()
            result = {"properties": [], "tasks": [], "resources": [], "assignments": [],
                      "relations": [], "calendars": []}
            sample = str(root / "x.mpp")
            (java_dir / "s.json").write_text(json.dumps({"path": sample, "result": result}), encoding="utf-8")
            (rust_dir / "s.json").write_text(json.dumps({"path": str(root / "y.mpp"), "result": result}), encoding="utf-8")
            self.assertEqual(compare_sample("s", sample, java_dir, rust_dir, False).code, 2)

            (rust_dir / "s.json").write_text(json.dumps({"path": sample, "result": result}), encoding="utf-8")
            self.assertEqual(compare_sample("s", str(root / "z.mpp"), java_dir, rust_dir, False).code, 2)

    def test_compare_sample_detects_wrapper_path_and_errors(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            java_dir = root / "java"
            rust_dir = root / "rust"
            java_dir.mkdir()
            rust_dir.mkdir()
            result = {"properties": [], "tasks": [], "resources": [], "assignments": [],
                      "relations": [], "calendars": []}
            entry = {"path": "x.mpp", "result": result}
            (java_dir / "s.json").write_text(json.dumps(entry), encoding="utf-8")
            (rust_dir / "s.json").write_text(json.dumps(entry), encoding="utf-8")
            self.assertEqual(compare_sample("s", "x.mpp", java_dir, rust_dir, False).code, 0)

            (rust_dir / "s.json").write_text(json.dumps({"result": result}), encoding="utf-8")
            self.assertEqual(compare_sample("s", "x.mpp", java_dir, rust_dir, False).code, 2)

            (rust_dir / "s.json").write_text(json.dumps({"path": "x.mpp", "error": "读取失败: boom"}), encoding="utf-8")
            result = compare_sample("s", "x.mpp", java_dir, rust_dir, False)
            self.assertEqual(result.code, 2)
            self.assertIn("boom", result.message)


if __name__ == "__main__":
    unittest.main()
