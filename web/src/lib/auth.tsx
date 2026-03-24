'use client'

import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { useRouter } from 'next/navigation'

interface AuthContextType {
  token: string | null
  accountId: string | null
  isLoading: boolean
  login: (token: string, accountId: string) => void
  logout: () => void
}

const AuthContext = createContext<AuthContextType>({
  token: null,
  accountId: null,
  isLoading: true,
  login: () => {},
  logout: () => {},
})

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(null)
  const [accountId, setAccountId] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const router = useRouter()

  useEffect(() => {
    const savedToken = localStorage.getItem('token')
    const savedAccountId = localStorage.getItem('accountId')
    if (savedToken) {
      setToken(savedToken)
      setAccountId(savedAccountId)
    }
    setIsLoading(false)
  }, [])

  const loginFn = (newToken: string, newAccountId: string) => {
    localStorage.setItem('token', newToken)
    localStorage.setItem('accountId', newAccountId)
    setToken(newToken)
    setAccountId(newAccountId)
  }

  const logout = () => {
    localStorage.removeItem('token')
    localStorage.removeItem('accountId')
    setToken(null)
    setAccountId(null)
    router.push('/login')
  }

  return (
    <AuthContext value={{ token, accountId, isLoading, login: loginFn, logout }}>
      {children}
    </AuthContext>
  )
}

export function useAuth() {
  return useContext(AuthContext)
}
