import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { useMutation } from "@tanstack/react-query"
import { toast } from "sonner"
import { KeyRound, Loader2, LogOut } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { invoke } from "@/lib/tauri"
import { useAuth } from "@/context/AuthContext"

const MIN_LENGTH = 8

const changePasswordSchema = z
  .object({
    ancien: z.string().min(1, "L'ancien mot de passe est requis"),
    nouveau: z.string().min(MIN_LENGTH, `Minimum ${MIN_LENGTH} caractères`),
    confirmation: z.string(),
  })
  .refine((d) => d.nouveau === d.confirmation, {
    message: "Les mots de passe ne correspondent pas",
    path: ["confirmation"],
  })
  .refine((d) => d.nouveau !== d.ancien, {
    message: "Le nouveau mot de passe doit être différent de l'ancien",
    path: ["nouveau"],
  })

type ChangePasswordForm = z.infer<typeof changePasswordSchema>

export default function ChangePasswordRequired() {
  const { user, completePasswordChange, logout } = useAuth()

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<ChangePasswordForm>({
    resolver: zodResolver(changePasswordSchema),
    defaultValues: { ancien: "", nouveau: "", confirmation: "" },
  })

  const mutation = useMutation({
    mutationFn: (data: ChangePasswordForm) =>
      invoke("change_password", {
        ancienMotDePasse: data.ancien,
        nouveauMotDePasse: data.nouveau,
      }),
    onSuccess: () => {
      toast.success("Mot de passe modifié")
      completePasswordChange()
    },
    onError: (err) => toast.error("Changement refusé", { description: String(err) }),
  })

  return (
    <div className="flex min-h-[60vh] items-center justify-center p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="h-5 w-5" />
            Changement de mot de passe obligatoire
          </CardTitle>
          <CardDescription>
            Le mot de passe de <strong>{user?.login}</strong> est le mot de passe par défaut ou trop faible.
            Choisissez-en un nouveau pour continuer.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit((data) => mutation.mutate(data))} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="ancien">Mot de passe actuel</Label>
              <Input id="ancien" type="password" autoComplete="current-password" {...register("ancien")} />
              {errors.ancien && <p className="text-sm text-destructive">{errors.ancien.message}</p>}
            </div>
            <div className="space-y-2">
              <Label htmlFor="nouveau">Nouveau mot de passe</Label>
              <Input id="nouveau" type="password" autoComplete="new-password" {...register("nouveau")} />
              {errors.nouveau && <p className="text-sm text-destructive">{errors.nouveau.message}</p>}
            </div>
            <div className="space-y-2">
              <Label htmlFor="confirmation">Confirmer le nouveau mot de passe</Label>
              <Input id="confirmation" type="password" autoComplete="new-password" {...register("confirmation")} />
              {errors.confirmation && <p className="text-sm text-destructive">{errors.confirmation.message}</p>}
            </div>
            <div className="flex gap-2 pt-2">
              <Button type="button" variant="outline" onClick={logout} disabled={mutation.isPending}>
                <LogOut className="h-4 w-4 mr-2" />
                Déconnexion
              </Button>
              <Button type="submit" className="flex-1" disabled={mutation.isPending}>
                {mutation.isPending && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                Enregistrer
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
