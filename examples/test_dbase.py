# -*- coding: utf-8 -*-

from dbase import DBFFile
import os
import time
import random
from datetime import datetime, timedelta
import dbf
from dbfread import DBF

def generate_test_records_dict(count):
    """生成字典格式的测试记录"""
    records = []
    start_date = datetime(1980, 1, 1)
    
    cn_titles = ["工程师", "经理", "主管", "总监", "专员"]
    cn_depts = ["研发部", "市场部", "销售部", "人事部", "财务部"]
    cn_comments = ["优秀", "良好", "一般", "待改进", "需努力"]

    for i in range(count):
        random_days = random.randint(0, 15000)
        birth_date = start_date + timedelta(days=random_days)
        birth_str = birth_date.strftime("%Y%m%d")

        record = {
            "NAME": f"Person_{i:05d}",
            "EMAIL": f"person_{i:05d}@example.com",
            "DEPT": f"Dept_{random.randint(1, 10):02d}",
            "TITLE": f"Title_{random.randint(1, 5)}",
            "AGE": random.randint(18, 70),
            "SALARY": round(random.uniform(3000.00, 50000.00), 2),
            "BONUS": round(random.uniform(1000.00, 10000.00), 2),
            "BIRTH": birth_str,
            "ACTIVE": "T" if random.choice([True, False]) else "F",
            "RATING": random.randint(1, 5),
            "CN_TITLE": random.choice(cn_titles),
            "CN_DEPT": random.choice(cn_depts),
            "CN_COMMENT": random.choice(cn_comments),
            "CN_NAME": f"张{random.randint(100, 999)}",
        }
        records.append(record)
    return records

def generate_test_records(count):
    """生成测试记录 - 包含中文的测试数据"""
    records = []
    start_date = datetime(1980, 1, 1)
    
    # 中文职位列表
    cn_titles = ["工程师", "经理", "主管", "总监", "专员"]
    # 中文部门列表
    cn_depts = ["研发部", "市场部", "销售部", "人事部", "财务部"]
    # 中文备注列表
    cn_comments = ["优秀", "良好", "一般", "待改进", "需努力"]

    for i in range(count):
        random_days = random.randint(0, 15000)
        birth_date = start_date + timedelta(days=random_days)
        birth_str = birth_date.strftime("%Y%m%d")

        record = (
            f"Person_{i:05d}",                    # NAME
            f"person_{i:05d}@example.com",        # EMAIL
            f"Dept_{random.randint(1, 10):02d}",  # DEPT
            f"Title_{random.randint(1, 5)}",      # TITLE
            random.randint(18, 70),               # AGE
            round(random.uniform(3000.00, 50000.00), 2),  # SALARY
            round(random.uniform(1000.00, 10000.00), 2),  # BONUS
            birth_str,                            # BIRTH
            "T" if random.choice([True, False]) else "F",  # ACTIVE
            random.randint(1, 5),                 # RATING
            random.choice(cn_titles),             # CN_TITLE
            random.choice(cn_depts),              # CN_DEPT
            random.choice(cn_comments),           # CN_COMMENT
            f"张{random.randint(100, 999)}",      # CN_NAME
        )
        records.append(record)
    return records


