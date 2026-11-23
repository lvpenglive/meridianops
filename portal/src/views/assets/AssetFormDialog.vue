<template>
  <el-dialog
    :model-value="modelValue"
    :title="mode === 'edit' ? '编辑资产' : '新增资产'"
    width="640px"
    destroy-on-close
    @update:model-value="emit('update:modelValue', $event)"
    @closed="onClosed"
  >
    <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px">
      <div class="form-section">
        <div class="section-title">基本信息</div>
        <el-form-item v-if="mode === 'create'" label="资产类型" prop="modelId">
          <el-select v-model="form.modelId" placeholder="选择 CI 模型" style="width: 100%" @change="onModelChange">
            <el-option v-for="m in models" :key="m.id" :label="m.name" :value="m.id" />
          </el-select>
        </el-form-item>
        <el-form-item v-else label="资产类型">
          <el-tag>{{ modelLabel }}</el-tag>
        </el-form-item>
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" placeholder="资产名称" clearable />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="form.status" style="width: 100%">
            <el-option label="运行中" value="running" />
            <el-option label="已停止" value="stopped" />
            <el-option label="维护中" value="maintenance" />
            <el-option label="未知" value="unknown" />
          </el-select>
        </el-form-item>
        <el-form-item label="标签">
          <el-input v-model="form.tags" placeholder="多个标签用逗号分隔" clearable />
        </el-form-item>
        <el-form-item label="负责人">
          <el-select
            v-model="form.ownerIds"
            multiple
            filterable
            clearable
            collapse-tags
            collapse-tags-tooltip
            placeholder="可多选系统用户"
            style="width: 100%"
          >
            <el-option
              v-for="u in userOptions"
              :key="u.id"
              :label="userOptionLabel(u)"
              :value="u.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="SSH 凭据">
          <el-select
            v-model="form.sshCredentialId"
            filterable
            clearable
            placeholder="用于作业/Agent 启停的登录凭据"
            style="width: 100%"
          >
            <el-option
              v-for="c in credentialOptions"
              :key="c.id"
              :label="`${c.name}（${c.username} · ${c.authType === 'key' ? '私钥' : '密码'}）`"
              :value="c.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="SSH 通道">
          <el-select v-model="form.sshExecMode" style="width: 100%">
            <el-option label="自动（按监控 Proxy 解析跳板）" value="auto" />
            <el-option label="强制直连" value="direct" />
          </el-select>
        </el-form-item>
      </div>

      <div v-if="attrs.length" class="form-section">
        <div class="section-title">扩展属性</div>
        <el-form-item
          v-for="attr in attrs"
          :key="attr.code"
          :label="attr.name"
          :prop="'attributes.' + attr.code"
          :rules="attr.isRequired ? [{ required: true, message: `${attr.name}不能为空`, trigger: 'blur' }] : []"
        >
          <el-select
            v-if="attr.valueType === 'enum'"
            v-model="form.attributes[attr.code]"
            :placeholder="`选择${attr.name}`"
            clearable
            style="width: 100%"
          >
            <el-option v-for="opt in attr.options || []" :key="opt" :label="opt" :value="opt" />
          </el-select>
          <el-input-number
            v-else-if="attr.valueType === 'number'"
            v-model="form.attributes[attr.code]"
            :controls="false"
            style="width: 100%"
          />
          <el-switch
            v-else-if="attr.valueType === 'boolean'"
            v-model="form.attributes[attr.code]"
          />
          <el-input
            v-else
            v-model="form.attributes[attr.code]"
            :placeholder="`输入${attr.name}`"
            clearable
          />
        </el-form-item>
      </div>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :loading="saving" @click="onSubmit">
        {{ mode === 'edit' ? '保存' : '确定' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import {
  createCiInstance,
  getCiModel,
  listCmdbUserOptions,
  updateCiInstance,
} from '../../api/cmdb'
import { listAllCredentials, type SshCredentialSimple } from '../../api/credential'
import type { CiInstance, CiModel, CiModelAttr, CmdbUserOption } from '../../api/types'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    mode: 'create' | 'edit'
    models?: CiModel[]
    /** 编辑时的实例；创建时可省略 */
    instance?: CiInstance | null
    /** 创建时预选模型 */
    defaultModelId?: string
  }>(),
  {
    models: () => [],
    instance: null,
    defaultModelId: '',
  },
)

const emit = defineEmits<{
  'update:modelValue': [boolean]
  saved: []
}>()

