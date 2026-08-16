-- ============================================================
-- 2026-08-15: 回填 knowledge_items.content_text
-- 修复 20260815000018 seed 时遗漏 content_text 导致全文检索失效的问题
-- 对齐 knowledge_routes.rs 中 markdown_to_text 的轻量去标记逻辑
-- ============================================================

UPDATE knowledge_items
SET content_text = REPLACE(REPLACE(content, '`', ''), '#', '')
WHERE content_text IS NULL;
