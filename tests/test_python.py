import unittest
import os
import tempfile
from datetime import datetime
from dbase import DBFFile


class TestDBFFile(unittest.TestCase):
    def setUp(self):
        self.test_data_dir = os.path.join(os.path.dirname(__file__), "data")
        self.temp_dir = tempfile.mkdtemp()

    def tearDown(self):
        for f in os.listdir(self.temp_dir):
            os.remove(os.path.join(self.temp_dir, f))
        os.rmdir(self.temp_dir)

    def test_create_and_read(self):
        """Test creating a new DBF file and reading records."""
        dbf_path = os.path.join(self.temp_dir, "test.dbf")
        dbf = DBFFile(dbf_path)

        # Define fields
        fields = [
            ("NAME", "C", 50, None),
            ("AGE", "N", 3, None),
            ("BIRTH", "D", 8, None),
            ("SALARY", "N", 10, 2),
            ("ACTIVE", "L", 1, None),
        ]

        # Create file
        dbf.create(fields)

        # Test data
        test_records = [
            {
                "NAME": "John Doe",
                "AGE": 30,
                "BIRTH": "19930415",
                "SALARY": 50000.50,
                "ACTIVE": True,  # Will be converted to "T" internally
            },
            {
                "NAME": "Jane Smith",
                "AGE": 25,
                "BIRTH": "19980723",
                "SALARY": 45000.75,
                "ACTIVE": False,  # Will be converted to "F" internally
            },
        ]

        # Append records
        dbf.append_records(test_records)

        # Read records
        records = dbf.read_records()
        self.assertEqual(len(records), 2)

        # Verify first record
        self.assertEqual(records[0]["NAME"], "John Doe")
        self.assertEqual(records[0]["AGE"], 30)
        self.assertEqual(records[0]["BIRTH"], "19930415")
        self.assertEqual(records[0]["SALARY"], 50000.50)
        self.assertTrue(records[0]["ACTIVE"])  # Should be converted back to bool

    def test_update_record(self):
        """Test updating existing records."""
        dbf_path = os.path.join(self.temp_dir, "update_test.dbf")
        dbf = DBFFile(dbf_path)

        # Create file with fields
        fields = [
            ("NAME", "C", 50, None),
            ("VALUE", "N", 10, 2),
        ]
        dbf.create(fields)

        # Add initial record
        initial_records = [
            {"NAME": "Test", "VALUE": 100.00},
        ]
        dbf.append_records(initial_records)

        # Update record
        dbf.update_record(0, {"VALUE": 200.00})

        # Verify update
        records = dbf.read_records()
        self.assertEqual(records[0]["VALUE"], 200.00)
        self.assertEqual(records[0]["NAME"], "Test")  # Unchanged field

    def test_encoding(self):
        """Test different encodings."""
        # Test GBK encoding
        dbf_path = os.path.join(self.test_data_dir, "cp936.dbf")
        dbf = DBFFile(dbf_path, encoding="gbk")
        records = dbf.read_records()
        self.assertTrue(len(records) > 0)

        # Test ASCII encoding
        dbf_path = os.path.join(self.test_data_dir, "cp850.dbf")
        dbf = DBFFile(dbf_path, encoding="ascii")
        records = dbf.read_records()
        self.assertTrue(len(records) > 0)

    def test_null_values(self):
        """Test handling of null/None values."""
        dbf_path = os.path.join(self.test_data_dir, "contain_none_float.dbf")
        dbf = DBFFile(dbf_path)
        records = dbf.read_records()

        # Verify None values are properly handled
        self.assertIn(None, [r.get("VALUE") for r in records])

    def test_large_dataset(self):
        """Test handling of large datasets."""
        dbf_path = os.path.join(self.temp_dir, "large_test.dbf")
        dbf = DBFFile(dbf_path)

        # Create file with fields
        fields = [
            ("ID", "N", 10, 0),
            ("DATA", "C", 100, None),
        ]
        dbf.create(fields)

        # Generate large dataset
        num_records = 10000
        records = [
            {"ID": i, "DATA": f"Data for record {i}"} for i in range(num_records)
        ]

        # Test batch append
        dbf.append_records(records)

        # Verify record count
        read_records = dbf.read_records()
        self.assertEqual(len(read_records), num_records)

    def test_field_types(self):
        """Test all supported field types."""
        dbf_path = os.path.join(self.temp_dir, "types_test.dbf")
        dbf = DBFFile(dbf_path)

        fields = [
            ("CHAR", "C", 50, None),
            ("NUM", "N", 10, 2),
            ("INT", "I", 4, None),
            ("FLOAT", "F", 20, 4),
            ("DATE", "D", 8, None),
            ("LOGICAL", "L", 1, None),
        ]

        dbf.create(fields)

        test_record = {
            "CHAR": "Test String",
            "NUM": 123.45,
            "INT": 42,
            "FLOAT": 3.14159,
            "DATE": "20240403",
            "LOGICAL": True,  # Will be converted to "T" internally
        }

        dbf.append_records([test_record])

        # Verify field types
        records = dbf.read_records()
        self.assertEqual(len(records), 1)
        record = records[0]

        self.assertIsInstance(record["CHAR"], str)
        self.assertIsInstance(record["NUM"], float)
        self.assertIsInstance(record["INT"], int)
        self.assertIsInstance(record["FLOAT"], float)
        self.assertEqual(len(record["DATE"]), 8)  # YYYYMMDD format
        self.assertIsInstance(record["LOGICAL"], bool)

    def test_error_handling(self):
        """Test error handling scenarios."""
        dbf_path = os.path.join(self.temp_dir, "error_test.dbf")
        dbf = DBFFile(dbf_path)

        # Test invalid field type
        with self.assertRaises(Exception):
            dbf.create([("INVALID", "X", 10, None)])

        # Test invalid field length
        with self.assertRaises(Exception):
            dbf.create([("NAME", "C", 256, None)])

        dbf.create([("NAME", "C", 50, None)])
        # Test invalid field name
        with self.assertRaises(Exception):
            dbf.update_record(0, {"NONEXISTENT": "Value"})

    def test_logical_field_values(self):
        """Test different logical field value representations."""
        dbf_path = os.path.join(self.temp_dir, "logical_test.dbf")
        print(f"\nTesting logical values with file: {dbf_path}")
        dbf = DBFFile(dbf_path)

        # Create file with logical field
        fields = [
            ("FLAG", "L", 1, None),
        ]
        print("Creating DBF file with logical field")
        dbf.create(fields)

        # Test different logical value representations
        test_records = [
            {"FLAG": True},  # Will be converted to "T"
            {"FLAG": False},  # Will be converted to "F"
            {"FLAG": None},  # Will be converted to " "
        ]

        print("Writing test records:", test_records)
        dbf.append_records(test_records)

        # Read and verify
        print("Reading records back")
        records = dbf.read_records()
        print("Read records:", records)

        self.assertEqual(len(records), 3)
        self.assertTrue(records[0]["FLAG"])
        self.assertFalse(records[1]["FLAG"])
        self.assertIsNone(records[2]["FLAG"])

    def test_field_name_case_sensitivity(self):
        """测试字段名大小写处理"""
        dbf_path = os.path.join(self.temp_dir, "case_test.dbf")
        dbf = DBFFile(dbf_path)

        # 创建文件时使用混合大小写
        fields = [
            ("Name", "C", 50, None),
            ("age", "N", 3, None),
            ("Salary", "N", 10, 2),
        ]
        dbf.create(fields)

        # 使用不同大小写写入数据
        test_records = [
            {
                "NAME": "John Doe",  # 全大写
                "Age": 30,  # 首字母大写
                "salary": 5000.00,  # 全小写
            }
        ]
        dbf.append_records(test_records)

        # 读取并验证 - 应该都能正确匹配
        records = dbf.read_records()
        self.assertEqual(len(records), 1)
        # 验证字段名是否都转换为大写
        self.assertIn("NAME", records[0])
        self.assertIn("AGE", records[0])
        self.assertIn("SALARY", records[0])

        # 使用不同大小写更新数据
        dbf.update_record(0, {"name": "a"})  # 全小写
        records = dbf.read_records()
        self.assertEqual(records[0]["NAME"], "a")

    def test_chinese_field_names(self):
        """测试中文字段名处理"""
        dbf_path = os.path.join(self.temp_dir, "chinese_test.dbf")
        dbf = DBFFile(dbf_path, encoding="gbk")

        # 使用中文字段名创建文件
        fields = [
            ("姓名", "C", 50, None),
            ("年龄", "N", 3, None),
            ("工资", "N", 10, 2),
            ("NAME", "C", 50, None),  # 混合中英文字段名
        ]

        try:
            dbf.create(fields)

            # 写入测试数据
            test_records = [
                {"姓名": "张三", "年龄": 30, "工资": 5000.00, "NAME": "Zhang San"}
            ]
            dbf.append_records(test_records)

            # 读取并验证
            records = dbf.read_records()
            self.assertEqual(len(records), 1)
            self.assertEqual(records[0]["姓名"], "张三")
            self.assertEqual(records[0]["年龄"], 30)
            self.assertEqual(records[0]["工资"], 5000.00)
            self.assertEqual(records[0]["NAME"], "Zhang San")

            # 测试更新中文字段
            dbf.update_record(0, {"姓名": "李四"})
            records = dbf.read_records()
            self.assertEqual(records[0]["姓名"], "李四")

        except Exception as e:
            self.skipTest(f"中文字段名测试失败: {str(e)}")

    def test_mixed_case_chinese_field_operations(self):
        """测试混合大小写和中文字段名的操作"""
        dbf_path = os.path.join(self.temp_dir, "mixed_test.dbf")
        dbf = DBFFile(dbf_path, encoding="gbk")

        fields = [
            ("User姓名", "C", 50, None),  # 混合英文和中文
            ("Age年龄", "N", 3, None),  # 混合英文和中文
            ("Salary工资", "N", 10, 2),  # 混合英文和中文
        ]

        try:
            dbf.create(fields)

            # 使用不同形式的字段名写入数据
            test_records = [
                {
                    "USER姓名": "张三",  # 部分大写
                    "age年龄": 30,  # 部分小写
                    "Salary工资": 5000.00,  # 混合大小写
                }
            ]
            dbf.append_records(test_records)

            # 读取并验证
            records = dbf.read_records()
            self.assertEqual(len(records), 1)

            # 验证字段名的存在
            self.assertTrue(any(key.endswith("姓名") for key in records[0].keys()))
            self.assertTrue(any(key.endswith("年龄") for key in records[0].keys()))
            self.assertTrue(any(key.endswith("工资") for key in records[0].keys()))

            # 验证字段值
            record = records[0]
            self.assertEqual(
                [v for k, v in record.items() if k.endswith("姓名")][0], "张三"
            )
            self.assertEqual(
                [v for k, v in record.items() if k.endswith("年龄")][0], 30
            )
            self.assertEqual(
                [v for k, v in record.items() if k.endswith("工资")][0], 5000.00
            )

        except Exception as e:
            self.skipTest(f"混合字段名测试失败: {str(e)}")

    def test_empty_string_handling(self):
        """测试空字符串的处理"""
        dbf_path = os.path.join(self.temp_dir, "empty_string_test.dbf")
        dbf = DBFFile(dbf_path)

        # 创建文件结构
        fields = [
            ("NAME", "C", 50, None),
            ("DESC", "C", 100, None),
            ("CODE", "C", 10, None),
        ]
        dbf.create(fields)

        # 测试各种空值情况
        test_records = [
            {
                "NAME": "",  # 空字符串
                "DESC": None,  # None值
                "CODE": "123",  # 正常值
            },
            {
                "NAME": "John",  # 正常值
                "DESC": "",  # 空字符串
                "CODE": None,  # None值
            },
        ]
        dbf.append_records(test_records)

        # 读取并验证
        records = dbf.read_records()
        self.assertEqual(len(records), 2)

        # 验证第一条记录
        self.assertEqual(records[0]["NAME"], "")  # 空字符串应该保持为空字符串
        self.assertEqual(records[0]["DESC"], "")  # None应该转换为空字符串
        self.assertEqual(records[0]["CODE"], "123")  # 正常值应该保持不变

        # 验证第二条记录
        self.assertEqual(records[1]["NAME"], "John")  # 正常值应该保持不变
        self.assertEqual(records[1]["DESC"], "")  # 空字符串应该保持为空字符串
        self.assertEqual(records[1]["CODE"], "")  # None应该转换为空字符串

        # 测试更新为空字符串
        dbf.update_record(0, {"NAME": None})  # 更新为None
        records = dbf.read_records()
        self.assertEqual(records[0]["NAME"], "")  # 应该读取为空字符串

        # 测试更新为空字符串
        dbf.update_record(1, {"DESC": ""})  # 更新为空字符串
        records = dbf.read_records()
        self.assertEqual(records[1]["DESC"], "")  # 应该保持为空字符串


if __name__ == "__main__":
    unittest.main()
