# morton_slice 实验结论

## 实验设置

- 存档：test42，从 `r.0.0.mca` 提取 8×8 chunk 网格
- Section 范围：sy = -4 到 19（共 24 层），实际存在 1280 个 section（256 个空）
- 每个 section：4096 个方块的 global block state ID，存储为 u16-LE，8192 字节
- 原始总体积：1280 × 8192 = **10,485,760 字节**
- 压缩算法：zlib level=6

## 数据特征

同一 sy 层的 section 内容高度相似：Minecraft 地形是水平分层的，
石头层（sy≈-1~3）、地面层（sy≈4~5）、空气层（sy≥8）各自内部几乎相同。
相比之下，同一 chunk 的竖列（sy=-4..19）从深岩石到空气变化巨大，跨 section 相似性极低。

## 实验结果

| 策略 | 压缩后 | vs 单独压缩 | vs 无序串联 |
|------|--------|------------|------------|
| 单独压缩（每 section 各自 zlib） | 230,043 B | baseline | — |
| 无序串联（全部拼接，不排序） | 213,969 B | +7.0% | baseline |
| row_major（cx→cz→sy，列优先） | 213,720 B | +7.1% | +0.1% |
| morton_col（Morton曲线，列优先） | 213,523 B | +7.2% | +0.2% |
| sy_first（sy→cx→cz，切片优先） | 197,352 B | +14.2% | +7.8% |
| **morton_slice（sy→Morton(cx,cz)）** | **197,058 B** | **+14.3%** | **+7.9%** |

### 分组压缩对比（组内 morton_slice，组间独立压缩）

| 分组大小 | sections/组 | 压缩后 | vs 单独压缩 |
|---------|------------|--------|------------|
| 1×1 chunk | ~24 | 216,713 B | +5.8% |
| 2×2 chunks | ~96 | 214,406 B | +6.8% |
| 4×4 chunks | ~384 | 203,174 B | +11.7% |
| 8×8 chunks（整体） | ~1280 | 197,058 B | +14.3% |

## 核心结论

### 1. 切片优先（sy-first）是主导因素，水平排列是次要因素

`sy_first`（+14.2%）和 `morton_slice`（+14.3%）几乎相同，差值只有 0.1%。
说明**水平方向的 Morton 曲线对压缩几乎没有贡献**，决定性因素是把同一 sy 的 section 放在一起。

列优先（+7.1%）和无序串联（+7.0%）几乎一样，说明**同 chunk 的竖向 section 放在一起对 zlib 没有帮助**。

### 2. 收益随分组规模单调递增，无甜点

分组越大，收益越高，不存在"最优分组大小"。
2×2 分组（+6.8%）比 1×1（+5.8%）只多 1%，远低于整体串联的 14.3%。
原因：zlib 字典窗口 32KB ≈ 4 个 section，2×2 分组在一个 sy 层只有 4 个 section（32KB），
字典填满后就开始压下一层，几乎没有跨 sy 层的字典积累。4×4 起才有明显收益。

**推论：如果要利用切片优先的收益，必须在区域（region）级别而不是 chunk 级别压缩。**

### 3. 现有格式的问题

当前 `ChunkRegionCrafter` 存储结构：
```
region/r.0.0.mca/sections/c.{cx}.{cz}.dump   ← 每个 chunk 一个文件，包含该 chunk 所有 24 个 sy 的 section
```

每个 `.dump` 文件内部是按 SectionsDump 结构序列化的，sections 按 NBT 原始顺序排列（通常是 y 升序）。
每个文件独立成为一个 git object，`git repack` 会用路径相似度启发式在 chunk 之间做 delta 压缩，
但同 sy 的跨 chunk delta 是否被命中取决于 git 的打包顺序，没有保证。

## 重构方向

目标：让 git 对象的物理排列与 morton_slice 顺序对齐。

### 方案 A：按 sy 拆分文件（推荐）

把每个 chunk 的 `.dump` 拆分为 24 个独立 section 文件，路径加入 sy 维度：

```
当前：sections/c.{cx}.{cz}.dump
方案：sections/sy.{sy}/c.{cx}.{cz}.bin
```

`git repack` 的对象排序基于路径名，`sy.0/c.0.0.bin` 和 `sy.0/c.1.0.bin` 会被排在一起，
天然命中切片优先顺序，delta 压缩效率大幅提升。
代价：文件数量从 N_chunks 增加到 N_chunks × 24，git 目录会更密。

### 方案 B：按 sy 合并文件

每个 sy 层的所有 chunk section 合并为一个文件：

```
当前：sections/c.{cx}.{cz}.dump（每 chunk 一文件）
方案：sections/sy.{sy}.dump（每 sy 层一文件，包含该 region 所有 chunk 的该层 section）
```

层内 chunk 按 Morton 曲线排列。这让 git 把整个 sy 层作为一个对象，跨 commit 的 delta
直接在层级别发生。代价：unflatten 时需要按 sy 重组回 per-chunk 数据，逻辑更复杂；
单个 sy 文件随任意 chunk 修改而变化，导致 git diff 粒度变粗。

### 方案 C：仅调整序列化顺序（最小改动）

保持文件结构不变，只修改 `SectionsDump` 内部 sections 的序列化顺序，
同时修改 git 对象写入顺序，使 `put_par` 按 `(sy, morton(cx, cz))` 排列对象。
实际效果有限，因为 git 打包时会重新排序，单文件内的顺序不直接影响跨文件 delta。

### 推荐

方案 A，拆分为 `sections/sy.{sy}/c.{cx}.{cz}.bin`。
改动集中在 `ChunkRegionCrafter::flatten` 和 `unflatten`，
以及 `SectionsDump` 的序列化方式（从整块序列化改为按 section 拆分存取）。
