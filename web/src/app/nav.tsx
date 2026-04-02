'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useAuth } from '@/lib/auth'

const signedInLinks = [
  { href: '/', label: 'Fleet' },
  { href: '/alerts', label: 'Alerts' },
  { href: '/settings', label: 'Settings' },
]

export function Nav() {
  const { token, isLoading, logout } = useAuth()
  const pathname = usePathname()

  // Landing page has its own nav
  if (!isLoading && !token && pathname === '/') return null

  return (
    <nav className="sticky top-0 z-40 border-b border-white/6 bg-[#08111a]/78 backdrop-blur-xl">
      <div className="mx-auto flex h-[4.5rem] w-full max-w-[1320px] items-center justify-between px-4 sm:px-6 lg:px-8">
        <Link href="/" className="group flex items-center gap-3">
          <div className="flex h-11 w-11 items-center justify-center rounded-2xl border border-[rgba(61,214,198,0.25)] bg-[rgba(61,214,198,0.12)] text-sm font-semibold text-[var(--nw-text)] shadow-[0_10px_30px_rgba(61,214,198,0.18)]">
            NW
          </div>
          <div className="leading-tight">
            <div className="text-[0.68rem] font-semibold uppercase tracking-[0.24em] text-[var(--nw-text-soft)]">
              NetWatch Cloud
            </div>
            <div className="text-sm font-semibold text-[var(--nw-text)] group-hover:text-[var(--nw-accent)]">
              Fleet intelligence for Linux infrastructure
            </div>
          </div>
        </Link>

        {!isLoading && (
          <div className="flex items-center gap-3">
            {token ? (
              <>
                <div className="hidden rounded-full border border-white/8 bg-white/4 p-1 md:flex md:items-center md:gap-1">
                  {signedInLinks.map(link => {
                    const isActive = link.href === '/'
                      ? pathname === '/'
                      : pathname?.startsWith(link.href)

                    return (
                      <Link
                        key={link.href}
                        href={link.href}
                        className={`rounded-full px-3 py-2 text-sm font-medium transition-colors ${
                          isActive
                            ? 'bg-[rgba(61,214,198,0.16)] text-[var(--nw-text)]'
                            : 'text-[var(--nw-text-muted)] hover:text-[var(--nw-text)]'
                        }`}
                      >
                        {link.label}
                      </Link>
                    )
                  })}
                </div>
                <div className="hidden rounded-full border border-[rgba(61,214,198,0.18)] bg-[rgba(61,214,198,0.08)] px-3 py-2 text-xs font-semibold uppercase tracking-[0.18em] text-[#a9fff4] lg:inline-flex">
                  Live
                </div>
                <button
                  onClick={logout}
                  className="nw-button-ghost px-4 py-2 text-sm"
                >
                  Logout
                </button>
              </>
            ) : (
              <>
                <Link href="/login" className="nw-button-ghost px-4 py-2 text-sm">
                  Sign in
                </Link>
                <Link
                  href="/register"
                  className="nw-button-primary px-4 py-2 text-sm"
                >
                  Start free trial
                </Link>
              </>
            )}
          </div>
        )}
      </div>
    </nav>
  )
}
