# bmsedit

供大模型（LLM）调用的谱面编辑 CLI。无状态子命令设计：每条命令
独立进程，读文件 → 操作 → 写文件 → 退出。

## 面向 LLM 的使用要点

- **人话输出为默认**，`--json` 输出结构化结果（`{"ok":…,"data":…}` 包络）。
  建议调用方固定携带 `--json`，解析零歧义。
- **错误走 stderr**，格式 `error[<code>]: <message>`；JSON 模式为
  `{"ok":false,"error":{"code":…,"message":…}}`。
- **退出码**：`0` 成功；`1` 用法错误（调整参数可重试）；`2` 硬错误
  （IO / 解析 / 验证失败）。
- **写回安全**：行级编辑后自动重新解析并做后验检查，失败即报错且
  不落盘；写盘经临时文件 + rename 原子完成。误编辑的回退由调用方
  自管（编辑前自行备份或走版本控制）。
- **带 `#RANDOM`/`#SWITCH` 的 BMS**：读取时按 `--seed` 选定分支展开
  编辑并输出警告；写回后分支结构退化为单分支文件。
- **lane 寻址**：统一逻辑 lane——整数 = 按键（1 起从左到右），
  `sc` = 转盘；键数从谱面自动推断。
- **位置**：`MEASURE:POS`；`POS` 支持分数（`1/4`）与小数（`0.25`）。

## 子命令一览

| 命令 | 说明 |
|------|------|
| `info <FILE>` | 谱面信息与统计 |
| `header <FILE> get [KEY]` / `set KEY=VALUE...` | 头部元数据 |
| `notes <FILE> list/add/remove/move/copy` | 音符编辑 |
| `timing <FILE> list/bpm/length/stop-add/stop-remove/scroll-add/scroll-remove` | 时序编辑 |
| `defs <FILE> list/rename/dedup/compact` | WAV/BMP 定义表（仅 BMS） |
| `measure <FILE> insert/remove/clone-content` | 小节结构 |
| `transform <FILE> mirror/sparsify` | 批量变换 |
| `validate <FILE>` | 结构不变量校验 |
| `lint <FILE>` | 常见问题检查（未引用定义、LN 未配对等） |

支持格式：BMS 文本（`.bms`/`.bme`/`.bml`/`.pms`，自动检测编码）
与 bmson（`.bmson`/JSON，v0/v1 编辑后写回升为 v2）。

## 示例

```console
$ bmsedit --json notes test.bms add --measure 8 --lane sc --pos 0 --def 01
{"ok":true,"warnings":[],"data":{"measure":8,"lane":"sc","pos":"0"}}
$ bmsedit notes test.bms list --from-measure 8
#008 0 lane=sc 01 [visible]
$ bmsedit defs test.bms rename wav 3F 07
renamed wav 3F -> 07
```
