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
            {"ID": i, "DATA": f"Data for record {i}"}
            for i in range(num_records)
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
            {"FLAG": True},    # Will be converted to "T"
            {"FLAG": False},   # Will be converted to "F"
            {"FLAG": None},    # Will be converted to " "
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

if __name__ == "__main__":
    unittest.main() 