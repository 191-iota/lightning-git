import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import api, { onUnauthorized } from '@/services/api'
import type { LoginPayload, LoginRes, RegisterPayload, User } from '@/types/api'

export const useAuthStore = defineStore('auth', () => {
  const TOKEN_KEY = 'token'
  const USER_KEY = 'user'

  const storedUser = localStorage.getItem(USER_KEY)
  const user = ref<User | null>(storedUser ? (JSON.parse(storedUser) as User) : null)
  const token = ref<string | null>(localStorage.getItem(TOKEN_KEY))

  const isAuthenticated = computed(() => !!token.value)

  function setSession(newToken: string, newUser: User) {
    token.value = newToken
    user.value = newUser
    localStorage.setItem(TOKEN_KEY, newToken)
    localStorage.setItem(USER_KEY, JSON.stringify(newUser))
  }

  function clearAuth() {
    user.value = null
    token.value = null
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(USER_KEY)
  }

  // Keeps reactive state in sync when the axios interceptor sees a 401.
  onUnauthorized(clearAuth)

  async function login(credentials: LoginPayload) {
    const { data } = await api.post<LoginRes>('/login', credentials)
    setSession(data.access_token, { id: data.user_id, email: data.email })
  }

  async function register(payload: RegisterPayload) {
    await api.post('/register', payload)
  }

  function logout() {
    clearAuth()
  }

  return {
    user,
    token,
    isAuthenticated,
    login,
    register,
    logout,
  }
})
