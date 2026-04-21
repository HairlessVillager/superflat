import csv
from collections import defaultdict

OLD_COMMIT = "7bdfe3b02ab5eaf3925db67822aecce44d4074c9"
NEW_COMMIT = "4af904babce399293b32a7bfeff49deb0d022e74"

# old.csv 中每个 file_path 的对象哈希集合及总体积
old_hashes = defaultdict(set)
old_size = defaultdict(int)

with open("old.csv", newline="", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        path = row["file_path"]
        old_hashes[path].add(row["hash"])
        old_size[path] += int(row["compressed_size"])

# new.csv 中每个 file_path 里，属于新 commit 的对象（即不在 old 中的哈希）
new_added_size = defaultdict(int)
new_added_count = defaultdict(int)

with open("new.csv", newline="", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        path = row["file_path"]
        h = row["hash"]
        if h not in old_hashes[path]:
            new_added_size[path] += int(row["compressed_size"])
            new_added_count[path] += 1

# 只保留有新增的 path
results = []
for path, added in new_added_size.items():
    old = old_size.get(path, 0)
    total = old + added
    ratio = added / total if total > 0 else 1.0
    results.append(
        {
            "file_path": path,
            "old_size": old,
            "added_size": added,
            "total_size": total,
            "added_ratio": ratio,
            "added_objects": new_added_count[path],
        }
    )

# 按新增体积降序排列
results_by_size = sorted(results, key=lambda x: x["added_size"], reverse=True)
# 按新增比例降序排列
results_by_ratio = sorted(results, key=lambda x: x["added_ratio"], reverse=True)


def human(n):
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024:
            return f"{n:.1f} {unit}"
        n /= 1024
    return f"{n:.1f} TB"


print("=" * 90)
print(f"{'按新增体积排序':^90}")
print("=" * 90)
print(
    f"{'file_path':<55} {'旧体积':>9} {'新增体积':>9} {'新增比例':>8} {'新增对象数':>6}"
)
print("-" * 90)
for r in results_by_size[:50]:
    print(
        f"{r['file_path']:<55} {human(r['old_size']):>9} {human(r['added_size']):>9} {r['added_ratio']:>7.1%} {r['added_objects']:>6}"
    )

print()
print("=" * 90)
print(f"{'按新增比例排序':^90}")
print("=" * 90)
print(
    f"{'file_path':<55} {'旧体积':>9} {'新增体积':>9} {'新增比例':>8} {'新增对象数':>6}"
)
print("-" * 90)
for r in results_by_ratio[:50]:
    print(
        f"{r['file_path']:<55} {human(r['old_size']):>9} {human(r['added_size']):>9} {r['added_ratio']:>7.1%} {r['added_objects']:>6}"
    )

print()
print(
    f"共涉及 {len(results)} 个 file_path 有新增，"
    f"总新增体积: {human(sum(r['added_size'] for r in results))}"
)

# 输出完整 CSV
with open("delta_report.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.DictWriter(
        f,
        fieldnames=[
            "file_path",
            "old_size",
            "added_size",
            "total_size",
            "added_ratio",
            "added_objects",
        ],
    )
    writer.writeheader()
    writer.writerows(results_by_size)
print("完整结果已写入 delta_report.csv")
