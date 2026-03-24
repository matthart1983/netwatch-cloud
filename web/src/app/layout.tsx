import type { Metadata } from 'next'
import { Geist_Mono } from 'next/font/google'
import './globals.css'
import { AuthProvider } from '@/lib/auth'
import { Nav } from './nav'

const mono = Geist_Mono({ subsets: ['latin'], variable: '--font-mono' })

export const metadata: Metadata = {
  title: 'NetWatch Cloud',
  description: 'Network monitoring for your fleet',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${mono.variable} font-mono bg-zinc-950 text-zinc-100 min-h-screen`}>
        <AuthProvider>
          <Nav />
          <main className="max-w-6xl mx-auto px-4 py-6">
            {children}
          </main>
        </AuthProvider>
      </body>
    </html>
  )
}
