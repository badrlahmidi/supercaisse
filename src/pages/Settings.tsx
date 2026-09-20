import { useState, useEffect } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { invoke } from "@/lib/tauri"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/ui/Tabs"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/Dialog"
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "@/ui/Table"
import { Badge } from "@/ui/Badge"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { toast } from "sonner"
import PageHeader from "@/components/PageHeader"
import { useAuth } from "@/context/AuthContext"
import { User, Shield, Database, Printer, Settings as SettingsIcon, Loader2, Eye, EyeOff, Trash2, Download, AlertTriangle } from "lucide-react"


interface User {
  id: number
  login: string
  nom: string
  role: string
}

interface Utilisateur {
  id: number
  login: string
  nom: string
  role: string
}

const userSchema = z.object({
  login: z.string().min(3, "Login minimum 3 caractères"),
  nom: z.string().min(1, "Nom requis"),
  role: z.enum(["admin", "manager", "caissier"]),
  password: z.string().min(6, "Mot de passe minimum 6 caractères").optional(),
  confirmPassword: z.string().optional(),
}).refine((data) => !data.password || data.password === data.confirmPassword, {
  message: "Les mots de passe ne correspondent pas",
  path: ["confirmPassword"],
})

type UserForm = z.infer<typeof userSchema>

const settingsSchema = z.object({
  shop_name: z.string().min(1),
  shop_address: z.string().optional(),
  shop_phone: z.string().optional(),
  shop_email: z.string().email().optional().or(z.literal("")),
  ice: z.string().optional(),
  if_number: z.string().optional(),
  rc_number: z.string().optional(),
  patente: z.string().optional(),
  default_tva: z.number().min(0).max(100).default(20),
  receipt_footer: z.string().optional(),
  currency: z.string().default("MAD"),
  printer_name: z.string().optional().default("POS-80"),
  fidelite_actif: z.string().optional().default("true"),
  fidelite_dh_pour_1_point: z.string().optional().default("100"),
  fidelite_valeur_1_point: z.string().optional().default("1"),
  business_type: z.enum(["standard", "restaurant", "mode", "vrac"]).default("standard"),
})

type SettingsForm = z.infer<typeof settingsSchema>

