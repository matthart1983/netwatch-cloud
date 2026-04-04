'use client'

import { createContext, useContext, useEffect, useReducer, ReactNode } from 'react'
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

interface AuthState {
  token: string | null
  accountId: string | null
  isLoading: boolean
}

type AuthAction =
  | { type: 'hydrate'; token: string | null; accountId: string | null }
  | { type: 'login'; token: string; accountId: string }
  | { type: 'logout' }

function authReducer(state: AuthState, action: AuthAction): AuthState {
  switch (action.type) {
    case 'hydrate':
      return {
        token: action.token,
        accountId: action.accountId,
        isLoading: false,
      }
    case 'login':
      return {
        token: action.token,
        accountId: action.accountId,
        isLoading: false,
      }
    case 'logout':
      return {
        token: null,
        accountId: null,
        isLoading: false,
      }
    default:
      return state
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(authReducer, {
    token: null,
    accountId: null,
    isLoading: true,
  })
  const router = useRouter()

  useEffect(() => {
    dispatch({
      type: 'hydrate',
      token: localStorage.getItem('token'),
      accountId: localStorage.getItem('accountId'),
    })
  }, [])

  const loginFn = (newToken: string, newAccountId: string) => {
    localStorage.setItem('token', newToken)
    localStorage.setItem('accountId', newAccountId)
    dispatch({ type: 'login', token: newToken, accountId: newAccountId })
  }

  const logout = () => {
    localStorage.removeItem('token')
    localStorage.removeItem('accountId')
    dispatch({ type: 'logout' })
    router.push('/login')
  }

  return (
    <AuthContext
      value={{
        token: state.token,
        accountId: state.accountId,
        isLoading: state.isLoading,
        login: loginFn,
        logout,
      }}
    >
      {children}
    </AuthContext>
  )
}

export function useAuth() {
  return useContext(AuthContext)
}
