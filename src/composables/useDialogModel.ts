import { computed } from 'vue'

export function useDialogModel(
  props: { modelValue: boolean },
  emit: (event: 'update:modelValue', value: boolean) => void
) {
  return computed({
    get: () => props.modelValue,
    set: (val: boolean) => emit('update:modelValue', val),
  })
}