export default function Settings() {
  const queryClient = useQueryClient()
  const { user: currentUser } = useAuth()
  const [showPassword, setShowPassword] = useState(false)
  const [activeTab, setActiveTab] = useState("general")
  const [editingUser, setEditingUser] = useState<Utilisateur | null>(null)
  const [showUserForm, setShowUserForm] = useState(false)
  const [deleteUserConfirm, setDeleteUserConfirm] = useState<Utilisateur | null>(null)

  const { data: users } = useQuery({
    queryKey: ["utilisateurs"],
    queryFn: () => invoke<Utilisateur[]>("get_utilisateurs"),
  })

  const { data: settings } = useQuery({
    queryKey: ["settings"],
    queryFn: () => invoke<SettingsForm>("get_settings"),
  })

  useEffect(() => {
    if (settings) {
      settingsForm.reset(settings)
    }
  }, [settings])

  const createUserMutation = useMutation({
    mutationFn: (data: UserForm) => invoke("add_utilisateur", data),
    onSuccess: () => {
      toast.success("Utilisateur créé")
      queryClient.invalidateQueries({ queryKey: ["utilisateurs"] })
      setShowUserForm(false)
    },
    onError: (err) => toast.error(String(err)),
  })

  const updateUserMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: UserForm }) => invoke("update_utilisateur", { id, ...data }),
    onSuccess: () => {
      toast.success("Utilisateur mis à jour")
      queryClient.invalidateQueries({ queryKey: ["utilisateurs"] })
      setEditingUser(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  const deleteUserMutation = useMutation({
    mutationFn: (id: number) => invoke("delete_utilisateur", { id }),
    onSuccess: () => {
      toast.success("Utilisateur supprimé")
      queryClient.invalidateQueries({ queryKey: ["utilisateurs"] })
    },
    onError: (err) => toast.error(String(err)),
  })

  const updateSettingsMutation = useMutation({
    mutationFn: (data: SettingsForm) => invoke("update_settings", data),
    onSuccess: () => toast.success("Paramètres enregistrés"),
    onError: (err) => toast.error(String(err)),
  })

  const userForm = useForm<UserForm>({
    resolver: zodResolver(userSchema),
    defaultValues: { login: "", nom: "", role: "caissier", password: "", confirmPassword: "" },
  })

  const settingsForm = useForm<SettingsForm>({
    resolver: zodResolver(settingsSchema),
    defaultValues: {
      shop_name: "SuperCaisse",
      shop_address: "",
      shop_phone: "",
      shop_email: "",
      ice: "",
      if_number: "",
      rc_number: "",
      patente: "",
      default_tva: 20,
      receipt_footer: "Merci de votre visite",
      currency: "MAD",
    },
  })

  const handleUserSubmit = (data: UserForm) => {
    const userData = {
      ...data,
      role: data.role as "admin" | "manager" | "caissier",
    }
    if (editingUser) {
      updateUserMutation.mutate({ id: editingUser.id, data: userData })
    } else {
      createUserMutation.mutate(userData)
    }
  }

  const openEditUser = (u: Utilisateur) => {
    setEditingUser(u)
    userForm.reset({ login: u.login, nom: u.nom, role: u.role as "admin" | "manager" | "caissier", password: "", confirmPassword: "" })
    setShowUserForm(true)
  }

  const openCreateUser = () => {
    setEditingUser(null)
    userForm.reset({ login: "", nom: "", role: "caissier", password: "", confirmPassword: "" })
    setShowUserForm(true)
  }

  const handleSettingsSubmit = (data: SettingsForm) => {
    updateSettingsMutation.mutate(data)
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Paramètres" description="Configuration du système" />

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full grid-cols-5">
          <TabsTrigger value="general">Général</TabsTrigger>
          <TabsTrigger value="users">Utilisateurs</TabsTrigger>
          <TabsTrigger value="receipt">Ticket</TabsTrigger>
          <TabsTrigger value="system">Système</TabsTrigger>
          <TabsTrigger value="backup">Sauvegarde</TabsTrigger>
        </TabsList>

        <TabsContent value="general" className="space-y-6">
          <form onSubmit={settingsForm.handleSubmit(handleSettingsSubmit)} className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <SettingsIcon className="h-5 w-5" />
                  Informations du magasin
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="nom_magasin">Nom du magasin *</Label>
                    <Input {...settingsForm.register("shop_name")} id="nom_magasin" placeholder="SuperCaisse" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="devise">Devise</Label>
                    <Input {...settingsForm.register("currency")} id="devise" placeholder="MAD" />
                  </div>
                  <div className="space-y-2 md:col-span-2">
                    <Label htmlFor="adresse">Adresse</Label>
                    <Input {...settingsForm.register("shop_address")} id="adresse" placeholder="Adresse complète" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="telephone">Téléphone</Label>
                    <Input {...settingsForm.register("shop_phone")} id="telephone" placeholder="+212 5 XX XX XX XX" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="email">Email</Label>
                    <Input type="email" {...settingsForm.register("shop_email")} id="email" placeholder="contact@magasin.ma" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="tva_defaut">TVA par défaut (%)</Label>
                    <Input type="number" min="0" max="100" step="0.1" {...settingsForm.register("default_tva", { valueAsNumber: true })} id="tva_defaut" />
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Identifiants légaux (facture conforme DGI)</CardTitle>
                <CardDescription>ICE, IF, RC et Patente sont 4 identifiants distincts exigés sur toute facture professionnelle au Maroc</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="ice">ICE (Identifiant Commun de l'Entreprise)</Label>
                    <Input {...settingsForm.register("ice")} id="ice" placeholder="15 chiffres" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="if_number">IF (Identifiant Fiscal)</Label>
                    <Input {...settingsForm.register("if_number")} id="if_number" placeholder="Optionnel" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="rc_number">RC (Registre de Commerce)</Label>
                    <Input {...settingsForm.register("rc_number")} id="rc_number" placeholder="Optionnel" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="patente">Patente</Label>
                    <Input {...settingsForm.register("patente")} id="patente" placeholder="Optionnel" />
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Type de commerce (Mode d'activité)</CardTitle>
                <CardDescription>L'interface s'adaptera automatiquement aux besoins spécifiques de votre métier</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  <Label htmlFor="business_type">Secteur d'activité *</Label>
                  <Select 
                    value={settingsForm.watch("business_type") || "standard"} 
                    onValueChange={(v) => settingsForm.setValue("business_type", v as any)}
                  >
                    <SelectTrigger id="business_type" className="w-full max-w-md">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="standard">Supermarché / Épicerie (Standard)</SelectItem>
                      <SelectItem value="restaurant">Restaurant / Café (Tables & Cuisine)</SelectItem>
                      <SelectItem value="mode">Boutique Mode / Prêt-à-porter (Tailles/Couleurs)</SelectItem>
                      <SelectItem value="vrac">Vrac / Boucherie (Balances connectées)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Programme de fidélité</CardTitle>
                <CardDescription>Configurez la récompense accordée aux clients</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center gap-4 mb-4">
                  <Label htmlFor="fidelite_actif" className="flex-1">Activer le programme de fidélité</Label>
                  <Select 
                    value={settingsForm.watch("fidelite_actif") || "true"} 
                    onValueChange={(v) => settingsForm.setValue("fidelite_actif", v)}
                  >
                    <SelectTrigger id="fidelite_actif" className="w-[120px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="true">Oui</SelectItem>
                      <SelectItem value="false">Non</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="fidelite_dh_pour_1_point">Montant d'achat pour 1 Point (DH)</Label>
                    <Input id="fidelite_dh_pour_1_point" type="number" {...settingsForm.register("fidelite_dh_pour_1_point")} />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="fidelite_valeur_1_point">Valeur de réduction d'1 Point (DH)</Label>
                    <Input id="fidelite_valeur_1_point" type="number" {...settingsForm.register("fidelite_valeur_1_point")} />
                  </div>
                </div>
              </CardContent>
            </Card>

            <div className="flex justify-end">
              <Button type="submit" disabled={updateSettingsMutation.isPending}>
                {updateSettingsMutation.isPending ? (
                  <><Loader2 className="h-4 w-4 animate-spin mr-2" />Enregistrement...</>
                ) : (
                  "Enregistrer les paramètres"
                )}
              </Button>
            </div>
          </form>
        </TabsContent>

        <TabsContent value="users" className="space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">Gestion des utilisateurs</h2>
            <Button onClick={openCreateUser}>
              <User className="h-4 w-4 mr-2" />
              Nouvel utilisateur
            </Button>
          </div>

          <Card>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Login</TableHead>
                    <TableHead>Nom</TableHead>
                    <TableHead>Rôle</TableHead>
                    <TableHead className="w-[120px]">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {users?.map((u) => (
                    <TableRow key={u.id}>
                      <TableCell className="font-mono text-sm">{u.login}</TableCell>
                      <TableCell>{u.nom}</TableCell>
                      <TableCell>
                        <Badge variant={
                          u.role === "admin" ? "destructive" :
                          u.role === "manager" ? "warning" : "default"
                        }>
                          {u.role}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          <Button variant="ghost" size="icon" onClick={() => openEditUser(u)}>
                            <SettingsIcon className="h-4 w-4" />
                          </Button>
                          {u.id !== currentUser?.id && (
                            <Button variant="ghost" size="icon" onClick={() => setDeleteUserConfirm(u)}>
                              <Trash2 className="h-4 w-4 text-destructive" />
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                {!users?.length && (
                  <TableRow>
                    <TableCell colSpan={4} className="text-center py-8 text-muted-foreground">
                      Aucun utilisateur
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
          </Card>

          <Dialog open={showUserForm} onOpenChange={setShowUserForm}>
            <DialogContent className="max-w-md">
              <DialogHeader>
                <DialogTitle>{editingUser ? "Modifier l'utilisateur" : "Nouvel utilisateur"}</DialogTitle>
              </DialogHeader>
              <form onSubmit={userForm.handleSubmit(handleUserSubmit)} className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="login">Login *</Label>
                  <Input {...userForm.register("login")} id="login" disabled={!!editingUser} />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="nom">Nom complet *</Label>
                  <Input {...userForm.register("nom")} id="nom" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="role">Rôle *</Label>
                  <Select
                    value={userForm.watch("role") || "caissier"}
                    onValueChange={(v) => userForm.setValue("role", v as "admin" | "manager" | "caissier")}
                  >
                    <SelectTrigger id="role">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="admin">Administrateur</SelectItem>
                      <SelectItem value="manager">Gestionnaire</SelectItem>
                      <SelectItem value="caissier">Caissier</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="password">{editingUser ? "Nouveau mot de passe (laisser vide pour ne pas changer)" : "Mot de passe *"}</Label>
                  <Input
                    type={showPassword ? "text" : "password"}
                    {...userForm.register("password")}
                    id="password"
                    autoComplete={editingUser ? "new-password" : "current-password"}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="confirmPassword">Confirmer le mot de passe</Label>
                  <Input
                    type={showPassword ? "text" : "password"}
                    {...userForm.register("confirmPassword")}
                    id="confirmPassword"
                    autoComplete={editingUser ? "new-password" : "current-password"}
                  />
                  {userForm.formState.errors.confirmPassword && (
                    <p className="text-sm text-destructive">{userForm.formState.errors.confirmPassword.message}</p>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <Button type="button" variant="ghost" size="sm" onClick={() => setShowPassword(!showPassword)}>
                    {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </Button>
                </div>
                <DialogFooter>
                  <Button type="button" variant="outline" onClick={() => setShowUserForm(false)}>
                    Annuler
                  </Button>
                  <Button type="submit" disabled={createUserMutation.isPending || updateUserMutation.isPending}>
                    {(createUserMutation.isPending || updateUserMutation.isPending) ? (
                      <>
                        <Loader2 className="h-4 w-4 animate-spin mr-2" />
                        Enregistrement...
                      </>
                    ) : (
                      "Enregistrer"
                    )}
                  </Button>
                </DialogFooter>
              </form>
            </DialogContent>
          </Dialog>
        </TabsContent>

        <TabsContent value="receipt" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Printer className="h-5 w-5" />
                Configuration du ticket
              </CardTitle>
              <CardDescription>Personnalisez l'apparence de vos tickets de caisse</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="printer_name">Imprimante Thermique (Partage Windows ou Port)</Label>
                <Input id="printer_name" {...settingsForm.register("printer_name")} placeholder="POS-80" />
                <p className="text-xs text-muted-foreground">Exemples : POS-80, \\localhost\Tickets, COM1</p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="receipt_footer">Pied de page du ticket</Label>
                <Input {...settingsForm.register("receipt_footer")} id="receipt_footer" placeholder="Merci de votre visite" />
              </div>
              <div className="p-4 bg-muted rounded-lg border border-dashed">
                <p className="text-sm font-medium mb-2">Aperçu du ticket</p>
                <div className="font-mono text-sm text-muted-foreground">
                  <div className="text-center font-bold">{settingsForm.watch("shop_name") || "SuperCaisse"}</div>
                  <div className="text-center text-xs">{settingsForm.watch("shop_address") || "Adresse du magasin"}</div>
                  <div className="text-center text-xs">{settingsForm.watch("shop_phone") || "Téléphone"}</div>
                  <div className="my-2 border-t" />
                  <div className="text-center text-xs mt-2">{settingsForm.watch("receipt_footer") || "Merci de votre visite"}</div>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="system" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Shield className="h-5 w-5" />
                Sécurité & Sessions
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Verrouillage automatique</p>
                  <p className="text-sm text-muted-foreground">Verrouiller l'application après inactivité</p>
                </div>
                <Button variant="outline">Configurer</Button>
              </div>
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Journal d'audit</p>
                  <p className="text-sm text-muted-foreground">Tracer toutes les actions sensibles</p>
                </div>
                <Button variant="outline">Voir les logs</Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Database className="h-5 w-5" />
                Base de données
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Sauvegarde automatique</p>
                  <p className="text-sm text-muted-foreground">Créer des sauvegardes planifiées</p>
                </div>
                <Button variant="outline">Configurer</Button>
              </div>
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Sauvegarde manuelle</p>
                  <p className="text-sm text-muted-foreground">Exporter la base de données maintenant</p>
                </div>
                <Button onClick={() => invoke("backup_database")}>
                  <Download className="h-4 w-4 mr-2" />
                  Sauvegarder maintenant
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="backup" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Database className="h-5 w-5" />
                Sauvegarde & Restauration
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Exporter la base de données</p>
                  <p className="text-sm text-muted-foreground">Télécharger une copie complète au format SQL</p>
                </div>
                <Button onClick={() => invoke("export_database")}>
                  <Download className="h-4 w-4 mr-2" />
                  Exporter
                </Button>
              </div>
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Importer une sauvegarde</p>
                  <p className="text-sm text-muted-foreground text-destructive">⚠️ Remplace toutes les données actuelles</p>
                </div>
                <Button variant="destructive" onClick={() => invoke("import_database")}>
                  Importer
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <Dialog open={!!deleteUserConfirm} onOpenChange={() => setDeleteUserConfirm(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="h-5 w-5" />
              Confirmer la suppression
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Êtes-vous sûr de vouloir supprimer <strong>{deleteUserConfirm?.nom}</strong> ({deleteUserConfirm?.login}) ? Cette action est irréversible.
          </p>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={() => setDeleteUserConfirm(null)}>Annuler</Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (deleteUserConfirm) deleteUserMutation.mutate(deleteUserConfirm.id)
                setDeleteUserConfirm(null)
              }}
              disabled={deleteUserMutation.isPending}
            >
              {deleteUserMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Trash2 className="h-4 w-4 mr-2" />}
              Supprimer
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}