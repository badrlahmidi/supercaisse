import { useState } from "react"
import { useNavigate, useLocation } from "react-router-dom"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Card, CardContent } from "@/ui/Card"
import { ShoppingCart, Lock, User, Eye, EyeOff, ShieldCheck, BarChart3, Package, TrendingUp } from "lucide-react"
import { toast } from "sonner"
import { invoke } from "@/lib/tauri"
import { useAuth, type SessionUser } from "@/context/AuthContext"

const loginSchema = z.object({
  login: z.string().min(1, "Le login est requis"),
  password: z.string().min(1, "Le mot de passe est requis"),
})

type LoginForm = z.infer<typeof loginSchema>

const features = [
  { icon: Package, label: "Gestion des stocks" },
  { icon: BarChart3, label: "Tableau de bord" },
  { icon: TrendingUp, label: "Rapports ventes" },
  { icon: ShieldCheck, label: "Sécurisé" },
]

export default function Login() {
  const navigate = useNavigate()
  const location = useLocation()
  const { loginAs } = useAuth()
  const [showPassword, setShowPassword] = useState(false)
  const [isLoading, setIsLoading] = useState(false)

  const from = (location.state as { from?: Location })?.from?.pathname || "/dashboard"

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<LoginForm>({
    resolver: zodResolver(loginSchema),
    defaultValues: { login: "", password: "" },
  })

  const onSubmit = async (data: LoginForm) => {
    setIsLoading(true)
    try {
      const user = await invoke<SessionUser | null>("login", { login: data.login, password: data.password })
      if (user) {
        await loginAs(user)
        toast.success(`Bienvenue ${user.nom}`)
        navigate(from, { replace: true })
      } else {
        toast.error("Login ou mot de passe incorrect")
      }
    } catch (err) {
      toast.error(String(err))
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen bg-background">
      {/* Left Panel - Branding */}
      <div className="hidden lg:flex lg:w-1/2 relative overflow-hidden bg-gradient-to-br from-[hsl(var(--sidebar-background))] via-[hsl(222_47%_14%)] to-[hsl(var(--sidebar-background))]">
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top_right,hsl(var(--primary)/0.15),transparent_50%)]" />
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_bottom_left,hsl(var(--sidebar-primary)/0.1),transparent_50%)]" />
        <div className="relative z-10 flex flex-col justify-between p-16 w-full">
          <div className="flex items-center gap-3">
            <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-primary shadow-lg shadow-primary/20">
              <ShoppingCart className="h-7 w-7 text-primary-foreground" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-white">SuperCaisse</h1>
              <p className="text-sm text-slate-400">Point de Vente Moderne</p>
            </div>
          </div>

          <div className="space-y-8 max-w-md">
            <div className="space-y-3">
              <h2 className="text-4xl font-bold text-white leading-tight">
                Gérez votre point de vente
                <span className="text-primary"> en toute simplicité</span>
              </h2>
              <p className="text-lg text-slate-400 leading-relaxed">
                Solution complète de gestion commerciale : ventes, stocks, clients et rapports en temps réel.
              </p>
            </div>

            <div className="grid grid-cols-2 gap-4">
              {features.map((f) => {
                const Icon = f.icon
                return (
                  <div key={f.label} className="flex items-center gap-3 p-3 rounded-lg bg-white/5 border border-white/10">
                    <div className="p-2 rounded-lg bg-primary/10">
                      <Icon className="h-4 w-4 text-primary" />
                    </div>
                    <span className="text-sm text-slate-300 font-medium">{f.label}</span>
                  </div>
                )
              })}
            </div>
          </div>

          <p className="text-sm text-slate-500">
            &copy; 2025 SuperCaisse POS — Version 1.0.0
          </p>
        </div>
      </div>

      {/* Right Panel - Form */}
      <div className="flex-1 flex items-center justify-center p-6 relative">
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,hsl(var(--primary)/0.05),transparent_60%)] lg:hidden" />

        <div className="w-full max-w-md relative animate-fade-in">
          {/* Mobile Logo */}
          <div className="lg:hidden text-center mb-8">
            <div className="inline-flex h-16 w-16 items-center justify-center rounded-2xl bg-primary shadow-lg shadow-primary/20 mb-4">
              <ShoppingCart className="h-9 w-9 text-primary-foreground" />
            </div>
            <h1 className="text-3xl font-bold text-foreground">SuperCaisse</h1>
            <p className="text-muted-foreground mt-1">Point de Vente Moderne</p>
          </div>

          <Card className="border-border/50 shadow-xl shadow-black/5">
            <CardContent className="p-8">
              <div className="text-center mb-8">
                <h2 className="text-2xl font-bold text-foreground">Connexion</h2>
                <p className="text-muted-foreground text-sm mt-1">
                  Entrez vos identifiants pour accéder à l'espace de vente
                </p>
              </div>

              <form onSubmit={handleSubmit(onSubmit)} className="space-y-5" noValidate>
                <div className="space-y-2">
                  <Input
                    id="login"
                    label="Login"
                    placeholder="Votre identifiant"
                    leftIcon={<User className="h-5 w-5" />}
                    error={errors.login?.message}
                    {...register("login")}
                    autoComplete="username"
                    autoFocus
                  />
                </div>

                <div className="space-y-2">
                  <Input
                    id="password"
                    label="Mot de passe"
                    type={showPassword ? "text" : "password"}
                    placeholder="Votre mot de passe"
                    leftIcon={<Lock className="h-5 w-5" />}
                    rightIcon={
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        className="text-muted-foreground hover:text-foreground"
                        onClick={() => setShowPassword(!showPassword)}
                        aria-label={showPassword ? "Masquer le mot de passe" : "Afficher le mot de passe"}
                      >
                        {showPassword ? <EyeOff className="h-5 w-5" /> : <Eye className="h-5 w-5" />}
                      </Button>
                    }
                    error={errors.password?.message}
                    {...register("password")}
                    autoComplete="current-password"
                  />
                </div>

                <Button
                  type="submit"
                  className="w-full py-3 text-base"
                  size="lg"
                  disabled={isLoading}
                  loading={isLoading}
                >
                  {isLoading ? "Connexion en cours..." : "Se connecter"}
                </Button>
              </form>

              {import.meta.env.DEV && (
                <div className="mt-8 pt-6 border-t border-border text-center">
                  <p className="text-sm text-muted-foreground">
                    Compte par défaut : <code className="text-foreground font-mono font-medium bg-muted px-2 py-0.5 rounded">admin / admin</code>
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
