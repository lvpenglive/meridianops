import type { Directive, DirectiveBinding } from 'vue'
import { useUserStore } from '../stores/user'

/**
 * v-permission 按钮级权限指令。
 *
 * 用法：
 *   v-permission="'user:create'"            // 单个权限码
 *   v-permission="['user:create','user:update']"  // 任一满足即显示
 *
 * 无权限时直接移除该 DOM 节点（比 v-if 更轻量，适合按钮/链接）。
 * 权限码含 '*' 时通配放行（开发模式匿名用户）。
 */
function check(value: unknown): boolean {
  const store = useUserStore()
  if (!value) return true // 未传值则放行（由调用方自行控制）
  const codes = Array.isArray(value) ? value : [value]
  if (codes.length === 0) return true
  return codes.some((c) => typeof c === 'string' && store.hasPermission(c))
}

function elRemove(el: HTMLElement) {
  el.parentNode?.removeChild(el)
}

export const permission: Directive = {
  mounted(el: HTMLElement, binding: DirectiveBinding) {
    if (!check(binding.value)) {
      elRemove(el)
    }
  },
  // 权限不会在运行时变化（除非重新登录），updated 时无需重复移除，
  // 但保留以应对权限码动态绑定的场景。
  updated(el: HTMLElement, binding: DirectiveBinding) {
    if (!check(binding.value)) {
      elRemove(el)
    }
  },
}