def test_dbf_rust():
    """使用 Rust 实现的 DBF 文件操作测试"""
    test_file = "test_rust.dbf"

    if os.path.exists(test_file):
        os.remove(test_file)

    dbf_rust = DBFFile(test_file, encoding="utf-8")

    # 定义10个字段
    fields = [
        ("NAME", "C", 50, None),  # 姓名
        ("EMAIL", "C", 50, None),  # 邮箱
        ("DEPT", "C", 20, None),  # 部门
        ("TITLE", "C", 20, None),  # 职位
        ("AGE", "N", 3, 0),  # 年龄
        ("SALARY", "N", 10, 2),  # 工资
        ("BONUS", "N", 8, 2),  # 奖金
        ("BIRTH", "D", 8, None),  # 生日
        ("ACTIVE", "C", 1, None),  # 在职状态
        ("RATING", "N", 1, 0),  # 评级
        # 添加中文字段定义
        ("CN_TITLE", "C", 20, None),  # 中文职位
        ("CN_DEPT", "C", 20, None),   # 中文部门
        ("CN_COMMENT", "C", 20, None),# 中文备注
        ("CN_NAME", "C", 20, None),   # 中文姓名
    ]

    print("\n=== Rust 实现测试 ===")
    print("1. 测试创建 DBF 文件结构...")
    create_start = time.time()
    try:
        dbf_rust.create(fields)
        create_time = time.time() - create_start
        initial_size = os.path.getsize(test_file)
        print(f"文件结构创建成功，耗时: {create_time:.3f}秒")
        print(f"初始文件大小: {initial_size} 字节")
    except Exception as e:
        print(f"创建文件结构失败: {e}")
        return

    # 生成并写入10000行记录
    record_count = 10000
    print(f"\n2. 测试写入{record_count}行记录...")
    records = generate_test_records(record_count)

    append_start = time.time()
    try:
        dbf_rust.append_records(records)
        append_time = time.time() - append_start
        current_size = os.path.getsize(test_file)
        print(f"写入完成:")
        print(f"- 耗时: {append_time:.3f}秒")
        print(f"- 速度: {record_count / append_time:.1f}条/秒")
        print(f"- 当前文件大小: {current_size / 1024 / 1024:.2f}MB")
    except Exception as e:
        print(f"写入记录失败: {e}")
        return

    print(f"\n3. 测试读取{record_count}条记录...")
    read_start = time.time()
    try:
        all_records = dbf_rust.read_records()
        read_time = time.time() - read_start
        print(f"读取完成:")
        print(f"- 总记录数: {len(all_records)}")
        print(f"- 耗时: {read_time:.3f}秒")
        print(f"- 速度: {len(all_records) / read_time:.1f}条/秒")
        # 打印前3条记录作为样本
        print("\n数据样本(前3条):")
        for i, record in enumerate(all_records[:3], 1):
            print(f"记录 {i}:", record)
    except Exception as e:
        print(f"读取记录失败: {e}")
        return

    # 添加更新测试
    print("\n5. 测试更新记录...")
    
    # 测试单条记录更新
    print("5.1 测试单条记录更新")
    update_start = time.time()
    try:
        # 更新第一条记录
        dbf_rust.update_record(0, {"SALARY":99999.99,"CN_TITLE":"经理"})
        single_update_time = time.time() - update_start
        print(f"单条更新完成，耗时: {single_update_time:.3f}秒")
        
        # 验证更新
        updated_record = dbf_rust.read_records()[0]
        print("更新后的记录:", updated_record)
    except Exception as e:
        print(f"单条记录更新失败: {e}")
        return

    # 性能统计
    final_size = os.path.getsize(test_file)
    print("\n4. 性能统计:")
    print(f"- 字段数量: {len(fields)}")
    print(f"- 记录数量: {record_count}")
    print(f"- 最终文件大小: {final_size / 1024 / 1024:.2f}MB")
    print(f"- 平均记录大小: {final_size / record_count:.1f}字节")
    print(f"- 创建耗时: {create_time:.3f}秒")
    print(f"- 写入耗时: {append_time:.3f}秒")
    print(f"- 写入速度: {record_count / append_time:.1f}条/秒")
    print(f"- 读取耗时: {read_time:.3f}秒")
    print(f"- 读取速度: {record_count / read_time:.1f}条/秒")


