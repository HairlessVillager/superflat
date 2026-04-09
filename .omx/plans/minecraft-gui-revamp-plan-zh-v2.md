# superflat GUI 改造修订版可执行计划（Critic ITERATE 对齐）

## Principles（4）
1. **先锁运行真相**：先做 Phase0 基线验收，再进入任何功能改造。
2. **合规优先于美观**：官方品牌/资产默认不打包，资源策略先过 CI 门禁。
3. **分层发布，兼容优先**：i18n 与适配层按 Tier 渐进，不破坏现有 CLI/核心库行为。
4. **证据驱动交付**：每阶段必须有命令、产物、阈值、失败分流与负责人。

## Decision Drivers（Top 3）
1. **可发布安全性**：避免资源侵权、构建环境不可复现、接口破坏。
2. **交付确定性**：主线 + 并行支线 + Gate，减少返工和串行阻塞。
3. **兼容与维护成本**：最小改动现有 core（`superflat/src`），把 GUI 风格改造收敛在前后端壳层。

## Viable Options（>=2）
### Option A：一次性全量改造（不推荐）
- 一次提交资源/i18n/API/界面全部改完。
- 优点：总周期看起来短。
- 风险：回归面巨大、定位困难、CI 失败成本高。

### Option B：分层渐进 + Gate（**推荐**）
- 先 Phase0 基线锁定，再按资源合规/i18n/适配层分阶段推进。
- 优点：可回滚、可并行、每阶段可独立验收。
- 风险：需要更严格的流程纪律。

### Option C：仅视觉皮肤最小改动（备选）
- 只改样式，不动 i18n 与接口层。
- 优点：上线快。
- 风险：长期债务高，后续仍需二次改造。

**推荐：Option B**（满足 Critic 六项补齐且可直接执行）。

---

## ADR
- **Decision**：采用 Option B（分层渐进 + Gate），以 Phase0/1/2/3 分段推进。
- **Drivers**：合规、可验证、兼容性。
- **Alternatives considered**：
  - A 全量一次性改造（回归风险过高）
  - C 仅视觉改造（无法解决 i18n/适配层刚需）
- **Why chosen**：唯一能同时覆盖 Critic 必补项并降低失败半径。
- **Consequences**：
  - 正向：每阶段可验收、可回滚、并行效率更高。
  - 代价：需要新增门禁脚本与 CI 规则。
- **Follow-ups**：
  1) 固化脚本模板与 CI job；
  2) 把资产与语言清单纳入版本审计；
  3) 发布后补一次 Tier-2 -> 全量 i18n 回归。

---

## 执行时序（主线 + 并行支线 + Gate）
```text
主线 M0: Phase0 基线锁定
  └─Gate G0（基线通过）
      ├─并行 B1: 资源合规矩阵 + CI 阻断
      ├─并行 B2: i18n Tier-1/2 框架与回退链
      └─并行 B3: 适配层 API 改造（兼容边界内）
          └─Gate G1（B1/B2/B3 全绿）
              └─主线 M1: GUI 风格迭代 + 文案覆盖
                  └─Gate G2（功能/合规/构建通过）
                      └─主线 M2: 发布候选（RC）与验收归档
```

---

## Phase0 验收卡（必须先过）
| 项 | 指标 | 阈值 | 命令（PowerShell） | 产物路径 | 负责人 | 失败动作 |
|---|---|---|---|---|---|---|
| 环境可用性 | cargo/rustc 可执行 | 100% | `cargo -V; rustc -V` | `.omx/reports/phase0/env-check.txt` | Build Owner | 进入 preflight 分流（见下） |
| E盘工具链 | 使用 `E:\envcofig\\.cargo\\bin` | 命令解析成功 | `$env:Path='E:\envcofig\\.cargo\\bin;'+$env:Path; cargo -V` | `.omx/reports/phase0/e-drive-cargo.txt` | Build Owner | 自动切到备用路径/终止并报障 |
| Workspace 构建 | 全 workspace build 通过 | exit code 0 | `cargo build --workspace` | `.omx/reports/phase0/build.log` | Rust Owner | 阻断后续 phase |
| 核心测试 | 核心库单测通过 | exit code 0 | `cargo test -p superflat` | `.omx/reports/phase0/test-superflat.log` | Core Owner | 开缺陷并回到修复分支 |
| GUI 后端可编译 | tauri backend 编译通过 | exit code 0 | `cargo check -p superflat-gui-backend` | `.omx/reports/phase0/check-gui-backend.log` | GUI Owner | 禁止进入 B1/B2/B3 |
| 基线快照 | 关键文件 hash 记录 | 100% | `Get-FileHash Cargo.toml,README.md,superflat/src/lib.rs` | `.omx/reports/phase0/baseline-hash.txt` | Release Owner | 重新采样并复核 |

