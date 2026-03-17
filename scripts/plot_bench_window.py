# /// script
# dependencies = [
#   "matplotlib",
#   "numpy",
# ]
# ///

import csv
from collections import defaultdict

import matplotlib.cm as cm  # pyright: ignore[reportMissingImports]
import matplotlib.pyplot as plt  # pyright: ignore[reportMissingImports]
import numpy as np  # pyright: ignore[reportMissingImports]
from matplotlib.ticker import MultipleLocator  # pyright: ignore[reportMissingImports]

data = defaultdict(lambda: {"round": [], "size": [], "time": []})
with open("docs/bench/bench-results.csv") as f:
    for row in csv.DictReader(f):
        w = int(row["window"])
        data[w]["round"].append(int(row["round"]) + 1)
        data[w]["size"].append(float(row["size_pack_mib"]))
        data[w]["time"].append(float(row["time_cost_s"]))

windows = sorted(data.keys())  # 0,1,2,4,8,16,32,64,128,256
n = len(windows)
colors = cm.Blues(np.linspace(0.25, 0.95, n))

fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 8), sharex=True)

for i, w in enumerate(windows):
    d = data[w]
    label = "gc --aggressive" if w == 0 else f"window={w}"
    color = "red" if w == 0 else colors[i]
    ax1.plot(d["round"], d["size"], color=color, label=label, marker="o", markersize=3)
    ax2.plot(d["round"], d["time"], color=color, label=label, marker="o", markersize=3)

ax1.set_ylabel("size-pack (MiB)")
ax1.set_title("git repack --depth 4095 --window N  (test42, no terrain)")
ax1.legend(loc="upper left", fontsize=8)
ax1.grid(True, alpha=0.3)

ax2.set_ylabel("time cost (s)")
ax2.set_xlabel("round")
ax2.legend(loc="upper left", fontsize=8)
ax2.grid(True, alpha=0.3)

ax2.xaxis.set_major_locator(MultipleLocator(1))

plt.tight_layout()
plt.savefig("docs/bench/bench-results.png", dpi=150)
print("Saved bench-results.png")
