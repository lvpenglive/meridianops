<template>
  <div class="knowledge-page">
    <!-- 顶部工具栏 -->
    <div class="toolbar">
      <div class="toolbar-left">
        <el-input
          v-model="searchQuery"
          placeholder="搜索知识库..."
          prefix-icon="Search"
          clearable
          style="width: 300px"
          @keyup.enter="handleSearch"
          @clear="handleSearch"
        />
        <el-select v-model="filterCategory" placeholder="全部分类" clearable @change="loadList" style="width: 150px">
          <el-option label="全部分类" value="all" />
          <el-option v-for="c in categories" :key="c.category" :label="categoryLabel(c.category)" :value="c.category" />
        </el-select>
      </div>
      <div class="toolbar-right">
        <el-button type="primary" @click="openCreateDialog" v-if="canCreate">
          <el-icon><Plus /></el-icon>
          新建知识
        </el-button>
      </div>
    </div>

    <div class="content-wrapper">
      <!-- 左侧：分类 + 标签 -->
      <div class="sidebar">
        <div class="sidebar-section">
          <div class="sidebar-title">分类</div>
          <div class="sidebar-list">
            <div
              class="sidebar-item"
              :class="{ active: filterCategory === 'all' }"
              @click="filterCategory = 'all'; loadList()"
            >
              <span>全部</span>
              <el-badge :value="totalAll" type="info" />
            </div>
            <div
              v-for="c in categories"
              :key="c.category"
              class="sidebar-item"
              :class="{ active: filterCategory === c.category }"
              @click="filterCategory = c.category; loadList()"
            >
              <span>{{ categoryLabel(c.category) }}</span>
              <el-badge :value="c.count" type="info" />
            </div>
          </div>
        </div>
        <div class="sidebar-section" v-if="tags.length > 0">
          <div class="sidebar-title">标签</div>
          <div class="tag-cloud">
            <el-tag
              v-for="t in tags"
              :key="t.tag"
              :type="filterTag === t.tag ? 'primary' : 'info'"
              :effect="filterTag === t.tag ? 'dark' : 'plain'"
              class="tag-item"
              @click="toggleTag(t.tag)"
            >
              {{ t.tag }} ({{ t.count }})
            </el-tag>
          </div>
        </div>
      </div>

      <!-- 右侧：知识列表 -->
      <div class="main-list">
        <div v-if="loading" class="loading-wrap">
          <el-skeleton :rows="5" animated />
        </div>
        <div v-else-if="items.length === 0" class="empty-wrap">
          <el-empty description="暂无知识条目" />
        </div>
        <div v-else>
          <el-card
            v-for="item in items"
            :key="item.id"
            class="knowledge-card"
            shadow="hover"
            @click="openDetail(item.id)"
          >
            <div class="card-header">
              <span class="card-title">{{ item.title }}</span>
              <el-tag size="small" type="info">{{ categoryLabel(item.category) }}</el-tag>
            </div>
            <div class="card-summary">{{ item.summary || '暂无摘要' }}</div>
            <div class="card-footer">
              <div class="card-tags">
                <el-tag v-for="t in item.tags" :key="t" size="small" effect="plain">{{ t }}</el-tag>
              </div>
              <div class="card-meta">
                <span><el-icon><View /></el-icon> {{ item.viewCount }}</span>
                <span><el-icon><Pointer /></el-icon> {{ item.helpfulCount }}</span>
                <span v-if="item.createdByName">{{ item.createdByName }}</span>
                <span>{{ formatDate(item.updatedAt) }}</span>
              </div>
            </div>
          </el-card>

          <div class="pagination-wrap" v-if="total > pageSize">
            <el-pagination
              v-model:current-page="currentPage"
              :page-size="pageSize"
              :total="total"
              layout="prev, pager, next"
              @current-change="loadList"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- 详情抽屉 -->
    <el-drawer v-model="detailVisible" size="60%" :title="detail?.title || '知识详情'" direction="rtl">
      <div v-if="detail" class="detail-content">
        <div class="detail-meta">
          <el-tag size="small">{{ categoryLabel(detail.category) }}</el-tag>
          <el-tag v-for="t in detail.tags" :key="t" size="small" effect="plain">{{ t }}</el-tag>
          <span class="meta-text">v{{ detail.version }} · {{ detail.createdByName || '未知' }} · {{ formatDate(detail.createdAt) }}</span>
          <span class="meta-text">浏览 {{ detail.viewCount }} · 有帮助 {{ detail.helpfulCount }}</span>
        </div>
        <el-divider />
        <div class="markdown-body" v-html="renderedContent"></div>
        <el-divider />
        <div class="detail-actions">
          <el-button @click="handleMarkHelpful" :disabled="helpfulClicked">
            <el-icon><Pointer /></el-icon>
            有帮助 ({{ detail.helpfulCount }})
          </el-button>
          <el-button @click="openVersionDialog" v-if="canCreate">
            <el-icon><Clock /></el-icon>
            版本历史
          </el-button>
          <el-button @click="openEditDialog" v-if="canEdit">
            <el-icon><Edit /></el-icon>
            编辑
          </el-button>
          <el-button type="danger" @click="handleDelete" v-if="canDelete">
            <el-icon><Delete /></el-icon>
            删除
          </el-button>
        </div>
      </div>
    </el-drawer>

    <!-- 创建/编辑对话框 -->
    <el-dialog
      v-model="formVisible"
      :title="editingId ? '编辑知识' : '新建知识'"
      width="70%"
      :close-on-click-modal="false"
    >
      <el-form :model="formData" label-width="80px">
        <el-form-item label="标题" required>
          <el-input v-model="formData.title" placeholder="请输入标题" />
        </el-form-item>
        <el-form-item label="分类">
          <el-select v-model="formData.category" placeholder="选择分类" style="width: 200px">
            <el-option v-for="c in dictCategories" :key="c.value" :label="c.label" :value="c.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="标签">
          <el-select
            v-model="formData.tags"
            multiple
            filterable
            allow-create
            default-first-option
            placeholder="输入标签后回车"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="摘要">
          <el-input v-model="formData.summary" type="textarea" :rows="2" placeholder="留空将自动生成" />
        </el-form-item>
        <el-form-item label="正文" required>
          <el-input
            v-model="formData.content"
            type="textarea"
            :rows="15"
            placeholder="支持 Markdown 格式"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="formVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSave" :loading="saving">保存</el-button>
      </template>
    </el-dialog>

    <!-- 版本历史对话框 -->
    <el-dialog v-model="versionVisible" title="版本历史" width="50%">
      <el-timeline>
        <el-timeline-item
          v-for="v in versions"
          :key="v.id"
          :timestamp="formatDate(v.createdAt)"
          placement="top"
        >
          <div class="version-item">
            <span class="version-num">v{{ v.version }}</span>
            <span class="version-title">{{ v.title }}</span>
            <span class="version-editor">{{ v.editedByName || '未知' }}</span>
          </div>
        </el-timeline-item>
      </el-timeline>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, View, Pointer, Clock, Edit, Delete } from '@element-plus/icons-vue'
