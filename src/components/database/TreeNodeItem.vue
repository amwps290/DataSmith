<template>
  <div class="tree-node" :class="{ 'is-connection': node.type === 'connection' }">
    <div
      :class="['tree-node-content', { selected: isSelected }]"
      :style="{ paddingLeft: (level * 15 + 20) + 'px' }"
      @click="handleClick"
      @dblclick="handleDblClick"
      @contextmenu="handleContextMenu"
    >
      <!-- 垂直引导线 -->
      <div 
        v-for="i in (level + (isExpanded && hasChildren ? 1 : 0))" 
        :key="i"
        :class="['tree-line', { 'is-current': i === level + 1 }]"
        :style="{ left: ((i-1) * 15 + 27) + 'px' }"
      ></div>

      <span class="tree-node-expand" @click="handleToggle">
        <DownOutlined v-if="hasChildren && isExpanded" />
        <RightOutlined v-else-if="hasChildren" />
        <span v-else style="display: inline-block; width: 16px;"></span>
      </span>
      <span class="tree-node-icon">
        <LoadingOutlined v-if="isLoading" spin />
        <component 
          v-else 
          :is="getIcon(node.type)" 
          :class="['icon-' + node.type]"
        />
      </span>
      <span class="tree-node-title" :class="{ 'bold': node.type === 'connection' }">
        {{ node.title }}
      </span>
      <a-spin v-if="isLoading" size="small" style="margin-left: 8px;" />
    </div>
    <div v-if="isExpanded && node.children && node.children.length > 0" class="tree-node-children">
      <TreeNodeItem
        v-for="child in node.children"
        :key="child.key"
        :node="child"
        :level="level + 1"
        :expanded-keys="expandedKeys"
        :selected-keys="selectedKeys"
        :loading-nodes="loadingNodes"
        @toggle="$emit('toggle', $event)"
        @select="$emit('select', $event)"
        @dblclick="$emit('dblclick', $event)"
        @contextmenu="$emit('contextmenu', $event)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  DatabaseOutlined,
  TableOutlined,
  FolderOutlined,
  FileOutlined,
  EyeOutlined,
  LoadingOutlined,
  RightOutlined,
  DownOutlined,
  AppstoreOutlined,
  FunctionOutlined,
  CalculatorOutlined,
  OrderedListOutlined,
  ThunderboltOutlined,
  ApiOutlined,
  BlockOutlined,
  ClusterOutlined,
  KeyOutlined,
  FieldStringOutlined,
} from '@ant-design/icons-vue'

interface TreeNode {
  key: string
  title: string
  type: string
  isLeaf?: boolean
  children?: TreeNode[]
  metadata?: any
}

const props = defineProps<{
  node: TreeNode
  level: number
  expandedKeys: string[]
  selectedKeys: string[]
  loadingNodes: Set<string>
}>()

const emit = defineEmits<{
  toggle: [node: TreeNode]
  select: [node: TreeNode]
  dblclick: [node: TreeNode]
  contextmenu: [payload: { event: MouseEvent; node: TreeNode }]
}>()

const isExpanded = computed(() => props.expandedKeys.includes(props.node.key))
const isSelected = computed(() => props.selectedKeys.includes(props.node.key))
const isLoading = computed(() => props.loadingNodes.has(props.node.key))
const hasChildren = computed(() => !props.node.isLeaf && props.node.type !== 'empty')

const handleToggle = (e: Event) => {
  e.stopPropagation()
  if (hasChildren.value) {
    emit('toggle', props.node)
  }
}

const handleClick = () => {
  emit('select', props.node)
}

const handleDblClick = () => {
  emit('dblclick', props.node)
}

const handleContextMenu = (e: MouseEvent) => {
  emit('contextmenu', { event: e, node: props.node })
}

const getIcon = (type: string) => {
  if (type === 'column') {
    return props.node.metadata?.is_primary_key ? KeyOutlined : FieldStringOutlined
  }
  
  const iconMap: Record<string, any> = {
    connection: ClusterOutlined, 
    database: DatabaseOutlined,  
    schemas: AppstoreOutlined,
    schema: AppstoreOutlined,
    'schema-tables': TableOutlined,
    'schema-views': EyeOutlined,
    'schema-functions': FunctionOutlined,
    'schema-aggregate-functions': CalculatorOutlined,
    'schema-indexes': OrderedListOutlined,
    'schema-extensions': BlockOutlined,
    tables: TableOutlined,
    table: TableOutlined,
    views: EyeOutlined,
    view: EyeOutlined,
    procedures: ThunderboltOutlined,
    procedure: ThunderboltOutlined,
    functions: FunctionOutlined,
    function: FunctionOutlined,
    'aggregate-function': CalculatorOutlined,
    triggers: ApiOutlined,
    trigger: ApiOutlined,
    events: FileOutlined,
    event: FileOutlined,
    indexes: OrderedListOutlined,
    index: OrderedListOutlined,
    'database-extensions': BlockOutlined,
    extensions: BlockOutlined,
    extension: BlockOutlined,
    collections: TableOutlined,
    collection: TableOutlined,
    keys: FolderOutlined,
    'redis-key': FileOutlined,
  }
  return iconMap[type] || FileOutlined
}
</script>

<style scoped>
.tree-node {
  width: 100%;
  position: relative;
}

.tree-node-content {
  display: flex;
  align-items: center;
  padding: 2px 4px;
  cursor: pointer;
  user-select: none;
  transition: background-color 0.1s;
  border-radius: 2px;
  position: relative;
  height: 24px;
}

.tree-node-content:hover {
  background-color: rgba(0, 0, 0, 0.04);
}

.dark-mode .tree-node-content:hover {
  background-color: rgba(255, 255, 255, 0.05);
}

.tree-node-content.selected {
  background-color: #e6f7ff;
  color: #1890ff;
}

.dark-mode .tree-node-content.selected {
  background-color: #111b26;
  color: #177ddc;
}

/* 垂直引导线样式 - 使用极浅灰色 */
.tree-line {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background-color: #f0f0f0; 
  pointer-events: none;
}

/* 当前节点的子引导线从箭头中间开始 */
.tree-line.is-current {
  top: 12px;
}

.dark-mode .tree-line {
  background-color: #262626;
}

.tree-node-expand {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  margin-right: 4px;
  font-size: 10px;
  color: #bfbfbf;
  flex-shrink: 0;
  z-index: 2;
}

.tree-node-expand:hover {
  color: #1890ff;
}

.tree-node-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin-right: 6px;
  font-size: 14px;
  color: #8c8c8c;
  flex-shrink: 0;
  z-index: 2;
}

.icon-connection { color: #1890ff; }
.icon-database { color: #fa8c16; }
.icon-table { color: #52c41a; }
.icon-schema { color: #722ed1; }
.icon-column { color: #8c8c8c; }
.icon-column.anticon-key { color: #ffc53d; }

.tree-node-content.selected .tree-node-icon {
  color: inherit;
}

.tree-node-title {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
}

.tree-node-title.bold {
  font-weight: 600;
  color: rgba(0, 0, 0, 0.85);
}

.dark-mode .tree-node-title.bold {
  color: rgba(255, 255, 255, 0.85);
}

.tree-node-children {
  width: 100%;
}
</style>
