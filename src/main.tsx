import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router-dom'
import { QueryClientProvider } from '@tanstack/react-query'
import { creerQueryClient } from '@/lib/queryClient'
import { Toaster } from '@/ui/Toast'
import { router } from '@/routes/router'
import { initTheme } from '@/store/ui'
import { initLocale } from '@/store/i18n'
import { installerJournalGlobal } from '@/lib/journal'
import './index.css'

installerJournalGlobal()
initTheme()
initLocale()

const queryClient = creerQueryClient()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <Toaster />
    </QueryClientProvider>
  </StrictMode>,
)