import { useUserStore } from '../../stores/user'
import {
  listKnowledge,
  getKnowledge,
  createKnowledge,
  updateKnowledge,
  deleteKnowledge,
  listCategories,
  listTags,
  listVersions,
  markHelpful,
  type KnowledgeItem,
  type KnowledgeDetail,
  type KnowledgeCategory,
  type KnowledgeTag,
  type KnowledgeVersion,
} from '../../api/knowledge'
import { listDictItems, type DictItem } from '../../api/dict'

const userStore = useUserStore()

const canCreate = computed(() => userStore.hasPermission('knowledge:create'))
const canEdit = computed(() => userStore.hasPermission('knowledge:update'))
const canDelete = computed(() => userStore.hasPermission('knowledge:delete'))

// 列表状态
const loading = ref(false)
const items = ref<KnowledgeItem[]>([])
const total = ref(0)
const currentPage = ref(1)
const pageSize = 10
const searchQuery = ref('')
const filterCategory = ref('all')
const filterTag = ref('')
const categories = ref<KnowledgeCategory[]>([])
const tags = ref<KnowledgeTag[]>([])
const totalAll = ref(0)

// 详情状态
const detailVisible = ref(false)
const detail = ref<KnowledgeDetail | null>(null)
const helpfulClicked = ref(false)

