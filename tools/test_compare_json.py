"""快照比较器的结构、类型和实体对齐回归测试。"""

import unittest
from decimal import Decimal

from compare_json import MISSING, compare_values


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


if __name__ == "__main__":
    unittest.main()
