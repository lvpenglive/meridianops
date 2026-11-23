-- ============================================================
-- 2026-08-15: 知识库索引优化 — 引入 jieba 中文分词
-- 1. 删除旧的组合 FULLTEXT 索引 ft_knowledge_search(title, content_text)
-- 2. title / content_text 分列建 FULLTEXT
-- 3. 不用 WITH PARSER ngram：部分 MySQL/MariaDB（含部分麒麟环境）未装 ngram 插件，
--    会导致首次部署迁移失败；content_text 已由应用侧 jieba 预分词为空格分隔
-- 存量数据由应用启动时 resegment_knowledge_content() 重新分词
-- ============================================================

ALTER TABLE knowledge_items DROP INDEX ft_knowledge_search;
ALTER TABLE knowledge_items ADD FULLTEXT INDEX ft_knowledge_title (title);
ALTER TABLE knowledge_items ADD FULLTEXT INDEX ft_knowledge_content (content_text);