// 表单状态
const formVisible = ref(false)
const editingId = ref('')
const saving = ref(false)
const formData = ref({
  title: '',
  category: 'general',
  tags: [] as string[],
  content: '',
  summary: '',
})

// 版本历史
const versionVisible = ref(false)
const versions = ref<KnowledgeVersion[]>([])

// 从字典 API 动态加载分类选项
const dictCategories = ref<DictItem[]>([])

function categoryLabel(cat: string): string {
  const found = dictCategories.value.find((c) => c.value === cat)
  return found?.label || cat
}

function formatDate(iso: string): string {
  if (!iso) return ''
  const d = new Date(iso)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

// 简单 Markdown 转 HTML
const renderedContent = computed(() => {
  if (!detail.value) return ''
  return simpleMarkdown(detail.value.content)
})

function simpleMarkdown(md: string): string {
  let html = md
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
  html = html.replace(/^### (.+)$/gm, '<h3>$1</h3>')
  html = html.replace(/^## (.+)$/gm, '<h2>$1</h2>')
  html = html.replace(/^# (.+)$/gm, '<h1>$1</h1>')
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
  html = html.replace(/`(.+?)`/g, '<code>$1</code>')
  html = html.replace(/^```[\s\S]*?```/gm, (match) => {
    const code = match.replace(/```\w*\n?/g, '').replace(/```$/g, '')
    return `<pre><code>${code}</code></pre>`
  })
  html = html.replace(/^- (.+)$/gm, '<li>$1</li>')
  html = html.replace(/^\d+\. (.+)$/gm, '<li>$1</li>')
  html = html.replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>')
  html = html.replace(/\n\n/g, '</p><p>')
  html = `<p>${html}</p>`
  html = html.replace(/<p><h/g, '<h')
  html = html.replace(/<\/h(\d)><\/p>/g, '</h$1>')
  html = html.replace(/<p><pre/g, '<pre')
  html = html.replace(/<\/pre><\/p>/g, '</pre>')
  return html
}

async function loadList() {
  loading.value = true
  try {
    const params: Record<string, unknown> = {
      page: currentPage.value,
      page_size: pageSize,
    }
    if (filterCategory.value && filterCategory.value !== 'all') {
      params.category = filterCategory.value
    }
    if (filterTag.value) {
      params.tag = filterTag.value
    }
    if (searchQuery.value.trim()) {
      params.q = searchQuery.value.trim()
    }
    const res = await listKnowledge(params)
    items.value = res.items
    total.value = res.total
  } catch {
    // 错误已由 request 拦截器处理
  } finally {
    loading.value = false
  }
}

async function loadSidebar() {
  try {
    const [cats, tagList] = await Promise.all([listCategories(), listTags()])
    categories.value = cats
    tags.value = tagList
    totalAll.value = cats.reduce((sum, c) => sum + c.count, 0)
  } catch {
    // 忽略
  }
}

function handleSearch() {
  currentPage.value = 1
  loadList()
}

function toggleTag(tag: string) {
  filterTag.value = filterTag.value === tag ? '' : tag
  currentPage.value = 1
  loadList()
}

async function openDetail(id: string) {
  detailVisible.value = true
  helpfulClicked.value = false
  try {
    detail.value = await getKnowledge(id)
  } catch {
    // 错误已处理
  }
}

function openCreateDialog() {
  editingId.value = ''
  formData.value = {
    title: '',
    category: 'general',
    tags: [],
    content: '',
    summary: '',
  }
  formVisible.value = true
}

async function openEditDialog() {
  if (!detail.value) return
  editingId.value = detail.value.id
  formData.value = {
    title: detail.value.title,
    category: detail.value.category,
    tags: [...detail.value.tags],
    content: detail.value.content,
    summary: detail.value.summary || '',
  }
  formVisible.value = true
}

async function handleSave() {
  if (!formData.value.title.trim()) {
    ElMessage.warning('请输入标题')
    return
  }
  if (!formData.value.content.trim()) {
    ElMessage.warning('请输入正文内容')
    return
  }
  saving.value = true
  try {
    const data = {
      title: formData.value.title,
      category: formData.value.category,
      tags: formData.value.tags,
      content: formData.value.content,
      summary: formData.value.summary || undefined,
    }
    if (editingId.value) {
      await updateKnowledge(editingId.value, data)
      ElMessage.success('更新成功')
    } else {
      await createKnowledge(data)
      ElMessage.success('创建成功')
    }
    formVisible.value = false
    loadList()
    loadSidebar()
  } catch {
    // 错误已处理
  } finally {
    saving.value = false
  }
}

async function handleDelete() {
  if (!detail.value) return
  await ElMessageBox.confirm('确定删除该知识条目？删除后不可恢复。', '警告', { type: 'warning' })
  try {
    await deleteKnowledge(detail.value.id)
    ElMessage.success('删除成功')
    detailVisible.value = false
    loadList()
    loadSidebar()
  } catch {
    // 错误已处理
  }
}

async function handleMarkHelpful() {
  if (!detail.value || helpfulClicked.value) return
  try {
    const res = await markHelpful(detail.value.id)
    detail.value.helpfulCount = res.helpfulCount
    helpfulClicked.value = true
    ElMessage.success('感谢反馈')
  } catch {
    // 错误已处理
  }
}

async function openVersionDialog() {
  if (!detail.value) return
  try {
    versions.value = await listVersions(detail.value.id)
    versionVisible.value = true
  } catch {
    // 错误已处理
  }
}

onMounted(() => {
  loadList()
  loadSidebar()
  // 从字典 API 加载分类选项（用于创建/编辑下拉框）
  listDictItems('knowledge_category').then((items) => {
    dictCategories.value = items
  }).catch(() => {
    // 字典接口失败时静默，categoryLabel 会 fallback 到原始值
  })
})
</script>

<style scoped>
.knowledge-page {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.toolbar-left {
  display: flex;
  gap: 12px;
}

.content-wrapper {
  display: flex;
  gap: 16px;
  flex: 1;
  min-height: 0;
}

.sidebar {
  width: 200px;
  flex-shrink: 0;
}

.sidebar-section {
  background: #fff;
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 16px;
}

.sidebar-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 8px;
}

.sidebar-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.sidebar-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  color: #606266;
  transition: all 0.2s;
}

.sidebar-item:hover {
  background: #f5f7fa;
}

.sidebar-item.active {
  background: #ecf5ff;
  color: #409eff;
  font-weight: 500;
}

.tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag-item {
  cursor: pointer;
}

.main-list {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
}

.loading-wrap,
.empty-wrap {
  background: #fff;
  border-radius: 8px;
  padding: 24px;
}

.knowledge-card {
  margin-bottom: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.knowledge-card:hover {
  transform: translateY(-2px);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.card-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.card-summary {
  font-size: 13px;
  color: #909399;
  line-height: 1.6;
  margin-bottom: 12px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.card-meta {
  display: flex;
  gap: 12px;
  font-size: 12px;
  color: #909399;
  align-items: center;
}

.card-meta span {
  display: flex;
  align-items: center;
  gap: 4px;
}

.pagination-wrap {
  display: flex;
  justify-content: center;
  margin-top: 16px;
}

.detail-content {
  padding: 0 8px;
}

.detail-meta {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.meta-text {
  font-size: 12px;
  color: #909399;
}

.markdown-body {
  font-size: 14px;
  line-height: 1.8;
  color: #303133;
}

.markdown-body :deep(h1) {
  font-size: 22px;
  margin: 16px 0 8px;
}

.markdown-body :deep(h2) {
  font-size: 18px;
  margin: 14px 0 6px;
}

.markdown-body :deep(h3) {
  font-size: 16px;
  margin: 12px 0 4px;
}

.markdown-body :deep(pre) {
  background: #f5f7fa;
  border-radius: 6px;
  padding: 12px;
  overflow-x: auto;
}

.markdown-body :deep(code) {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
}

.markdown-body :deep(li) {
  margin: 4px 0;
}

.markdown-body :deep(strong) {
  color: #303133;
}

.detail-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}

.version-item {
  display: flex;
  gap: 8px;
  align-items: center;
}

.version-num {
  font-weight: 600;
  color: #409eff;
}

.version-title {
  flex: 1;
}

.version-editor {
  font-size: 12px;
  color: #909399;
}
</style>
