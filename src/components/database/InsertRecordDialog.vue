<template>
  <a-modal
    v-model:open="visible"
    :title="$t('dialog.insert_record.title', { table })"
    width="700px"
    @ok="handleInsert"
    @cancel="handleCancel"
    :confirm-loading="inserting"
  >
    <a-spin :spinning="loadingColumns">
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item
          v-for="col in columns"
          :key="col.name"
          :label="col.name"
          :required="!col.nullable && !col.is_auto_increment"
        >
          <a-input
            v-if="isTextType(col.data_type)"
            v-model:value="formData[col.name]"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
          />
          <a-input-number
            v-else-if="isNumericType(col.data_type)"
            v-model:value="formData[col.name]"
            style="width: 100%"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
          />
          <a-date-picker
            v-else-if="isDateType(col.data_type)"
            v-model:value="formData[col.name]"
            show-time
            style="width: 100%"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
          />
          <a-switch
            v-else-if="isBooleanType(col.data_type)"
            v-model:checked="formData[col.name]"
            :disabled="col.is_auto_increment"
          />
          <a-textarea
            v-else
            v-model:value="formData[col.name]"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
            :rows="3"
          />
          <div v-if="col.comment" style="font-size: 12px; color: #999; margin-top: 4px;">
            {{ col.comment }}
          </div>
        </a-form-item>
      </a-form>
    </a-spin>
  </a-modal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { message } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import { metadataApi, dataApi } from '@/api'
import { useDialogModel } from '@/composables/useDialogModel'

interface InsertColumn {
  name: string
  data_type: string
  nullable: boolean
  is_auto_increment: boolean
  default_value: string | null
  comment: string
}

const { t } = useI18n()

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
  table: string
  schema?: string
}>()

const emit = defineEmits(['update:modelValue', 'inserted'])

const visible = useDialogModel(props, emit)

const inserting = ref(false)
const loadingColumns = ref(false)
const columns = ref<InsertColumn[]>([])
const formData = ref<Record<string, any>>({})

function normalizeType(dataType: string): string {
  return dataType.trim().toLowerCase()
}

function isTextType(dataType: string): boolean {
  const normalized = normalizeType(dataType)
  return ['varchar', 'char', 'text', 'character varying', 'character'].includes(normalized)
}

function isNumericType(dataType: string): boolean {
  const normalized = normalizeType(dataType)
  return ['int', 'integer', 'bigint', 'smallint', 'tinyint', 'decimal', 'numeric', 'float', 'double', 'real'].includes(normalized)
}

function isDateType(dataType: string): boolean {
  const normalized = normalizeType(dataType)
  return ['date', 'datetime', 'timestamp', 'timestamp without time zone', 'timestamp with time zone'].includes(normalized)
}

function isBooleanType(dataType: string): boolean {
  const normalized = normalizeType(dataType)
  return ['boolean', 'bool', 'tinyint(1)'].includes(normalized)
}

function getPlaceholder(col: InsertColumn): string {
  if (col.is_auto_increment) {
    return t('dialog.insert_record.auto_generated')
  }
  if (col.default_value) {
    return t('dialog.insert_record.default_value', { value: col.default_value })
  }
  if (col.nullable) {
    return t('dialog.insert_record.optional')
  }
  return t('dialog.insert_record.required')
}

async function loadTableStructure() {
  if (!props.table) return

  loadingColumns.value = true
  try {
    const result = await metadataApi.getTableStructure({
      connectionId: props.connectionId,
      table: props.table,
      schema: props.schema || null,
      database: props.database,
    }) as unknown as InsertColumn[]

    columns.value = result
    formData.value = {}
  } catch (error: unknown) {
    message.error(t('dialog.insert_record.load_fail', { error: String(error) }))
  } finally {
    loadingColumns.value = false
  }
}

async function handleInsert() {
  // 验证必填字段
  for (const col of columns.value) {
    const currentValue = formData.value[col.name]
    if (!col.nullable && !col.is_auto_increment && (currentValue === undefined || currentValue === null || currentValue === '')) {
      message.error(t('dialog.insert_record.field_required', { field: col.name }))
      return
    }
  }

  inserting.value = true
  try {
    // 过滤掉自增字段和空值
    const data: Record<string, any> = {}
    for (const col of columns.value) {
      if (!col.is_auto_increment && formData.value[col.name] !== undefined && formData.value[col.name] !== '') {
        data[col.name] = formData.value[col.name]
      }
    }

    await dataApi.insertTableData({
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      schema: props.schema,
      data,
    })

    message.success(t('dialog.insert_record.success'))
    emit('inserted')
    handleCancel()
  } catch (error: unknown) {
    message.error(t('dialog.insert_record.fail', { error: String(error) }))
  } finally {
    inserting.value = false
  }
}

function handleCancel() {
  formData.value = {}
  visible.value = false
}

watch(visible, (newVal) => {
  if (newVal) {
    loadTableStructure()
  }
})
</script>

<style scoped>
:deep(.ant-form-item) {
  margin-bottom: 16px;
}
</style>
