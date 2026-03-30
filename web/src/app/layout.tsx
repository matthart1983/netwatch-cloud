import type { Metadata } from 'next'
import { Geist, Geist_Mono } from 'next/font/google'
import './globals.css'
import { AuthProvider } from '@/lib/auth'
import { Nav } from './nav'

const sans = Geist({ subsets: ['latin'], variable: '--font-sans' })
const mono = Geist_Mono({ subsets: ['latin'], variable: '--font-mono' })

export const metadata: Metadata = {
  title: 'NetWatch Cloud',
  description: 'Lightweight network monitoring for Linux fleets. Real-time metrics, instant alerts, 2-minute setup.',
  openGraph: {
    title: 'NetWatch Cloud',
    description: 'Network monitoring without the complexity.',
    images: ['/og.svg'],
  },
  icons: {
    icon: '/favicon.svg',
  },
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${sans.variable} ${mono.variable} font-sans bg-zinc-950 text-zinc-100 min-h-screen`}>
        <AuthProvider>
          <Nav />
          <main className="max-w-6xl mx-auto px-4 py-6 has-[.dashboard-wide]:max-w-screen-2xl">
            {children}
          </main>
        </AuthProvider>
      </body>
    </html>
  )
}