### Phase0 失败分流（E盘提权 preflight 脚本化）
**脚本目标**：检测 E 盘权限、路径、工具链、可写性；失败自动分流。

1) 新增脚本：`scripts/preflight-e-drive.ps1`（执行前必须 `Set-ExecutionPolicy -Scope Process Bypass`）
2) 检查项：
- `Test-Path E:\envcofig\\.cargo\\bin\\cargo.exe`
- `whoami /groups`（是否具备所需权限组）
- `New-Item E:\envcofig\\_perm_probe.tmp`（写权限探测）
3) 分流策略：
- **A类（路径缺失）**：回退到默认 `%USERPROFILE%\\.cargo\\bin`，记录 warning；
- **B类（权限不足）**：提示“管理员 PowerShell 重试”，并输出最小复现命令；
- **C类（cargo 不可用）**：阻断并生成故障单模板 `.omx/reports/phase0/preflight-fail.md`。

---

## 资源合规矩阵（含 CI 阻断）
| 资源类型 | 允许 | 禁止 | 仅开发映射 | 规则来源 |
|---|---|---|---|---|
| 自研纹理/图标 | ✅ | - | - | 项目自产 |
| 官方 Minecraft 品牌 Logo/字体/材质原包 | - | ❌ 默认打包/分发 | 可本地开发预览（不进发行包） | 官方使用边界 |
| 社区可商用授权资源 | ✅（需 LICENSE） | ❌ 无许可证 | 可临时验证（需清单） | 许可证条款 |
| 官方 `minecraft/lang/*.json` | ✅ 仅作对照/提取键 | ❌ 原样打包到发行物 | 开发阶段可缓存映射 | 官方 manifest 26.1.2，assetIndex 136 语言文件 |

### CI 阻断策略
- 新增 `ci-resource-compliance` job：
  1. 扫描 `assets/`、`public/`、打包清单；
  2. 命中黑名单关键字（`minecraft/font`, `assets/minecraft/textures`, 官方 logo 文件名）直接 fail；
  3. 无 LICENSE 的第三方资产 fail；
  4. 产出 `compliance-report.json` 到 `.omx/reports/compliance/`。

---

## i18n 分层落地
### Tier-1（必须）
- 范围：主导航、设置、关键操作按钮、错误提示。
- 回退：`当前语言 -> en_us -> key`。
- 更新频率：每个 PR 必须同步 Tier-1 文案键。

### Tier-2（发布前完成）
- 范围：次级页面、帮助文案、提示文本。
- 回退：同 Tier-1，新增“缺失键日志计数”。
- 更新频率：每周一次批量补齐 + RC 前冻结。

### 全量（后续迭代）
- 范围：全部 UI 文案 + 长文本场景。
- 回退：同上，增加长度/溢出检测报告。
- 更新频率：按版本里程碑。

### 目标文件（首批）
- `superflat-gui-frontend/app.rs`
- `superflat-gui-frontend/main.rs`
- `superflat-gui-frontend/styles.css`（与 key 关联的 class 标识）

---

## 适配层接口清单（新增/改动 API）
> 目标：把 GUI 改造与 core 能力解耦，尽量不改 `superflat/src/*` 语义。

### 新增 API（建议）
1. `get_ui_theme_manifest() -> ThemeManifest`
   - 影响文件：`superflat-gui-backend/src/lib.rs`、`superflat-gui-frontend/app.rs`
   - 兼容边界：仅新增读取，不影响既有命令行为。
2. `get_i18n_bundle(locale: String) -> I18nBundle`
   - 影响文件：`superflat-gui-backend/src/lib.rs`、frontend 文案加载处
   - 兼容边界：缺失 locale 必须回退 `en_us`。

### 改动 API（建议）
3. 现有 GUI 命令返回结构补 `warnings: Vec<String>`（非破坏字段）
   - 影响文件：`superflat-gui-backend/src/lib.rs`、frontend 调用解析处
   - 兼容边界：旧字段保持不变；新增字段可选读取。

### 不可触碰区（本计划内）
- `superflat/src/crafter/*`（世界生成核心）
- `superflat/src/odb/*`（对象存储逻辑）
- CLI 行为契约：`superflat-cli/main.rs` 对外参数语义不变

---

## 分阶段计划（含验收标准与验证步骤）

### Phase 0：基线锁定与 preflight（0.5 天）
- 任务：执行 Phase0 验收卡；落地 E 盘 preflight 脚本。
- 验收标准：表内 6 项全部达标；失败分流可触发并留痕。
- 验证步骤：运行 preflight -> build/test/check -> 产物文件存在且可追溯。