def test_dbf_python():
    """使用 Python dbf 库实现的 DBF 文件操作测试"""
    test_file = "test_python.dbf"

    if os.path.exists(test_file):
        os.remove(test_file)

    # 定义字段规范
    table_def = ("NAME C(50); EMAIL C(50); DEPT C(20); TITLE C(20); "
                 "AGE N(3,0); SALARY N(10,2); BONUS N(8,2); BIRTH D; "
                 "ACTIVE C(1); RATING N(1,0); "
                 "CN_TITLE C(20); CN_DEPT C(20); CN_COMMENT C(20); CN_NAME C(20)")

    print("\n=== Python dbf 实现测试 ===")
    print("1. 测试创建 DBF 文件结构...")
    create_start = time.time()
    try:
        # 创建表结构
        table = dbf.Table(test_file, table_def,codepage="cp936")
        table.open(mode=dbf.READ_WRITE)
        create_time = time.time() - create_start
        initial_size = os.path.getsize(test_file)
        print(f"文件结构创建成功，耗时: {create_time:.3f}秒")
        print(f"初始文件大小: {initial_size} 字节")
    except Exception as e:
        print(f"创建文件结构失败: {e}")
        return

    # 生成并写入10000行记录
    record_count = 10000
    print(f"\n2. 测试写入{record_count}行记录...")
    records = generate_test_records_dict(record_count)

    append_start = time.time()
    try:
        for record in records:
            # 转换日期格式
            birth_str = record["BIRTH"]
            birth_date = datetime.strptime(birth_str, "%Y%m%d").date()
            record["BIRTH"] = birth_date  # 更新日期格式
            table.append(record)

        table.close()
        append_time = time.time() - append_start
        current_size = os.path.getsize(test_file)
        print(f"写入完成:")
        print(f"- 耗时: {append_time:.3f}秒")
        print(f"- 速度: {record_count / append_time:.1f}条/秒")
        print(f"- 当前文件大小: {current_size / 1024 / 1024:.2f}MB")
    except Exception as e:
        print(f"写入记录失败: {e}")
        table.close()
        return

    print(f"\n3. 测试读取{record_count}条记录...")
    read_start = time.time()
    try:
        # 使用 dbfread 读取文件
        table_read = DBF(test_file)

        all_records = []
        for record in table_read:
            # dbfread 已经将记录转换为字典格式，直接添加即可
            all_records.append(record)

        read_time = time.time() - read_start
        print(f"读取完成:")
        print(f"- 总记录数: {len(all_records)}")
        print(f"- 耗时: {read_time:.3f}秒")
        print(f"- 速度: {len(all_records) / read_time:.1f}条/秒")
        # 打印前3条记录作为样本
        print("\n数据样本(前3条):")
        for i, record in enumerate(all_records[:3], 1):
            print(f"记录 {i}:", record)
    except Exception as e:
        print(f"读取记录失败: {e}")
        return

    # 添加更新测试
    print("\n5. 测试更新记录...")
    
    # 测试单条记录更新
    print("5.1 测试单条记录更新")
    update_start = time.time()
    try:
        with dbf.Table(test_file, codepage="cp936")  as t:
            with t[0] as record:
                record["SALARY"] = 99999.99
        single_update_time = time.time() - update_start
        print(f"单条更新完成，耗时: {single_update_time:.3f}秒")

    except Exception as e:
        print(f"单条记录更新失败: {e}")
        table.close()
        return
    
    try:
        # 使用 dbfread 读取文件
        table_read = DBF(test_file)
        all_records = []
        for record in table_read:
            # dbfread 已经将记录转换为字典格式，直接添加即可
            all_records.append(record)
        for i, record in enumerate(all_records[:1], 1):
            print(f"更新后记录 {i}:", record)
    except Exception as e:
        print(f"读取记录失败: {e}")
        return
    
    # 性能统计
    final_size = os.path.getsize(test_file)
    print("\n4. 性能统计:")
    print(
        f"- 字段数量: {len(table.field_names)}"
    )  # 仍然使用之前的 table 对象获取字段数量
    print(f"- 记录数量: {record_count}")
    print(f"- 最终文件大小: {final_size / 1024 / 1024:.2f}MB")
    print(f"- 平均记录大小: {final_size / record_count:.1f}字节")
    print(f"- 创建耗时: {create_time:.3f}秒")
    print(f"- 写入耗时: {append_time:.3f}秒")
    print(f"- 写入速度: {record_count / append_time:.1f}条/秒")
    print(f"- 读取耗时: {read_time:.3f}秒")
    print(f"- 读取速度: {record_count / read_time:.1f}条/秒")


if __name__ == "__main__":
    test_dbf_rust()
    test_dbf_python()
