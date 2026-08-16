<template>
  <div class="jobs-page">
    <!-- ============ Tab ============ -->
    <el-tabs v-model="activeTab" class="no-grow-tabs">
      <el-tab-pane label="作业定义" name="definitions">
        <div class="toolbar">
          <el-input
            v-model="defQuery.keyword"
            placeholder="搜索作业名称/描述"
            clearable
            style="width: 220px"
            :prefix-icon="Search"
            @keyup.enter="loadDefinitions"
            @clear="loadDefinitions"
          />
          <el-select
            v-model="defQuery.status"
            placeholder="启用状态"
            clearable
            style="width: 130px"
            @change="loadDefinitions"
            @clear="loadDefinitions"
          >
            <el-option label="已启用" value="enabled" />
            <el-option label="已禁用" value="disabled" />
          </el-select>
          <el-button :icon="Refresh" @click="loadDefinitions">刷新</el-button>
          <el-button
            v-if="canCreate"
            type="primary"
            :icon="Plus"
            @click="openCreateDialog"
          >
            新建作业
          </el-button>
        </div>

        <el-table
          v-loading="defLoading"
          :data="defList"
          border
          stripe
          class="mt-12"
        >
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="name" label="作业名称" min-width="160">
            <template #default="{ row }">
              <div class="job-name-cell">
                <el-icon><Operation /></el-icon>
                <span>{{ row.name }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="scriptType" label="脚本类型" width="110" align="center">
            <template #default="{ row }">
              <el-tag size="small" :type="scriptTagType(row.scriptType)" effect="light">
                {{ row.scriptType }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="执行器" width="100" align="center">
            <template #default="{ row }">
              <el-tag size="small" :type="(row.executorType || 'ssh') === 'ssh' ? 'primary' : 'info'" effect="plain">
                {{ (row.executorType || 'ssh') === 'ssh' ? 'SSH' : '模拟' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="目标范围" width="120" align="center">
            <template #default="{ row }">
              <span>{{ scopeLabel(row.targetScope) }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="timeoutSecs" label="超时" width="90" align="center">
            <template #default="{ row }">{{ row.timeoutSecs }}s</template>
          </el-table-column>
          <el-table-column label="启用" width="70" align="center">
            <template #default="{ row }">
              <el-tag size="small" :type="row.enabled ? 'success' : 'info'" effect="plain">
                {{ row.enabled ? '是' : '否' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="createdBy" label="创建人" width="110" />
          <el-table-column prop="updatedAt" label="更新时间" width="170" />
          <el-table-column label="操作" width="280" fixed="right">
            <template #default="{ row }">
              <el-button
                v-if="canExecute"
                link
                type="primary"
                :icon="Promotion"
                :disabled="!row.enabled"
                @click="openExecuteDialog(row)"
              >
                执行
              </el-button>
              <el-button v-if="canCreate" link :icon="Edit" @click="openEditDialog(row)">编辑</el-button>
              <el-popconfirm
                v-if="canAdmin"
                title="确定删除该作业定义？（若已有执行历史则不可删除）"
                confirm-button-text="删除"
                cancel-button-text="取消"
                @confirm="handleDelete(row)"
              >
                <template #reference>
                  <el-button link type="danger" :icon="Delete">删除</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>

        <div class="pagination-wrap">
          <el-pagination
            v-model:current-page="defQuery.page"
            v-model:page-size="defQuery.pageSize"
            :total="defTotal"
            :page-sizes="[10, 20, 50, 100]"
            layout="total, sizes, prev, pager, next, jumper"
            background
            @size-change="loadDefinitions"
            @current-change="loadDefinitions"
          />
        </div>
      </el-tab-pane>

      <el-tab-pane label="执行历史" name="runs">
        <div class="toolbar">
          <el-input
            v-model="runQuery.keyword"
            placeholder="搜索作业名称/执行人"
            clearable
            style="width: 240px"
            :prefix-icon="Search"
            @keyup.enter="loadRuns"
            @clear="loadRuns"
          />
          <el-select
            v-model="runQuery.status"
            placeholder="执行状态"
            clearable
            style="width: 140px"
            @change="loadRuns"
            @clear="loadRuns"
          >
            <el-option label="执行中" value="running" />
            <el-option label="成功" value="success" />
            <el-option label="失败" value="failed" />
            <el-option label="部分成功" value="partial" />
            <el-option label="超时" value="timeout" />
          </el-select>
          <el-button :icon="Refresh" @click="loadRuns">刷新</el-button>
        </div>

        <el-table v-loading="runLoading" :data="runList" border stripe class="mt-12">
          <el-table-column prop="id" label="Run ID" width="90" />
          <el-table-column label="作业" min-width="180">
            <template #default="{ row }">
              <div class="job-name-cell">
                <el-icon><Cpu /></el-icon>
                <span>
                  {{ row.jobName }}
                  <small style="color: #909399">#{{ row.jobId }}</small>
                </span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="triggerMode" label="触发方式" width="100" align="center">
            <template #default="{ row }">
              <el-tag size="small" type="info" effect="plain">
                {{ triggerLabel(row.triggerMode) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="结果" width="160" align="center">
            <template #default="{ row }">
              <el-space direction="horizontal" size="mini">
                <el-tag size="small" :type="statusTagType(row.overallStatus)" effect="dark">
                  {{ statusLabel(row.overallStatus) }}
                </el-tag>
                <span class="count-badge">
                  <el-icon v-if="row.successCount > 0" :size="14" color="#67c23a"><CircleCheckFilled /></el-icon>
                  <span :class="{ 'text-success': row.successCount > 0 }">{{ row.successCount }}</span>
                  <span style="color:#c0c4cc;margin:0 2px">/</span>
                  <el-icon v-if="row.failedCount > 0" :size="14" color="#f56c6c"><CircleCloseFilled /></el-icon>
                  <span :class="{ 'text-danger': row.failedCount > 0 }">{{ row.failedCount }}</span>
                </span>
              </el-space>
            </template>
          </el-table-column>
          <el-table-column prop="startedBy" label="执行人" width="110" />
          <el-table-column label="开始 → 结束" width="170">
            <template #default="{ row }">
              <div class="time-stack">
                <span>{{ formatTime(row.startedAt) }}</span>
                <span v-if="row.finishedAt" style="color:#909399">→ {{ formatTime(row.finishedAt) }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="耗时" width="110" align="center">
            <template #default="{ row }">{{ formatDuration(row.durationMs) }}</template>
          </el-table-column>
          <el-table-column label="操作" width="120" fixed="right">
            <template #default="{ row }">
              <el-button link type="primary" :icon="View" @click="openRunDetail(row)">查看</el-button>
            </template>
          </el-table-column>
        </el-table>

        <div class="pagination-wrap">
          <el-pagination
            v-model:current-page="runQuery.page"
            v-model:page-size="runQuery.pageSize"
            :total="runTotal"
            :page-sizes="[10, 20, 50, 100]"
            layout="total, sizes, prev, pager, next, jumper"
            background
            @size-change="loadRuns"
            @current-change="loadRuns"
          />
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- ============ 新建/编辑作业对话框 ============ -->
    <el-dialog
      v-model="formDialogVisible"
      :title="isEdit ? '编辑作业定义' : '新建作业定义'"
      width="1000px"
      top="6vh"
      :close-on-click-modal="false"
      destroy-on-close
      class="job-form-dialog"
    >
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="110px">
        <!-- 基本信息分组 -->
        <el-divider content-position="left">
          <span class="divider-text"><el-icon><InfoFilled /></el-icon> 基本信息</span>
        </el-divider>
        <el-row :gutter="20">
          <el-col :span="14">
            <el-form-item label="作业名称" prop="name">
              <el-input v-model="form.name" placeholder="例如：巡检 Linux 磁盘使用率" maxlength="100" show-word-limit />
            </el-form-item>
          </el-col>
          <el-col :span="10">
            <el-form-item label="脚本类型" prop="scriptType">
              <el-select v-model="form.scriptType" style="width: 100%">
                <el-option label="Shell (bash/sh)" value="shell" />
                <el-option label="Python" value="python" />
                <el-option label="PowerShell" value="powershell" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="描述" prop="description">
              <el-input v-model="form.description" placeholder="简要描述作业用途，便于后续检索" maxlength="200" show-word-limit />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="超时(秒)" prop="timeoutSecs">
              <el-input-number v-model="form.timeoutSecs" :min="10" :max="86400" style="width: 100%" />
              <span class="form-tip">超过该时间未完成将自动终止（10–86400 秒）</span>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="启用状态" prop="enabled">
              <el-switch v-model="form.enabled" active-text="启用" inactive-text="禁用" />
              <span class="form-tip">禁用后不可被执行，但可继续编辑</span>
            </el-form-item>
          </el-col>
        </el-row>

        <!-- 脚本内容分组 -->
        <el-divider content-position="left">
          <span class="divider-text"><el-icon><Document /></el-icon> 脚本内容</span>
        </el-divider>
        <el-form-item label="脚本内容" prop="scriptContent" label-width="110px">
          <div class="script-editor-wrap">
            <div class="script-editor-toolbar">
              <el-tag size="small" :type="scriptTagType(form.scriptType)" effect="light">
                {{ form.scriptType }}
              </el-tag>
              <span class="script-editor-hint">
                支持最大 500KB · 当前
                <strong>{{ (form.scriptContent.length / 1024).toFixed(1) }} KB</strong>
              </span>
            </div>
            <el-input
              v-model="form.scriptContent"
              type="textarea"
              :rows="10"
              placeholder="#!/bin/bash&#10;df -h&#10;free -m&#10;uptime"
              spellcheck="false"
              class="script-textarea"
            />
          </div>
        </el-form-item>

        <!-- 执行配置分组 -->
        <el-divider content-position="left">
          <span class="divider-text"><el-icon><Setting /></el-icon> 执行配置</span>
        </el-divider>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="执行器" prop="executorType">
              <el-radio-group v-model="form.executorType">
                <el-radio value="ssh">SSH</el-radio>
                <el-radio value="mock">模拟</el-radio>
              </el-radio-group>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="执行用户" prop="runAs">
              <el-input v-model="form.runAs" placeholder="默认 root" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="SSH 端口" prop="port">
              <el-input-number v-model="form.port" :min="1" :max="65535" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="24" v-if="form.executorType === 'ssh'">
            <el-form-item label="SSH 凭据" prop="credentialId">
              <el-select
                v-model="form.credentialId"
                placeholder="选择已创建的 SSH 凭据"
                style="width: 100%"
                filterable
              >
                <el-option
                  v-for="c in credentialOptions"
                  :key="c.id"
                  :label="`${c.name}（${c.username} · ${c.authType === 'key' ? '私钥' : '密码'}）`"
                  :value="c.id"
                />
              </el-select>
              <div class="cred-hint">
                <el-icon><InfoFilled /></el-icon>
                未找到凭据？请先到
                <router-link to="/system/credentials" class="cred-link">SSH 凭据管理</router-link>
                创建
              </div>
            </el-form-item>
          </el-col>
          <el-col :span="24" v-else>
            <el-alert
              type="info"
              :closable="false"
              show-icon
              title="模拟执行器"
              description="不会真正连接目标资产，由 MockExecutor 生成示例输出，用于业务链路验证。"
            />
          </el-col>
        </el-row>

        <!-- 目标范围分组 -->
        <el-divider content-position="left">
          <span class="divider-text"><el-icon><Aim /></el-icon> 目标范围</span>
        </el-divider>
        <el-form-item label="目标范围" prop="targetScope">
          <el-radio-group v-model="form.targetScope">
            <el-radio value="manual">执行时选择</el-radio>
            <el-radio value="static">静态资产列表</el-radio>
            <el-radio value="cmdb_query">CMDB 查询 (V1.5)</el-radio>
          </el-radio-group>
          <div class="cred-hint">
            <el-icon><InfoFilled /></el-icon>
            <template v-if="form.targetScope === 'manual'">每次执行时手动选择目标资产，灵活度最高</template>
            <template v-else-if="form.targetScope === 'static'">预先固定资产列表，适合标准化巡检任务</template>
            <template v-else>通过 CMDB 查询动态获取目标资产，V1.5 发布</template>
          </div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="formDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmitForm">
          {{ isEdit ? '保存修改' : '创建作业' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- ============ 执行作业对话框 ============ -->
    <el-dialog
      v-model="execDialogVisible"
      :title="`执行：${execJob?.name || ''}`"
      width="600px"
      :close-on-click-modal="false"
      destroy-on-close
    >
      <el-alert
        type="warning"
        :closable="false"
        show-icon
        title="V1 为 Mock 执行引擎"
        description="不会真的连接目标资产。执行结果由 MockExecutor 生成示例 stdout，用于验证业务链路。真实 SSH 接入在 V1.5 发布。"
        style="margin-bottom: 12px"
      />
      <el-descriptions :column="2" border size="small" style="margin-bottom: 12px">
        <el-descriptions-item label="脚本类型">{{ execJob?.scriptType }}</el-descriptions-item>
        <el-descriptions-item label="超时">{{ execJob?.timeoutSecs }} 秒</el-descriptions-item>
        <el-descriptions-item label="脚本内容" :span="2">
          <pre class="script-preview">{{ execJob?.scriptContent }}</pre>
        </el-descriptions-item>
      </el-descriptions>
      <el-form label-width="100px">
        <el-form-item label="目标资产" required>
          <el-select
            v-model="execAssetIds"
            multiple
            filterable
            placeholder="请选择至少 1 个目标资产（可多选）"
            style="width: 100%"
          >
            <el-option
              v-for="a in assetList"
              :key="a.id"
              :value="a.id"
              :label="`${a.assetName} (${a.primaryIp || '无IP'})`"
            />
          </el-select>
          <small style="color:#909399">共 {{ assetList.length }} 个 CMDB 资产</small>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="execDialogVisible = false">取消</el-button>
        <el-button
          type="primary"
          :loading="executing"
          :disabled="execAssetIds.length === 0"
          @click="handleExecuteConfirm"
        >
          立即执行（{{ execAssetIds.length }} 台）
        </el-button>
      </template>
    </el-dialog>

    <!-- ============ 执行历史详情抽屉 ============ -->
    <el-drawer
      v-model="runDetailVisible"
      title="执行详情"
      size="80%"
      destroy-on-close
    >
      <div v-loading="runDetailLoading" v-if="runDetail">
        <el-descriptions :column="3" border size="small">
          <el-descriptions-item label="Run ID">{{ runDetail.run.id }}</el-descriptions-item>
          <el-descriptions-item label="作业">{{ runDetail.run.jobName }} #{{ runDetail.run.jobId }}</el-descriptions-item>
          <el-descriptions-item label="执行人">{{ runDetail.run.startedBy }}</el-descriptions-item>
          <el-descriptions-item label="整体状态">
            <el-tag size="small" :type="statusTagType(runDetail.run.overallStatus)" effect="dark">
              {{ statusLabel(runDetail.run.overallStatus) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="成功率">
            <span style="color:#67c23a">{{ runDetail.run.successCount }}</span>
            /
            <span style="color:#f56c6c">{{ runDetail.run.failedCount }}</span>
            / {{ runDetail.run.targetCount }}
          </el-descriptions-item>
          <el-descriptions-item label="耗时">
            {{ runDetail.run.finishedAt ? formatDuration(runDetail.run.durationMs) : '执行中...' }}
          </el-descriptions-item>
        </el-descriptions>

        <el-divider>脚本内容</el-divider>
        <pre class="script-preview" style="max-height: 160px">{{ runDetail.run.scriptContent }}</pre>

        <el-divider>各资产执行结果</el-divider>
        <el-table :data="runDetail.targets" border size="small">
          <el-table-column label="资产" min-width="200">
            <template #default="{ row }">
              <div>
                <strong>{{ row.assetName }}</strong>
                <small style="color:#909399; margin-left: 6px">{{ row.assetIp || '无IP' }}</small>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="110" align="center">
            <template #default="{ row }">
              <el-tag size="small" :type="statusTagType(row.status)" effect="dark">
                {{ statusLabel(row.status) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="退出码" width="80" align="center" prop="exitCode">
            <template #default="{ row }">
              <span v-if="row.exitCode !== undefined && row.exitCode !== null">
                {{ row.exitCode }}
              </span>
              <span v-else style="color:#c0c4cc">-</span>
            </template>
          </el-table-column>
          <el-table-column label="耗时" width="100" align="center">
            <template #default="{ row }">{{ formatDuration(row.durationMs) }}</template>
          </el-table-column>
          <el-table-column label="执行日志" min-width="300">
            <template #default="{ row }">
              <el-tabs v-model="selectedOutputTab[row.id]" size="small">
                <el-tab-pane label="stdout" name="stdout">
                  <pre
                    class="output-preview"
                    :class="{ 'has-more': row.stdout.includes('(truncated)') }"
                    @click="openFullOutput(row, 'stdout')"
                  >{{ row.stdout || '(无输出)' }}</pre>
                </el-tab-pane>
                <el-tab-pane label="stderr" name="stderr">
                  <pre
                    class="output-preview stderr"
                    :class="{ 'has-more': row.stderr.includes('(truncated)') }"
                    @click="openFullOutput(row, 'stderr')"
                  >{{ row.stderr || '(无错误)' }}</pre>
                </el-tab-pane>
              </el-tabs>
            </template>
          </el-table-column>
        </el-table>

        <div style="margin-top: 16px; text-align: center">
          <el-button :icon="Refresh" type="primary" plain @click="refreshRunDetail">
            刷新详情
          </el-button>
        </div>
      </div>
    </el-drawer>

    <!-- 完整输出对话框 -->
    <el-dialog v-model="fullOutputVisible" title="完整执行日志" width="800px" top="5vh">
      <div v-if="fullOutputData">
        <el-descriptions :column="3" size="small" border style="margin-bottom: 12px">
          <el-descriptions-item label="资产">{{ fullOutputData.assetName }} ({{ fullOutputData.assetIp }})</el-descriptions-item>
          <el-descriptions-item label="状态">{{ statusLabel(fullOutputData.status) }}</el-descriptions-item>
          <el-descriptions-item label="退出码">{{ fullOutputData.exitCode ?? '-' }}</el-descriptions-item>
        </el-descriptions>
        <el-tabs v-model="fullOutputTab" size="default">
          <el-tab-pane label="stdout" name="stdout">
            <pre class="full-output">{{ fullOutputData.stdout || '(无输出)' }}</pre>
          </el-tab-pane>
          <el-tab-pane label="stderr" name="stderr">
            <pre class="full-output stderr">{{ fullOutputData.stderr || '(无错误)' }}</pre>
          </el-tab-pane>
        </el-tabs>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, onMounted } from 'vue'
import {
  ElMessage, ElMessageBox, type FormInstance, type FormRules,
} from 'element-plus'
import {
  Search, Refresh, Plus, Edit, Delete, Operation, Promotion,
  Cpu, CircleCheckFilled, CircleCloseFilled, View,
  InfoFilled, Document, Setting, Aim,
} from '@element-plus/icons-vue'
import {
  listJobDefinitions, getJobDefinition, createJobDefinition, updateJobDefinition,
  deleteJobDefinition, executeJob, listJobRuns, getJobRun, getJobRunTargetOutput, listJobAssets,
  type JobDefinition, type JobRun, type JobRunTarget, type ExecutorType,
} from '../../api/job'
import { listAllCredentials, type SshCredentialSimple } from '../../api/credential'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()

const canCreate = computed(() => userStore.hasPermission('job:create'))
const canExecute = computed(() => userStore.hasPermission('job:execute'))
const canAdmin = computed(() => userStore.hasPermission('job:admin'))

// ===== Tab 控制 =====
const activeTab = ref('definitions')

// ===== 作业定义 =====
const defLoading = ref(false)
const defList = ref<JobDefinition[]>([])
const defTotal = ref(0)
const defQuery = reactive({
  page: 1,
  pageSize: 20,
  keyword: '',
  status: '' as 'enabled' | 'disabled' | '',
})
async function loadDefinitions() {
  defLoading.value = true
  try {
    const res = await listJobDefinitions(defQuery)
    defList.value = res.list
    defTotal.value = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载作业列表失败')
  } finally {
    defLoading.value = false
  }
}

// ===== 执行历史 =====
const runLoading = ref(false)
const runList = ref<JobRun[]>([])
const runTotal = ref(0)
const runQuery = reactive({
  page: 1,
  pageSize: 20,
  keyword: '',
  status: '',
})
async function loadRuns() {
  runLoading.value = true
  try {
    const res = await listJobRuns(runQuery)
    runList.value = res.list
    runTotal.value = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载执行历史失败')
  } finally {
    runLoading.value = false
  }
}

// ===== 资产列表（供执行对话框多选）=====
const assetList = ref<any[]>([])
async function loadAssetList() {
  try {
    const res = await listJobAssets({ pageSize: 500 })
    assetList.value = res.list
  } catch {
    assetList.value = []
  }
}

// ===== 新建/编辑作业 =====
const formDialogVisible = ref(false)
const isEdit = ref(false)
const editingId = ref<number | null>(null)
const formRef = ref<FormInstance>()
const form = reactive({
  name: '',
  description: '',
  scriptType: 'shell' as 'shell' | 'python' | 'powershell',
  scriptContent: '',
  timeoutSecs: 300,
  targetScope: 'manual' as 'static' | 'cmdb_query' | 'manual',
  targetAssetIds: [] as number[],
  targetCmdbQuery: '',
  runAs: 'root',
  port: 22,
  enabled: true,
  executorType: 'ssh' as ExecutorType,
  credentialId: null as number | null,
})
// V1.5: 凭据下拉列表
const credentialOptions = ref<SshCredentialSimple[]>([])
async function loadCredentialOptions() {
  try {
    credentialOptions.value = await listAllCredentials()
  } catch {
    credentialOptions.value = []
  }
}
const submitting = ref(false)
const formRules: FormRules = {
  name: [
    { required: true, message: '请输入作业名称', trigger: 'blur' },
    { min: 2, max: 100, message: '长度 2-100', trigger: 'blur' },
  ],
  scriptType: [{ required: true, message: '请选择脚本类型', trigger: 'change' }],
  scriptContent: [
    { required: true, message: '脚本内容不能为空', trigger: 'blur' },
    { min: 1, max: 500000, message: '脚本上限 500KB', trigger: 'blur' },
  ],
  targetScope: [{ required: true, message: '请选择目标范围', trigger: 'change' }],
  runAs: [{ required: true, message: '请输入执行用户', trigger: 'blur' }],
  port: [{ required: true, message: '端口必填', trigger: 'blur' }],
  credentialId: [
    {
      validator: (_rule: any, value: any, callback: any) => {
        if (form.executorType === 'ssh' && (value === null || value === undefined)) {
          callback(new Error('SSH 执行器必须选择凭据'))
        } else {
          callback()
        }
      },
      trigger: 'change',
    },
  ],
}

function openCreateDialog() {
  isEdit.value = false
  editingId.value = null
  Object.assign(form, {
    name: '', description: '',
    scriptType: 'shell', scriptContent: '',
    timeoutSecs: 300, targetScope: 'manual',
    targetAssetIds: [], targetCmdbQuery: '',
    runAs: 'root', port: 22, enabled: true,
    executorType: 'ssh', credentialId: null,
  })
  loadCredentialOptions()
  formDialogVisible.value = true
}
async function openEditDialog(row: JobDefinition) {
  try {
    const def = await getJobDefinition(row.id)
    isEdit.value = true
    editingId.value = row.id
    Object.assign(form, {
      name: def.name,
      description: def.description || '',
      scriptType: def.scriptType,
      scriptContent: def.scriptContent,
      timeoutSecs: def.timeoutSecs,
      targetScope: def.targetScope,
      targetAssetIds: def.targetAssetIds || [],
      targetCmdbQuery: def.targetCmdbQuery || '',
      runAs: def.runAs,
      port: def.port,
      enabled: def.enabled,
      executorType: def.executorType || 'ssh',
      credentialId: def.credentialId ?? null,
    })
    loadCredentialOptions()
    formDialogVisible.value = true
  } catch (e: any) {
    ElMessage.error(e?.message || '加载作业详情失败')
  }
}
async function handleSubmitForm() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      const payload = { ...form }
      // 执行器为 mock 时清除凭据关联
      if (payload.executorType === 'mock') {
        payload.credentialId = null
      }
      if (isEdit.value && editingId.value) {
        await updateJobDefinition(editingId.value, payload)
        ElMessage.success('作业定义已更新')
      } else {
        await createJobDefinition(payload)
        ElMessage.success('作业定义已创建')
      }
      formDialogVisible.value = false
      loadDefinitions()
    } catch (e: any) {
      ElMessage.error(e?.message || '保存失败')
    } finally {
      submitting.value = false
    }
  })
}
async function handleDelete(row: JobDefinition) {
  try {
    await deleteJobDefinition(row.id)
    ElMessage.success('删除成功')
    loadDefinitions()
  } catch (e: any) {
    ElMessage.error(e?.message || '删除失败')
  }
}

// ===== 执行作业 =====
const execDialogVisible = ref(false)
const execJob = ref<JobDefinition | null>(null)
const execAssetIds = ref<number[]>([])
const executing = ref(false)
function openExecuteDialog(row: JobDefinition) {
  execJob.value = row
  execAssetIds.value = row.targetAssetIds && row.targetAssetIds.length > 0 ? [...row.targetAssetIds] : []
  loadAssetList()
  execDialogVisible.value = true
}
async function handleExecuteConfirm() {
  if (!execJob.value || execAssetIds.value.length === 0) return
  executing.value = true
  try {
    const res = await executeJob(execJob.value.id, execAssetIds.value)
    ElMessage.success(`已提交！Run ID: ${res.jobRunId}，共 ${res.targetCount} 台资产`)
    execDialogVisible.value = false
    // 切到执行历史 tab 并打开该 run 的详情
    activeTab.value = 'runs'
    await loadRuns()
    setTimeout(() => openRunDetailById(res.jobRunId), 300)
  } catch (e: any) {
    ElMessage.error(e?.message || '执行失败')
  } finally {
    executing.value = false
  }
}

// ===== 执行历史详情 =====
const runDetailVisible = ref(false)
const runDetailLoading = ref(false)
const runDetail = ref<{ run: JobRun; targets: JobRunTarget[] } | null>(null)
const selectedOutputTab = reactive<Record<number, string>>({})

async function openRunDetail(row: JobRun) {
  openRunDetailById(row.id)
}

async function openRunDetailById(runId: number) {
  runDetailVisible.value = true
  await refreshRunDetailById(runId)
}

async function refreshRunDetail() {
  if (runDetail.value) await refreshRunDetailById(runDetail.value.run.id)
}

async function refreshRunDetailById(runId: number) {
  runDetailLoading.value = true
  try {
    const res = await getJobRun(runId)
    runDetail.value = res
    // 默认所有 target 选中 stdout tab
    res.targets.forEach(t => {
      if (!selectedOutputTab[t.id]) selectedOutputTab[t.id] = 'stdout'
    })
  } catch (e: any) {
    ElMessage.error(e?.message || '加载执行详情失败')
  } finally {
    runDetailLoading.value = false
  }
}

// ===== 完整输出 =====
const fullOutputVisible = ref(false)
const fullOutputTab = ref('stdout')
const fullOutputData = ref<{
  assetName: string
  assetIp: string
  status: string
  exitCode?: number
  stdout: string
  stderr: string
} | null>(null)

async function openFullOutput(target: JobRunTarget, initialTab: 'stdout' | 'stderr') {
  // 如果显示的是截断的，拉完整
  let stdout = target.stdout
  let stderr = target.stderr
  if (stdout.includes('(truncated)') || stderr.includes('(truncated)')) {
    try {
      const full = await getJobRunTargetOutput(runDetail.value!.run.id, target.id)
      stdout = full.stdout
      stderr = full.stderr
    } catch (e: any) {
      ElMessage.warning(e?.message || '拉取完整日志失败')
    }
  }
  fullOutputTab.value = initialTab
  fullOutputData.value = {
    assetName: target.assetName,
    assetIp: target.assetIp,
    status: target.status,
    exitCode: target.exitCode,
    stdout,
    stderr,
  }
  fullOutputVisible.value = true
}

// ===== 标签和工具 =====
function scriptTagType(t: string) {
  if (t === 'python') return 'warning' as const
  if (t === 'powershell') return 'success' as const
  return 'info' as const
}
function scopeLabel(s: string) {
  if (s === 'static') return '静态资产'
  if (s === 'cmdb_query') return 'CMDB 查询'
  return '执行时选择'
}
function triggerLabel(m: string) {
  if (m === 'cron') return '定时'
  if (m === 'api') return 'API'
  return '手动'
}
function statusLabel(s: string) {
  const map: Record<string, string> = {
    pending: '等待中', running: '执行中', success: '成功',
    failed: '失败', partial: '部分成功', timeout: '超时',
    cancelled: '已取消', skipped: '跳过',
  }
  return map[s] || s
}
function statusTagType(s: string) {
  switch (s) {
    case 'success': return 'success' as const
    case 'running': return 'primary' as const
    case 'failed': case 'timeout': case 'cancelled': return 'danger' as const
    case 'partial': return 'warning' as const
    default: return 'info' as const
  }
}
function formatTime(s?: string): string {
  if (!s) return ''
  const d = new Date(s)
  if (isNaN(d.getTime())) return s
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}
function formatDuration(ms: number): string {
  if (ms <= 0) return '—'
  const s = Math.floor(ms / 1000)
  if (s < 60) return `${s}s ${ms % 1000}ms`
  const m = Math.floor(s / 60)
  const ss = s % 60
  if (m < 60) return `${m}m ${ss}s`
  const h = Math.floor(m / 60)
  const mm = m % 60
  return `${h}h ${mm}m ${ss}s`
}

onMounted(() => {
  loadDefinitions()
  loadRuns()
})
</script>

<style scoped>
.jobs-page { padding: 16px; }
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
.mt-12 { margin-top: 12px; }
.pagination-wrap {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.job-name-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
}

.count-badge {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  font-family: monospace;
  font-size: 12px;
}

.time-stack {
  display: flex;
  flex-direction: column;
  font-size: 12px;
  line-height: 1.4;
}

.script-preview {
  background: #2d2d2d;
  color: #f8f8f2;
  padding: 8px 12px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.5;
  overflow: auto;
  max-height: 200px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}

.output-preview {
  background: #f5f7fa;
  padding: 6px 8px;
  margin: 0;
  border-radius: 3px;
  font-family: 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.4;
  max-height: 100px;
  overflow: auto;
  cursor: pointer;
  white-space: pre-wrap;
  word-break: break-all;
  transition: background 0.2s;
}
.output-preview:hover { background: #ebeef5; }
.output-preview.stderr { color: #f56c6c; background: #fef0f0; }
.output-preview.has-more::after {
  content: '点击查看完整日志 >>';
  display: block;
  color: #409eff;
  margin-top: 4px;
  font-size: 11px;
}

.full-output {
  background: #1e1e1e;
  color: #d4d4d4;
  padding: 16px;
  border-radius: 6px;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
  max-height: 60vh;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
.full-output.stderr { color: #f48771; background: #2d1e1e; }

.text-success { color: #67c23a; }
.text-danger { color: #f56c6c; }

.cred-hint {
  font-size: 12px;
  color: #909399;
  line-height: 1.4;
  margin-top: 4px;
  display: flex;
  align-items: center;
  gap: 4px;
}
.cred-hint .el-icon {
  font-size: 13px;
  flex-shrink: 0;
}
.cred-link {
  color: var(--el-color-primary);
  text-decoration: none;
}
.cred-link:hover {
  text-decoration: underline;
}

/* 对话框分组标题 */
.divider-text {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}
.divider-text .el-icon {
  color: var(--el-color-primary);
  font-size: 16px;
}
:deep(.el-divider--horizontal) {
  margin: 8px 0 18px;
}

/* 表单项提示文字 */
.form-tip {
  display: block;
  font-size: 12px;
  color: #909399;
  line-height: 1.4;
  margin-top: 4px;
}

/* 脚本编辑器 */
.script-editor-wrap {
  width: 100%;
}
.script-editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: #f5f7fa;
  border: 1px solid #dcdfe6;
  border-bottom: none;
  border-radius: 4px 4px 0 0;
}
.script-editor-hint {
  font-size: 12px;
  color: #909399;
}
.script-editor-hint strong {
  color: var(--el-color-primary);
}
:deep(.script-textarea .el-textarea__inner) {
  border-radius: 0 0 4px 4px;
  font-family: 'JetBrains Mono', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.5;
}

:deep(.no-grow-tabs .el-tabs__item) {
  height: 36px;
}
</style>

<!-- 对话框全局样式（el-dialog 通过 teleport 渲染到 body，需用非 scoped 样式） -->
<style>
.job-form-dialog {
  max-width: 96vw;
}
.job-form-dialog .el-dialog__body {
  padding: 12px 24px 16px;
}
.job-form-dialog .el-form-item {
  margin-bottom: 18px;
}
</style>
