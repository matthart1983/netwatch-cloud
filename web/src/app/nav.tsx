'use client'

import Link from 'next/link'
import { useAuth } from '@/lib/auth'

export function Nav() {
  const { token, isLoading, logout } = useAuth()

  return (
    <nav className="border-b border-zinc-800 bg-zinc-900/50">
      <div className="max-w-6xl mx-auto px-4 h-14 flex items-center justify-between">
        <Link href="/" className="text-lg font-bold text-emerald-400">
          NetWatch
        </Link>
        {!isLoading && (
          <div className="flex items-center gap-4">
            {token ? (
              <>
                <Link href="/" className="text-sm text-zinc-400 hover:text-zinc-100">
                  Hosts
                </Link>
                <Link href="/settings" className="text-sm text-zinc-400 hover:text-zinc-100">
                  Settings
                </Link>
                <button
                  onClick={logout}
                  className="text-sm text-zinc-400 hover:text-zinc-100"
                >
                  Logout
                </button>
              </>
            ) : (
              <>
                <Link href="/login" className="text-sm text-zinc-400 hover:text-zinc-100">
                  Login
                </Link>
                <Link
                  href="/register"
                  className="text-sm bg-emerald-600 hover:bg-emerald-500 text-white px-3 py-1.5 rounded"
                >
                  Sign Up
                </Link>
              </>
            )}
          </div>
        )}
      </div>
    </nav>
  )
}