### Phase 1：资源合规与 CI 门禁（1 天）
- 任务：建立合规矩阵、黑白名单、CI 阻断 job。
- 验收标准：模拟违规资产可被 CI 阻断；合规报告生成成功。
- 验证步骤：构造 1 个违规样本 + 1 个合规样本跑 CI 本地/云端验证。

### Phase 2：i18n 分层与回退链（1-2 天）
- 任务：完成 Tier-1 + Tier-2 框架，接入 fallback 与缺失键日志。
- 验收标准：Tier-1 覆盖率 100%，Tier-2 覆盖率 >=80%，切换不崩溃。
- 验证步骤：语言切换冒烟、缺失键注入、长文本溢出检查。

### Phase 3：适配层接口与 GUI 集成（1-2 天）
- 任务：新增/改动 API 接入，前端消费并保留兼容。
- 验收标准：旧调用不破坏；新字段/新接口在 GUI 可见并可回退。
- 验证步骤：接口契约测试 + GUI 冒烟 + `cargo test -p superflat` 回归。

### Phase 4：RC 验收与发布（0.5-1 天）
- 任务：合并主线、生成验收包、归档证据。
- 验收标准：所有 Gate 通过；报告齐全；发布清单无禁用资产。
- 验证步骤：`cargo build --workspace`、关键路径手工冒烟、合规报告复核。

---

## 可执行命令样例（Windows PowerShell）
```powershell
# 0) E盘 preflight
Set-ExecutionPolicy -Scope Process Bypass
$env:Path = "E:\envcofig\.cargo\bin;" + $env:Path
powershell -File scripts\preflight-e-drive.ps1

# 1) Phase0 基线
cargo -V | Tee-Object .omx\reports\phase0\env-check.txt
rustc -V | Tee-Object -Append .omx\reports\phase0\env-check.txt
cargo build --workspace 2>&1 | Tee-Object .omx\reports\phase0\build.log
cargo test -p superflat 2>&1 | Tee-Object .omx\reports\phase0\test-superflat.log
cargo check -p superflat-gui-backend 2>&1 | Tee-Object .omx\reports\phase0\check-gui-backend.log

# 2) 合规门禁（示例）
powershell -File scripts\check-resource-compliance.ps1 -FailOnViolation

# 3) 发布前总验收
cargo build --workspace
cargo test -p superflat
```

---

## 可用 agent-types roster + team/ralph 配置建议
### Available agent-types roster（建议）
- `executor`：实现与脚本落地
- `reviewer`：合规/接口审查
- `verifier`：构建、测试、证据归档
- `planner`：Gate 管理与风险收敛

### 人员配置建议
- **3 人最小编制**：`2 executor + 1 verifier`
- **4 人推荐编制**：`2 executor + 1 reviewer + 1 verifier`
- **ralph 跟进场景**：仅在 team 阶段后仍有跨阶段残留缺陷时，单独启动 `ralph` 做持续收敛。

### Reasoning levels（按 lane）
- Delivery（executor）：`medium`
- Compliance/API review（reviewer）：`high`
- Verification（verifier）：`high`
- Planning/Gate（planner）：`medium`

### 启动提示（omx team / $team）
```powershell
# 推荐：4 工位并行
omx team 4:executor "执行 superflat GUI 修订计划：Phase1-Phase3 并行，Gate 驱动"

# 若需要混合 CLI
$env:OMX_TEAM_WORKER_CLI_MAP = "codex,codex,codex,claude"
omx team 4:executor "同上，含合规审查与验证 lane"
```

### Team verification path（必须走完）
1. `omx team status <team-name>`：确认 `pending/in_progress/failed`。
2. 检查邮箱：`.omx/state/team/<team>/mailbox/leader-fixed.json` 有 ACK。
3. Gate 结束前循环监控：每 30s `omx team status <team-name>`。
4. 终态条件：`pending=0 && in_progress=0 && failed=0`。
5. 再执行：`omx team shutdown <team-name>`，并复核状态清理。

---

## Gate 清单（执行时必须显式打勾）
- [ ] G0：Phase0 验收卡全通过
- [ ] G1：资源合规 + i18n Tier-1/2 + 适配层接口三线全绿
- [ ] G2：构建/测试/合规报告/RC 冒烟全部通过

## Open Questions
- [ ] i18n Tier-2 覆盖率阈值是否固定为 `>=80%`，还是按页面权重（推荐按权重）
- [ ] 合规黑名单关键字是否需要法务最终确认版本
