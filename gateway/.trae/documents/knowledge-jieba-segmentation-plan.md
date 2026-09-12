# 知识库索引策略优化：引入 jieba 中文分词

## Context

当前知识库的 FULLTEXT 索引 `ft_knowledge_search (title, content_text)` 使用 MySQL 默认的 ngram parser（2-gram）。ngram 对中文做机械切分，会产生大量误匹配（如搜"系统"命中"系列体操"），搜索准确率不理想。

**目标**：引入 jieba 中文分词，在应用层将 `content_text` 预切分为空格分隔的词，FULLTEXT 索引改用 default parser 做词级匹配，显著提升中文搜索准确率。

**为什么选 jieba-rs 而非 MySQL MeCab**：远程 MySQL（`120.26.105.115:3306`）不一定支持 MeCab 插件 + 中文词典，jieba-rs 是纯 Rust 实现，不依赖服务端插件，自包含可控。

## 方案设计

### 核心思路

| 字段 | 分词处理 | FULLTEXT parser | 说明 |
|---|---|---|---|
| `title` | 不分词（保持原样用于展示） | ngram | 标题短，ngram 够用 |
| `content_text` | jieba 分词后存储（空格分隔） | default | 词级匹配，精准度高 |

拆为两个独立 FULLTEXT 索引，搜索 SQL 也改为分别 MATCH。

### 改动清单

#### 1. `gateway/Cargo.toml` — 添加 jieba-rs 依赖

```toml
jieba-rs = "0.7"
```

#### 2. `gateway/src/knowledge_routes.rs` — 核心代码改动

**a. 添加 jieba 全局实例 + 分词函数**

使用 `std::sync::OnceLock`（Rust 1.70+ 标准库，无需额外依赖）初始化 jieba 实例：

```rust
use std::sync::OnceLock;
use jieba_rs::Jieba as JiebaRs;

static JIEBA: OnceLock<JiebaRs> = OnceLock::new();

fn jieba() -> &'static JiebaRs {
    JIEBA.get_or_init(JiebaRs::new)
}

/// 中文分词：用 jieba 切词，空格连接。
/// 英文/数字保持原样（jieba 不切英文）。
fn segment_chinese(text: &str) -> String {
    jieba().cut(text, true).join(" ")
}
```

**b. 修改 `markdown_to_text` 函数（L107-127）**

在现有 markdown 去标记逻辑之后，增加 jieba 分词步骤：

```rust
fn markdown_to_text(md: &str) -> String {
    let plain = md.lines()
        .map(|line| { /* 现有去标记逻辑不变 */ })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    // 新增：jieba 中文分词
    segment_chinese(&plain)
}
```

**c. 修改 `search_knowledge` 的 SQL（L546-561）**

将 `MATCH(title, content_text)` 拆为两个独立 MATCH：

```sql
-- 修改前
AND (MATCH(title, content_text) AGAINST(? IN BOOLEAN MODE) OR title LIKE ? OR content_text LIKE ?)
ORDER BY CASE WHEN MATCH(title, content_text) AGAINST(? IN BOOLEAN MODE) THEN 0 ELSE 1 END

-- 修改后
AND (MATCH(title) AGAINST(? IN BOOLEAN MODE) OR MATCH(content_text) AGAINST(? IN BOOLEAN MODE) OR title LIKE ? OR content_text LIKE ?)
ORDER BY CASE WHEN MATCH(title) AGAINST(? IN BOOLEAN MODE) OR MATCH(content_text) AGAINST(? IN BOOLEAN MODE) THEN 0 ELSE 1 END
```

绑定参数顺序：`q`(title MATCH), `segmented_q`(content MATCH), `%q%`(title LIKE), `%q%`(content LIKE), `q`(title ORDER), `segmented_q`(content ORDER)。

- `title` MATCH 传原始查询（ngram 自动切分）
- `content_text` MATCH 传 jieba 分词后的查询（default parser 按空格切词）
- LIKE 兜底传原始查询

**d. 添加 `resegment_knowledge_content` 启动任务函数**

```rust
/// 一次性任务：用 jieba 重新分词所有 knowledge_items.content_text。
/// 通过 system_settings 标记防重复执行。
pub async fn resegment_knowledge_content(pool: &DbPool) -> anyhow::Result<()> {
    // 1. 检查标记
    if let Some(v) = db::get_setting(pool, "knowledge_jieba_segmented").await? {
        if v == "1" { return Ok(()) }
    }
    tracing::info!("resegmenting knowledge content_text with jieba...");

    // 2. 读取所有 id + content
    let rows = sqlx::query("SELECT id, content FROM knowledge_items")
        .fetch_all(pool).await?;

    // 3. 逐条重新分词并更新
    for row in &rows {
        let id: String = row.try_get("id")?;
        let content: String = row.try_get("content")?;
        let new_text = markdown_to_text(&content);
        sqlx::query("UPDATE knowledge_items SET content_text = ? WHERE id = ?")
            .bind(&new_text).bind(&id).execute(pool).await?;
    }

    // 4. 写标记
    db::upsert_settings(pool, &[
        ("knowledge_jieba_segmented".into(), "1".into(), "system".into())
    ]).await?;
    tracing::info!("knowledge content_text resegmented ({} rows)", rows.len());
    Ok(())
}
```

**e. `list_knowledge`（L200-205）的 LIKE 搜索无需改 SQL**

`content_text LIKE '%keyword%'` 在分词后的文本上仍能命中（关键词是子串），且误匹配更少。无需改动。

#### 3. `gateway/migrations/20260815000020_knowledge_index_jieba.sql` — 新迁移

```sql
-- 知识库索引优化：引入 jieba 分词后的索引结构调整
-- 1. 删除旧的组合 ngram FULLTEXT 索引
-- 2. title 单独保留 ngram FULLTEXT（标题短，ngram 够用）
-- 3. content_text 改用 default parser（内容已由 jieba 预分词为空格分隔）

ALTER TABLE knowledge_items DROP INDEX ft_knowledge_search;
ALTER TABLE knowledge_items ADD FULLTEXT INDEX ft_knowledge_title (title) WITH PARSER ngram;
ALTER TABLE knowledge_items ADD FULLTEXT INDEX ft_knowledge_content (content_text);
```

#### 4. `gateway/src/main.rs` — 启动时触发重新分词

在 `tokio::spawn(cmdb_routes::pull_scheduler_loop(...))` 后面（L80 附近）加一行：

```rust
tokio::spawn(knowledge_routes::resegment_knowledge_content(state.db.clone()));
```

后台异步执行，不阻塞启动。执行完后通过 `system_settings` 标记防重复。

## 验证方案

1. `cargo build` 编译通过
2. 启动 gateway，观察日志：
   - 迁移 `20260815000020` 应用成功
   - `resegmenting knowledge content_text with jieba...` 日志出现
   - `knowledge content_text resegmented (12 rows)` 完成
3. 再次启动 gateway，确认不会重复执行（日志不出现 resegmenting）
4. 搜索测试（用 admin 登录后调用 API）：
   - 搜"主从延迟" → 应命中"MySQL 主从延迟告警处理"
   - 搜"并行复制" → 应命中同一条
   - 搜"OOM Killer" → 应命中"Linux 系统 OOM Killer 处理"
   - 搜"fail2ban" → 应命中"SSH 登录失败排查指南"
   - 搜"系统" → 不应命中不含"系统"一词的条目（验证不会 ngram 误匹配）
5. 创建一条新知识条目，确认 `content_text` 被正确分词（通过搜索验证）