const formRef = ref<FormInstance>()
const saving = ref(false)
const attrs = ref<CiModelAttr[]>([])
const userOptions = ref<CmdbUserOption[]>([])
const credentialOptions = ref<SshCredentialSimple[]>([])

const form = reactive({
  id: '',
  modelId: '',
  name: '',
  status: 'running',
  tags: '',
  ownerIds: [] as string[],
  sshCredentialId: null as number | null,
  sshExecMode: 'auto' as 'auto' | 'direct',
  attributes: {} as Record<string, any>,
})

const formRules: FormRules = {
  modelId: [{ required: true, message: '请选择资产类型', trigger: 'change' }],
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const modelLabel = computed(() => {
  const fromList = props.models.find((m) => m.id === form.modelId)?.name
  return fromList || form.modelId || '—'
})

function userOptionLabel(u: CmdbUserOption) {
  const name = u.displayName?.trim()
  return name && name !== u.username ? `${name}（${u.username}）` : u.username
}

function resetForm() {
  form.id = ''
  form.modelId = ''
  form.name = ''
  form.status = 'running'
  form.tags = ''
  form.ownerIds = []
  form.sshCredentialId = null
  form.sshExecMode = 'auto'
  form.attributes = {}
  attrs.value = []
}

async function loadOptions() {
  const [users, creds] = await Promise.all([
    listCmdbUserOptions().catch(() => [] as CmdbUserOption[]),
    listAllCredentials().catch(() => [] as SshCredentialSimple[]),
  ])
  userOptions.value = users
  credentialOptions.value = creds
}

async function onModelChange(modelId: string) {
  if (!modelId) {
    attrs.value = []
    return
  }
  try {
    const detail = await getCiModel(modelId)
    attrs.value = detail.attributes
  } catch {
    attrs.value = []
  }
  for (const attr of attrs.value) {
    if (form.attributes[attr.code] === undefined) {
      if (attr.defaultValue) {
        form.attributes[attr.code] =
          attr.valueType === 'number' ? Number(attr.defaultValue) : attr.defaultValue
      } else {
        form.attributes[attr.code] = attr.valueType === 'number' ? undefined : ''
      }
    }
  }
}

async function hydrate() {
  resetForm()
  await loadOptions()
  if (props.mode === 'edit' && props.instance) {
    const row = props.instance
    form.id = row.id
    form.modelId = row.modelId
    form.name = row.name
    form.status = row.status
    form.tags = row.tags || ''
    form.ownerIds = row.ownerIds?.length
      ? [...row.ownerIds]
      : row.ownerId
        ? [row.ownerId]
        : []
    form.sshCredentialId = row.sshCredentialId ?? null
    form.sshExecMode = row.sshExecMode === 'direct' ? 'direct' : 'auto'
    form.attributes = { ...(row.attributes || {}) }
    await onModelChange(row.modelId)
  } else {
    if (props.defaultModelId) {
      form.modelId = props.defaultModelId
      await onModelChange(props.defaultModelId)
    }
  }
}

function onClosed() {
  resetForm()
  formRef.value?.resetFields()
}

async function onSubmit() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    saving.value = true
    try {
      const payload = {
        name: form.name.trim(),
        status: form.status,
        tags: form.tags,
        ownerIds: form.ownerIds,
        attributes: form.attributes,
        sshCredentialId: form.sshCredentialId,
        sshExecMode: form.sshExecMode === 'direct' ? 'direct' : null,
      }
      if (props.mode === 'edit') {
        await updateCiInstance(form.id, payload)
        ElMessage.success('更新成功')
      } else {
        const created = await createCiInstance({
          modelId: form.modelId,
          name: payload.name,
          status: payload.status,
          tags: payload.tags,
          ownerIds: payload.ownerIds,
          attributes: payload.attributes,
        })
        if ((form.sshCredentialId != null || form.sshExecMode === 'direct') && created?.id) {
          await updateCiInstance(created.id, {
            name: payload.name,
            status: payload.status,
            tags: payload.tags,
            ownerIds: payload.ownerIds,
            attributes: payload.attributes,
            sshCredentialId: form.sshCredentialId,
            sshExecMode: payload.sshExecMode,
          })
        }
        ElMessage.success('创建成功')
      }
      emit('update:modelValue', false)
      emit('saved')
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : '操作失败'
      ElMessage.error(msg)
    } finally {
      saving.value = false
    }
  })
}

watch(
  () => props.modelValue,
  (open) => {
    if (open) void hydrate()
  },
)
</script>

<style scoped>
.form-section {
  margin-bottom: 8px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-secondary);
  margin-bottom: 12px;
}
</style>
