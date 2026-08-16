-- ============================================================
-- 2026-08-15: 知识库索引优化 — 引入 jieba 中文分词
-- 1. 删除旧的组合 ngram FULLTEXT 索引 ft_knowledge_search(title, content_text)
-- 2. title 单独保留 ngram FULLTEXT（标题短，ngram 够用）
-- 3. content_text 改用 default parser（内容已由 jieba 预分词为空格分隔）
-- 存量数据由应用启动时 resegment_knowledge_content() 重新分词
-- ============================================================

ALTER TABLE knowledge_items DROP INDEX ft_knowledge_search;
ALTER TABLE knowledge_items ADD FULLTEXT INDEX ft_knowledge_title (title) WITH PARSER ngram;
ALTER TABLE knowledge_items ADD FULLTEXT INDEX ft_knowledge_content (content_text);
