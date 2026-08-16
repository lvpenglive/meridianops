<template>
  <div class="page-wrap">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Key /></el-icon>
        <span>SSH 凭据管理</span>
      </div>
      <div class="header-actions">
        <el-input
          v-model="query.keyword"
          placeholder="搜索名称/用户名/描述"
          clearable
          style="width: 240px"
          @keyup.enter="loadList"
          @clear="loadList"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-button :icon="Refresh" @click="loadList">刷新</el-button>
        <el-button v-if="canCreate" type="primary" :icon="Plus" @click="openCreate">新建凭据</el-button>
      </div>
    </div>

    <el-alert type="info" :closable="false" class="mb-12" show-icon>
      <template #title>
        SSH 凭据用于作业中心「SSH 执行器」类型的作业。密码与私钥使用 AES-256-GCM 加密存储，列表与详情接口均不返回明文。
      </template>
    </el-alert>

    <el-table v-loading="loading" :data="list" border stripe class="mt-12">
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="name" label="凭据名称" min-width="150">
        <template #default="{ row }">
          <div class="cred-name-cell">
            <el-icon><Key /></el-icon>
            <span>{{ row.name }}</span>
          </div>
        </template>
      </el-table-column>
      <el-table-column label="认证方式" width="110" align="center">
        <template #default="{ row }">
          <el-tag size="small" :type="row.authType === 'key' ? 'warning' : 'success'" effect="light">
            {{ row.authType === 'key' ? '私钥' : '密码' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="username" label="SSH 用户名" width="140" />
      <el-table-column prop="hostKeyFingerprint" label="主机密钥指纹" min-width="180" show-overflow-tooltip>
        <template #default="{ row }">
          <span v-if="row.hostKeyFingerprint" class="mono-text">{{ row.hostKeyFingerprint }}</span>
          <span v-else class="muted">未设置</span>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="160" show-overflow-tooltip />
      <el-table-column prop="createdBy" label="创建人" width="110" />
      <el-table-column prop="updatedAt" label="更新时间" width="170" />
      <el-table-column label="操作" width="160" fixed="right">
        <template #default="{ row }">
          <el-button v-if="canCreate" link :icon="Edit" @click="openEdit(row)">编辑</el-button>
          <el-popconfirm
            v-if="canDelete"
            title="确定删除该凭据？（被作业引用时无法删除）"
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
        v-model:current-page="query.page"
        v-model:page-size="query.pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        background
        @size-change="loadList"
        @current-change="loadList"
      />
    </div>

    <!-- ============ 新建/编辑对话框 ============ -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑凭据' : '新建凭据'"
      width="600px"
      :close-on-click-modal="false"
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="120px">
        <el-form-item label="凭据名称" prop="name">
          <el-input v-model="form.name" placeholder="例如：生产环境 root 凭据" maxlength="128" show-word-limit />
        </el-form-item>
        <el-form-item label="认证方式" prop="authType">
          <el-radio-group v-model="form.authType">
            <el-radio value="password">密码认证</el-radio>
            <el-radio value="key">私钥认证</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="SSH 用户名" prop="username">
          <el-input v-model="form.username" placeholder="例如：root / ops" maxlength="128" />
        </el-form-item>

        <template v-if="form.authType === 'password'">
          <el-form-item :label="isEdit ? '新密码' : '密码'" prop="password">
            <el-input
              v-model="form.password"
              type="password"
              show-password
              :placeholder="isEdit ? '留空则不修改原密码' : '请输入密码'"
            />
          </el-form-item>
        </template>

        <template v-else>
          <el-form-item :label="isEdit ? '新私钥' : '私钥'" prop="privateKey">
            <el-input
              v-model="form.privateKey"
              type="textarea"
              :rows="6"
              :placeholder="isEdit ? '留空则不修改原私钥' : '粘贴 PEM 格式私钥（-----BEGIN ... PRIVATE KEY-----）'"
              spellcheck="false"
            />
          </el-form-item>
          <el-form-item label="私钥口令" prop="passphrase">
            <el-input
              v-model="form.passphrase"
              type="password"
              show-password
              :placeholder="isEdit ? '留空则不修改原口令' : '无私钥口令可留空'"
            />
          </el-form-item>
        </template>

        <el-form-item label="主机密钥指纹" prop="hostKeyFingerprint">
          <el-input
            v-model="form.hostKeyFingerprint"
            placeholder="可选，如 SHA256:xxxx（留空表示不校验主机密钥）"
          />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="form.description" placeholder="简要描述凭据用途" maxlength="500" show-word-limit />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, onMounted } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Search, Refresh, Plus, Edit, Delete, Key } from '@element-plus/icons-vue'
