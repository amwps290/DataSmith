<template>
  <a-modal
    v-model:open="visible"
    :title="$t('dialog.backup_database.title', { database })"
    width="600px"
    @ok="handleBackup"
    @cancel="handleCancel"
    :confirm-loading="backing"
  >
    <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
      <a-form-item :label="$t('dialog.backup_database.content')">
        <a-checkbox-group v-model:value="backupOptions">
          <a-checkbox value="structure">{{ $t('dialog.backup_database.structure') }}</a-checkbox>
          <a-checkbox value="data">{{ $t('dialog.backup_database.table_data') }}</a-checkbox>
          <a-checkbox value="views">{{ $t('dialog.backup_database.views') }}</a-checkbox>
          <a-checkbox value="procedures">{{ $t('dialog.backup_database.procedures') }}</a-checkbox>
          <a-checkbox value="functions">{{ $t('dialog.backup_database.functions') }}</a-checkbox>
          <a-checkbox value="triggers">{{ $t('dialog.backup_database.triggers') }}</a-checkbox>
        </a-checkbox-group>
      </a-form-item>

      <a-form-item :label="$t('dialog.backup_database.save_path')" required>
        <a-input
          v-model:value="savePath"
          :placeholder="$t('dialog.backup_database.save_path_placeholder')"
          readonly
          @click="selectSavePath"
        >
          <template #suffix>
            <FolderOpenOutlined style="cursor: pointer" @click="selectSavePath" />
          </template>
        </a-input>
      </a-form-item>

      <a-form-item :label="$t('dialog.backup_database.compress')">
        <a-switch v-model:checked="compress" />
        <span style="margin-left: 8px; color: #999; font-size: 12px;">
          {{ $t('dialog.backup_database.compress_tip') }}
        </span>
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<script setup lang="ts">
import { ref, watch} from 'vue'
import { FolderOpenOutlined } from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import { metadataApi, queryApi, exportApi, utilsApi } from '@/api'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { downloadDir } from '@tauri-apps/api/path'
import { useDialogModel } from '@/composables/useDialogModel'

const { t } = useI18n()

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
}>()

const emit = defineEmits(['update:modelValue', 'backed'])

const visible = useDialogModel(props, emit)

const backing = ref(false)
const backupOptions = ref(['structure', 'data'])
const savePath = ref('')
const compress = ref(false)

// 生成默认文件名
function getDefaultFileName(): string {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, -5)
  const extension = compress.value ? '.sql.gz' : '.sql'
  return `${props.database}_backup_${timestamp}${extension}`
}

// 当对话框打开时，设置默认保存路径（Downloads目录）
watch(() => props.modelValue, async (newVal) => {
  if (newVal && !savePath.value) {
    try {
      const downloadsPath = await downloadDir()
      const fileName = getDefaultFileName()
      savePath.value = `${downloadsPath}\\${fileName}`
    } catch (error) {
      console.error('Failed to get download dir:', error)
      savePath.value = getDefaultFileName()
    }
  }
})

// 当压缩选项改变时，更新文件扩展名
watch(compress, async () => {
  try {
    const downloadsPath = await downloadDir()
    const fileName = getDefaultFileName()
    savePath.value = `${downloadsPath}\\${fileName}`
  } catch (error) {
    savePath.value = getDefaultFileName()
  }
})

async function selectSavePath() {
  const defaultPath = getDefaultFileName()

  const path = await save({
    defaultPath,
    filters: [{
      name: t('dialog.backup_database.sql_file'),
      extensions: compress.value ? ['sql.gz'] : ['sql'],
    }],
  })

  if (path) {
    savePath.value = path
  }
}

async function handleBackup() {
  if (!savePath.value) {
    message.error(t('dialog.backup_database.save_path_required'))
    return
  }

  if (backupOptions.value.length === 0) {
    message.error(t('dialog.backup_database.content_required'))
    return
  }

  backing.value = true
  try {
    let backupSql = `-- ${t('dialog.backup_database.comment_backup')}: ${props.database}\n`
    backupSql += `-- ${t('dialog.backup_database.comment_time')}: ${new Date().toLocaleString()}\n\n`

    // 备份表结构和数据
    if (backupOptions.value.includes('structure') || backupOptions.value.includes('data')) {
      const tables = await metadataApi.getTables(props.connectionId, props.database)

      for (const table of tables) {
        // 导出表结构
        if (backupOptions.value.includes('structure')) {
          const ddl = await exportApi.tableDdl(
            props.connectionId,
            props.database,
            table.name,
          )
          backupSql += `\n-- ${t('dialog.backup_database.comment_structure')}: ${table.name}\n`
          backupSql += `DROP TABLE IF EXISTS \`${table.name}\`;\n`
          backupSql += `${ddl};\n\n`
        }

        // 导出表数据
        if (backupOptions.value.includes('data')) {
          const result = await queryApi.executeQuery(
            props.connectionId,
            `SELECT * FROM \`${table.name}\``,
            props.database,
          )

          const resultData = result[0]
          if (resultData && resultData.rows && resultData.rows.length > 0) {
            backupSql += `-- ${t('dialog.backup_database.comment_data')}: ${table.name}\n`

            for (const row of resultData.rows) {
              const columns = Object.keys(row)
              const values = columns.map(col => {
                const val = row[col]
                if (val === null) return 'NULL'
                if (typeof val === 'string') return `'${val.replace(/'/g, "''")}'`
                return val
              })

              backupSql += `INSERT INTO \`${table.name}\` (\`${columns.join('`, `')}\`) VALUES (${values.join(', ')});\n`
            }
            backupSql += '\n'
          }
        }
      }
    }

    // 备份视图
    if (backupOptions.value.includes('views')) {
      const views = await metadataApi.getViews(props.connectionId, props.database)

      for (const view of views) {
        const definition = await invoke<string>('get_view_definition', {
          connectionId: props.connectionId,
          database: props.database,
          view: view.name,
        })
        backupSql += `\n-- ${t('dialog.backup_database.comment_view')}: ${view.name}\n`
        backupSql += `DROP VIEW IF EXISTS \`${view.name}\`;\n`
        backupSql += `${definition};\n\n`
      }
    }

    // 保存到文件
    await utilsApi.writeFile(savePath.value, backupSql)

    // 显示备份成功提示
    Modal.success({
      title: t('dialog.backup_database.success_title'),
      content: t('dialog.backup_database.success_content', { database: props.database, path: savePath.value }),
      okText: t('common.ok'),
    })

    emit('backed')
    handleCancel()
  } catch (error: unknown) {
    message.error(t('dialog.backup_database.fail', { error: String(error) }))
  } finally {
    backing.value = false
  }
}

function handleCancel() {
  backupOptions.value = ['structure', 'data']
  savePath.value = ''
  compress.value = false
  visible.value = false
}
</script>
