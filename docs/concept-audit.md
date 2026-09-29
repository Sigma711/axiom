# 逐概念验收台账

[概念清单](concept-audit.csv)以 `644edb5f` 发布版本的 `/api/practice` 目录为基线，共 414 行，每个 ID 恰好一行。`dependency_group` 标出共享数据依赖的推进批次；`shared_market_result` 表示现有行情或业绩结果的通用能力，仍须逐项核查。`catalog_source_policy` 是目录声明的数据要求，**不是已通过真实来源验收的状态**。

后三列留空表示本轮尚未逐项重新审阅，不能据此判定一个概念实现或未实现。完成每项时填入来源 URL、版本或文件指纹及核查日期，公式列记录独立算例，浏览器列记录对应的实际操作与主题、窄屏证据。遇到无法取得所需字段的市场或时段，应记下明确的不可用条件，不能用教学输入代替。

第一批优先把 `book_adjustment`、`book_share_counts`、`book_float_market_cap`、`book_free_float` 和 `book_pitfall_adjustment` 放在同一公司行动及股本历史数据链中。A 股原始日线、美国供应商日线、调整收盘价、除权参考价和账户现金/股数是不同对象；没有可追溯记录时不合并。后一批的完成证据仍按 [下一阶段计划](next-stage.md) 执行。