import {
  listCredentials, getCredential, createCredential, updateCredential, deleteCredential,
  type SshCredential, type AuthType, type CreateCredentialPayload,
} from '../../api/credential'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const canCreate = computed(() => userStore.hasPermission('credential:create'))
const canDelete = computed(() => userStore.hasPermission('credential:delete'))

// ===== 列表 =====
const loading = ref(false)
const list = ref<SshCredential[]>([])
const total = ref(0)
const query = reactive({ page: 1, pageSize: 20, keyword: '' })

async function loadList() {
  loading.value = true
  try {
    const res = await listCredentials(query)
    list.value = res.list
    total.value = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载凭据列表失败')
  } finally {
    loading.value = false
  }
}

// ===== 新建/编辑 =====
const dialogVisible = ref(false)
const isEdit = ref(false)
const editingId = ref<number | null>(null)
const formRef = ref<FormInstance>()
const form = reactive({
  name: '',
  authType: 'password' as AuthType,
  username: '',
  password: '',
  privateKey: '',
  passphrase: '',
  hostKeyFingerprint: '',
  description: '',
})
const submitting = ref(false)

const formRules = computed<FormRules>(() => ({
  name: [
    { required: true, message: '请输入凭据名称', trigger: 'blur' },
    { min: 2, max: 128, message: '长度 2-128', trigger: 'blur' },
  ],
  authType: [{ required: true, message: '请选择认证方式', trigger: 'change' }],
  username: [{ required: true, message: '请输入 SSH 用户名', trigger: 'blur' }],
  password: isEdit.value
    ? []
    : [{ required: form.authType === 'password', message: '请输入密码', trigger: 'blur' }],
  privateKey: isEdit.value
    ? []
    : [{ required: form.authType === 'key', message: '请粘贴私钥', trigger: 'blur' }],
}))

function openCreate() {
  isEdit.value = false
  editingId.value = null
  Object.assign(form, {
    name: '', authType: 'password', username: '',
    password: '', privateKey: '', passphrase: '',
    hostKeyFingerprint: '', description: '',
  })
  dialogVisible.value = true
}

async function openEdit(row: SshCredential) {
  try {
    const detail = await getCredential(row.id)
    isEdit.value = true
    editingId.value = row.id
    Object.assign(form, {
      name: detail.name,
      authType: detail.authType,
      username: detail.username,
      password: '',
      privateKey: '',
      passphrase: '',
      hostKeyFingerprint: detail.hostKeyFingerprint || '',
      description: detail.description || '',
    })
    dialogVisible.value = true
  } catch (e: any) {
    ElMessage.error(e?.message || '加载凭据详情失败')
  }
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    // 构造 payload：编辑时空的敏感字段不传（后端保持原值）
    const payload: CreateCredentialPayload = {
      name: form.name.trim(),
      authType: form.authType,
      username: form.username.trim(),
      hostKeyFingerprint: form.hostKeyFingerprint.trim() || undefined,
      description: form.description.trim() || undefined,
    }
    if (form.authType === 'password' && form.password) {
      payload.password = form.password
    }
    if (form.authType === 'key') {
      if (form.privateKey) payload.privateKey = form.privateKey
      if (form.passphrase) payload.passphrase = form.passphrase
    }

    submitting.value = true
    try {
      if (isEdit.value && editingId.value) {
        await updateCredential(editingId.value, payload)
        ElMessage.success('凭据已更新')
      } else {
        await createCredential(payload)
        ElMessage.success('凭据已创建')
      }
      dialogVisible.value = false
      loadList()
    } catch (e: any) {
      ElMessage.error(e?.message || '保存失败')
    } finally {
      submitting.value = false
    }
  })
}

async function handleDelete(row: SshCredential) {
  try {
    await deleteCredential(row.id)
    ElMessage.success('凭据已删除')
    loadList()
  } catch (e: any) {
    ElMessage.error(e?.message || '删除失败')
  }
}

onMounted(() => {
  loadList()
})
</script>

<style scoped>
.page-wrap {
  padding: 16px 20px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.page-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 600;
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.cred-name-cell {
  display: flex;
  align-items: center;
  gap: 6px;
}
.mono-text {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
  color: #606266;
}
.muted {
  color: #c0c4cc;
}
.mt-12 {
  margin-top: 12px;
}
.mb-12 {
  margin-bottom: 12px;
}
.pagination-wrap {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
