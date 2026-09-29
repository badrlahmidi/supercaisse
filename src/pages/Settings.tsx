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
import { useForm, type DefaultValues } from "react-hook-form"
import type { PermissionRole } from "@/types/generated/PermissionRole"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { toast } from "sonner"
import PageHeader from "@/components/PageHeader"
import { useAuth } from "@/context/AuthContext"
import { Checkbox } from "@/ui/Checkbox"
import { User, Shield, Database, Printer, Settings as SettingsIcon, Loader2, Eye, EyeOff, Trash2, Download, AlertTriangle, Upload, Palette, X, Languages, Cloud, CloudOff } from "lucide-react"
import { useI18nStore } from "@/store/i18n"
import { iceSaisieValide, ifSaisieValide } from "@/lib/fiscal"
import MisesAJour from "@/components/MisesAJour"
import type { FichierSauvegarde } from "@/types/generated/FichierSauvegarde"
import type { ChoixAppairageCloud } from "@/types/generated/ChoixAppairageCloud"
import type { EtatSynchroCloud } from "@/types/generated/EtatSynchroCloud"

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
  password: z.string().min(8, "Mot de passe minimum 8 caractères").optional().or(z.literal("")),
  confirmPassword: z.string().optional(),
}).refine((data) => !data.password || data.password === data.confirmPassword, {
  message: "Les mots de passe ne correspondent pas",
  path: ["confirmPassword"],
})

type UserForm = z.infer<typeof userSchema>

function pourcentageValide(valeur: string | undefined): boolean {
  if (!valeur?.trim()) return true
  const pct = Number(valeur.replace(",", "."))
  return Number.isFinite(pct) && pct >= 0 && pct <= 100
}

const settingsSchema = z.object({
  shop_name: z.string().min(1),
  shop_address: z.string().optional(),
  shop_phone: z.string().optional(),
  shop_email: z.string().email().optional().or(z.literal("")),
  ice: z.string().optional().refine(iceSaisieValide, "ICE invalide : 15 chiffres attendus"),
  if_number: z.string().optional().refine(ifSaisieValide, "IF invalide : chiffres uniquement"),
  rc_number: z.string().optional(),
  patente: z.string().optional(),
  default_tva: z.number().min(0).max(100).default(20),
  receipt_footer: z.string().optional(),
  currency: z.string().default("MAD"),
  printer_name: z.string().optional().default("POS-80"),
  fidelite_actif: z.string().optional().default("true"),
  autoriser_stock_negatif: z.enum(["true", "false"]).optional().default("false"),
  remise_max_caissier: z.string().optional().default("10").refine(pourcentageValide, "Pourcentage entre 0 et 100 attendu"),
  remise_max_manager: z.string().optional().default("100").refine(pourcentageValide, "Pourcentage entre 0 et 100 attendu"),
  fidelite_dh_pour_1_point: z.string().optional().default("100"),
  fidelite_valeur_1_point: z.string().optional().default("1"),
  business_type: z.enum(["standard", "restaurant"]).default("standard"),
  idle_timeout: z.string().optional().default("300"),
  logo_base64: z.string().nullable().optional(),
  receipt_header: z.string().nullable().optional(),
  doc_primary_color: z.string().nullable().optional(),
})

type SettingsForm = z.infer<typeof settingsSchema>

type SauvegardeDisponible = FichierSauvegarde

const VALEURS_PAR_DEFAUT: DefaultValues<SettingsForm> = {
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
}

export default function Settings() {
  const queryClient = useQueryClient()
  const { user: currentUser, logout } = useAuth()
  const { locale, setLocale, t } = useI18nStore()
  const [showPassword, setShowPassword] = useState(false)
  const [activeTab, setActiveTab] = useState("general")
  const [editingUser, setEditingUser] = useState<Utilisateur | null>(null)
  const [showUserForm, setShowUserForm] = useState(false)
  const [deleteUserConfirm, setDeleteUserConfirm] = useState<Utilisateur | null>(null)
  const [showImportConfirm, setShowImportConfirm] = useState(false)
  const [importPath, setImportPath] = useState("")
  const [cloudEmail, setCloudEmail] = useState("")
  const [cloudMotDePasse, setCloudMotDePasse] = useState("")
  const [appairageChoix, setAppairageChoix] = useState<ChoixAppairageCloud | null>(null)
  const [associationsChoisies, setAssociationsChoisies] = useState<Record<number, string>>({})
  const [showDesappairageConfirm, setShowDesappairageConfirm] = useState(false)

  const etatSynchroQuery = useQuery({
    queryKey: ["etat-synchro"],
    queryFn: () => invoke<EtatSynchroCloud>("obtenir_etat_synchro"),
  })

  const appairageMutation = useMutation({
    mutationFn: () =>
      invoke<ChoixAppairageCloud>("demarrer_appairage_cloud", {
        email: cloudEmail.trim(),
        mot_de_passe: cloudMotDePasse,
      }),
    onSuccess: (choix) => {
      setAppairageChoix(choix)
      const premiereBoutiqueCloud = choix.magasins_cloud[0]?.cloud_magasin_id ?? ""
      setAssociationsChoisies(
        Object.fromEntries(choix.magasins_locaux.map((m) => [m.id, premiereBoutiqueCloud]))
      )
    },
    onError: (err) => toast.error("Connexion refusée", { description: String(err) }),
  })

  const annulerAppairageMutation = useMutation({
    mutationFn: () => invoke("annuler_appairage_cloud"),
    onSuccess: () => setAppairageChoix(null),
  })

  const finaliserAppairageMutation = useMutation({
    mutationFn: () =>
      invoke("finaliser_appairage_cloud", {
        associations: Object.entries(associationsChoisies).map(([magasinLocalId, cloudMagasinId]) => ({
          magasin_local_id: Number(magasinLocalId),
          cloud_magasin_id: cloudMagasinId,
        })),
      }),
    onSuccess: () => {
      toast.success("Synchronisation cloud activée")
      setAppairageChoix(null)
      setCloudEmail("")
      setCloudMotDePasse("")
      queryClient.invalidateQueries({ queryKey: ["etat-synchro"] })
    },
    onError: (err) => toast.error("Échec de l'appairage", { description: String(err) }),
  })

  const desappairerMutation = useMutation({
    mutationFn: () => invoke("desappairer_cloud"),
    onSuccess: () => {
      toast.success("Synchronisation cloud désactivée")
      queryClient.invalidateQueries({ queryKey: ["etat-synchro"] })
    },
    onError: (err) => toast.error("Échec de la déconnexion", { description: String(err) }),
  })

  const exportMutation = useMutation({
    mutationFn: () => invoke<string>("export_database"),
    onSuccess: (path) => toast.success("Base exportée", { description: path }),
    onError: (err) => toast.error("Échec de l'export", { description: String(err) }),
  })

  const backupMutation = useMutation({
    mutationFn: () => invoke<string>("backup_database"),
    onSuccess: (path) => {
      toast.success("Sauvegarde créée", { description: path })
      queryClient.invalidateQueries({ queryKey: ["sauvegardes"] })
    },
    onError: (err) => toast.error("Échec de la sauvegarde", { description: String(err) }),
  })

  const { data: sauvegardes = [], isLoading: sauvegardesLoading } = useQuery({
    queryKey: ["sauvegardes"],
    queryFn: () => invoke<SauvegardeDisponible[]>("list_backups"),
    enabled: showImportConfirm,
  })

  const importMutation = useMutation({
    mutationFn: (path: string) => invoke<string>("import_database", { path }),
    onSuccess: (securite) => {
      toast.success("Sauvegarde restaurée", {
        description: `Copie de la base précédente : ${securite}. Reconnectez-vous.`,
        duration: 10000,
      })
      setShowImportConfirm(false)
      setImportPath("")
      queryClient.clear()
      logout()
    },
    onError: (err) => toast.error("Restauration refusée", { description: String(err) }),
  })
  const [pinValue, setPinValue] = useState("")
  const [savingPin, setSavingPin] = useState(false)
  const [permRole, setPermRole] = useState("manager")

  const MODULES = [
    "articles", "categories", "clients", "fournisseurs", "ventes",
    "achats", "stock", "inventaire", "journal", "cheques",
    "rapports", "magasins", "audit", "settings", "reappro",
  ] as const

  const ACTIONS = ["voir", "creer", "modifier", "exporter"] as const

  const MODULE_LABELS: Record<string, string> = {
    articles: "Articles",
    categories: "Catégories",
    clients: "Clients",
    fournisseurs: "Fournisseurs",
    ventes: "Ventes",
    achats: "Achats",
    stock: "Stock",
    inventaire: "Inventaire",
    journal: "Journal de caisse",
    cheques: "Chèques",
    rapports: "Rapports",
    magasins: "Magasins",
    audit: "Audit",
    settings: "Paramètres",
    reappro: "Réappro",
  }

  const ACTION_LABELS: Record<string, string> = {
    voir: "Voir",
    creer: "Créer",
    modifier: "Modifier",
    exporter: "Exporter",
  }

  const { data: users } = useQuery({
    queryKey: ["utilisateurs"],
    queryFn: () => invoke<Utilisateur[]>("get_utilisateurs"),
  })

  const { data: settings } = useQuery({
    queryKey: ["settings"],
    queryFn: () => invoke<SettingsForm>("get_settings"),
  })

  const { data: permissionsData } = useQuery({
    queryKey: ["permissions", permRole],
    queryFn: () => invoke<PermissionRole[]>("get_permissions", { role: permRole }),
    enabled: permRole !== "admin",
  })

  const permMap = (() => {
    const map: Record<string, Record<string, boolean>> = {}
    if (permRole === "admin") {
      for (const m of MODULES) {
        map[m] = {}
        for (const a of ACTIONS) map[m][a] = true
      }
      return map
    }
    if (permissionsData) {
      for (const row of permissionsData) {
        if (!map[row.module]) map[row.module] = {}
        map[row.module][row.action] = row.allowed
      }
    }
    return map
  })()

  const updatePermMutation = useMutation({
    mutationFn: ({ role, module, action, allowed }: { role: string; module: string; action: string; allowed: boolean }) =>
      invoke("update_permission", { role, module, action, allowed }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["permissions", permRole] })
    },
    onError: (err) => toast.error(String(err)),
  })

  const signalerChampsInvalides = (erreurs: Record<string, { message?: string } | undefined>) => {
    const messages = Object.entries(erreurs).map(([champ, erreur]) => erreur?.message || champ)
    toast.error("Paramètres non enregistrés", { description: messages.join(" · ") })
  }

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
    defaultValues: VALEURS_PAR_DEFAUT,
  })

  useEffect(() => {
    if (settings) {
      const renseignes = Object.fromEntries(Object.entries(settings).filter(([, valeur]) => valeur !== null && valeur !== undefined))
      settingsForm.reset({ ...VALEURS_PAR_DEFAUT, ...renseignes } as SettingsForm)
    }
  }, [settings, settingsForm])

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
    setPinValue("")
    setShowUserForm(true)
  }

  const openCreateUser = () => {
    setEditingUser(null)
    userForm.reset({ login: "", nom: "", role: "caissier", password: "", confirmPassword: "" })
    setPinValue("")
    setShowUserForm(true)
  }

  const handleSettingsSubmit = (data: SettingsForm) => {
    updateSettingsMutation.mutate(data)
  }

  return (
    <div className="space-y-6">
      <PageHeader title={t("settings.title")} description={t("settings.description")} />

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full grid-cols-6">
          <TabsTrigger value="general">{t("settings.general")}</TabsTrigger>
          <TabsTrigger value="users">{t("settings.users")}</TabsTrigger>
          <TabsTrigger value="receipt">{t("settings.receipt")}</TabsTrigger>
          <TabsTrigger value="system">{t("settings.systemTab")}</TabsTrigger>
          <TabsTrigger value="backup">{t("settings.backup")}</TabsTrigger>
          <TabsTrigger value="cloud">{t("settings.cloud")}</TabsTrigger>
        </TabsList>

        <TabsContent value="general" className="space-y-6">
          <form onSubmit={settingsForm.handleSubmit(handleSettingsSubmit, signalerChampsInvalides)} className="space-y-6">
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
                    <Input {...settingsForm.register("ice")} id="ice" placeholder="15 chiffres (obligatoire pour facturer)" />
                    {settingsForm.formState.errors.ice && (
                      <p className="text-sm text-destructive">{settingsForm.formState.errors.ice.message}</p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="if_number">IF (Identifiant Fiscal)</Label>
                    <Input {...settingsForm.register("if_number")} id="if_number" placeholder="Obligatoire pour facturer" />
                    {settingsForm.formState.errors.if_number && (
                      <p className="text-sm text-destructive">{settingsForm.formState.errors.if_number.message}</p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="rc_number">RC (Registre de Commerce)</Label>
                    <Input {...settingsForm.register("rc_number")} id="rc_number" placeholder="Obligatoire pour facturer" />
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

            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Stock</CardTitle>
                <CardDescription>
                  Par défaut, une vente, une conversion ou un transfert est refusé si le magasin n'a pas le stock suffisant.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex items-center gap-4">
                  <Label htmlFor="autoriser_stock_negatif" className="flex-1">Autoriser la vente en stock négatif</Label>
                  <Select
                    value={settingsForm.watch("autoriser_stock_negatif") || "false"}
                    onValueChange={(v) => settingsForm.setValue("autoriser_stock_negatif", v as "true" | "false")}
                  >
                    <SelectTrigger id="autoriser_stock_negatif" className="w-[120px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="false">Non</SelectItem>
                      <SelectItem value="true">Oui</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Remises</CardTitle>
                <CardDescription>
                  Remise maximale par ligne, remise document comprise. L'administrateur n'est pas plafonné.
                </CardDescription>
              </CardHeader>
              <CardContent className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="remise_max_caissier">Caissier (%)</Label>
                  <Input {...settingsForm.register("remise_max_caissier")} id="remise_max_caissier" inputMode="decimal" placeholder="10" />
                  {settingsForm.formState.errors.remise_max_caissier && (
                    <p className="text-sm text-destructive">{settingsForm.formState.errors.remise_max_caissier.message}</p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="remise_max_manager">Manager (%)</Label>
                  <Input {...settingsForm.register("remise_max_manager")} id="remise_max_manager" inputMode="decimal" placeholder="100" />
                  {settingsForm.formState.errors.remise_max_manager && (
                    <p className="text-sm text-destructive">{settingsForm.formState.errors.remise_max_manager.message}</p>
                  )}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Languages className="h-5 w-5" />
                  {t("settings.language")}
                </CardTitle>
                <CardDescription>{t("settings.languageDescription")}</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex gap-3">
                  <Button
                    type="button"
                    variant={locale === "fr" ? "default" : "outline"}
                    onClick={() => setLocale("fr")}
                  >
                    {t("settings.french")}
                  </Button>
                  <Button
                    type="button"
                    variant={locale === "ar" ? "default" : "outline"}
                    onClick={() => setLocale("ar")}
                  >
                    {t("settings.arabic")}
                  </Button>
                </div>
              </CardContent>
            </Card>

            <div className="flex justify-end">
              <Button type="submit" disabled={updateSettingsMutation.isPending}>
                {updateSettingsMutation.isPending ? (
                  <><Loader2 className="h-4 w-4 animate-spin mr-2" />{t("settings.saving")}</>
                ) : (
                  t("settings.saveSettings")
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
                <div className="space-y-2">
                  <Label htmlFor="pin">PIN rapide (4 à 6 chiffres, optionnel)</Label>
                  <div className="flex gap-2">
                    <Input
                      type="text"
                      inputMode="numeric"
                      maxLength={6}
                      pattern="[0-9]*"
                      placeholder="ex : 4826"
                      id="pin"
                      value={pinValue}
                      onChange={(e) => setPinValue(e.target.value.replace(/\D/g, "").slice(0, 6))}
                      className="max-w-[120px]"
                    />
                    {editingUser && (
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={pinValue.length < 4 || savingPin}
                        onClick={async () => {
                          if (!editingUser || pinValue.length < 4) return
                          setSavingPin(true)
                          try {
                            await invoke("set_user_pin", { userId: editingUser.id, pin: pinValue })
                            toast.success("PIN enregistré")
                            setPinValue("")
                          } catch (err) {
                            toast.error(String(err))
                          } finally {
                            setSavingPin(false)
                          }
                        }}
                      >
                        {savingPin ? <Loader2 className="h-4 w-4 animate-spin" /> : "Enregistrer le PIN"}
                      </Button>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground">Changement rapide de caissier depuis l'écran de verrouillage. Évitez les chiffres identiques ou qui se suivent ; 5 erreurs bloquent le PIN 5 minutes.</p>
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

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Shield className="h-5 w-5" />
                Permissions par module
              </CardTitle>
              <CardDescription>
                {"Configurez les droits d'accès par rôle pour chaque module de l'application"}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Tabs value={permRole} onValueChange={setPermRole}>
                <TabsList>
                  <TabsTrigger value="admin">Admin</TabsTrigger>
                  <TabsTrigger value="manager">Manager</TabsTrigger>
                  <TabsTrigger value="caissier">Caissier</TabsTrigger>
                </TabsList>

                {(["admin", "manager", "caissier"] as const).map((role) => (
                  <TabsContent key={role} value={role}>
                    {role === "admin" && (
                      <p className="text-sm text-muted-foreground mb-4">
                        {"L'administrateur a tous les droits. Les permissions ne peuvent pas être modifiées."}
                      </p>
                    )}
                    <div className="rounded-md border overflow-auto">
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead className="min-w-[140px]">Module</TableHead>
                            {ACTIONS.map((action) => (
                              <TableHead key={action} className="text-center w-[100px]">
                                {ACTION_LABELS[action]}
                              </TableHead>
                            ))}
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {MODULES.map((mod_) => (
                            <TableRow key={mod_}>
                              <TableCell className="font-medium">{MODULE_LABELS[mod_]}</TableCell>
                              {ACTIONS.map((action) => (
                                <TableCell key={action} className="text-center">
                                  <Checkbox
                                    checked={permMap[mod_]?.[action] ?? false}
                                    disabled={role === "admin"}
                                    onChange={(e) => {
                                      updatePermMutation.mutate({
                                        role,
                                        module: mod_,
                                        action,
                                        allowed: e.target.checked,
                                      })
                                    }}
                                  />
                                </TableCell>
                              ))}
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </div>
                  </TabsContent>
                ))}
              </Tabs>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="receipt" className="space-y-6">
          <form onSubmit={settingsForm.handleSubmit(handleSettingsSubmit, signalerChampsInvalides)} className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Printer className="h-5 w-5" />
                Configuration du ticket
              </CardTitle>
              <CardDescription>Personnalisez l'apparence de vos tickets de caisse et documents PDF</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="printer_name">Imprimante Thermique (Partage Windows ou Port)</Label>
                <Input id="printer_name" {...settingsForm.register("printer_name")} placeholder="POS-80" />
                <p className="text-xs text-muted-foreground">Exemples : POS-80, \\localhost\Tickets, COM1</p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="receipt_header">En-tête du ticket</Label>
                <Input {...settingsForm.register("receipt_header")} id="receipt_header" placeholder="Bienvenue chez nous !" />
                <p className="text-xs text-muted-foreground">Texte affiché sous le logo et le nom du magasin</p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="receipt_footer">Pied de page du ticket</Label>
                <Input {...settingsForm.register("receipt_footer")} id="receipt_footer" placeholder="Merci de votre visite" />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Palette className="h-5 w-5" />
                Personnalisation des documents
              </CardTitle>
              <CardDescription>Logo et couleurs des factures, devis et bons de livraison PDF</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>Logo du magasin</Label>
                <div className="flex items-center gap-4">
                  {settingsForm.watch("logo_base64") ? (
                    <div className="relative">
                      <img
                        src={`data:image/png;base64,${settingsForm.watch("logo_base64")}`}
                        alt="Logo"
                        className="h-16 w-16 object-contain border rounded-lg bg-white p-1"
                      />
                      <button
                        type="button"
                        className="absolute -top-2 -right-2 bg-destructive text-destructive-foreground rounded-full p-0.5"
                        onClick={() => settingsForm.setValue("logo_base64", null)}
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </div>
                  ) : (
                    <div className="h-16 w-16 border-2 border-dashed rounded-lg flex items-center justify-center text-muted-foreground">
                      <Upload className="h-6 w-6" />
                    </div>
                  )}
                  <div>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        const input = document.createElement("input")
                        input.type = "file"
                        input.accept = "image/png,image/jpeg,image/webp"
                        input.onchange = (e) => {
                          const file = (e.target as HTMLInputElement).files?.[0]
                          if (!file) return
                          if (file.size > 500 * 1024) {
                            toast.error("Le logo ne doit pas dépasser 500 Ko")
                            return
                          }
                          const reader = new FileReader()
                          reader.onload = () => {
                            const base64 = (reader.result as string).split(",")[1]
                            settingsForm.setValue("logo_base64", base64)
                          }
                          reader.readAsDataURL(file)
                        }
                        input.click()
                      }}
                    >
                      <Upload className="h-4 w-4 mr-2" />
                      Choisir un logo
                    </Button>
                    <p className="text-xs text-muted-foreground mt-1">PNG, JPEG ou WebP, max 500 Ko</p>
                  </div>
                </div>
              </div>
              <div className="space-y-2">
                <Label htmlFor="doc_primary_color">Couleur principale des documents</Label>
                <div className="flex items-center gap-3">
                  <input
                    type="color"
                    id="doc_primary_color"
                    value={settingsForm.watch("doc_primary_color") || "#2563eb"}
                    onChange={(e) => settingsForm.setValue("doc_primary_color", e.target.value)}
                    className="h-10 w-14 cursor-pointer rounded border p-1"
                  />
                  <Input
                    value={settingsForm.watch("doc_primary_color") || "#2563eb"}
                    onChange={(e) => settingsForm.setValue("doc_primary_color", e.target.value)}
                    placeholder="#2563eb"
                    className="max-w-[140px]"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => settingsForm.setValue("doc_primary_color", null)}
                  >
                    Réinitialiser
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">Utilisée pour les en-têtes et titres des factures PDF</p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Aperçu</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid gap-6 md:grid-cols-2">
                <div>
                  <p className="text-sm font-medium mb-2">Ticket de caisse (thermique)</p>
                  <div className="p-4 bg-white dark:bg-zinc-950 rounded-lg border border-dashed">
                    <div className="font-mono text-sm text-black dark:text-zinc-200">
                      {settingsForm.watch("logo_base64") && (
                        <div className="flex justify-center mb-2">
                          <img src={`data:image/png;base64,${settingsForm.watch("logo_base64")}`} alt="" className="h-10 object-contain" />
                        </div>
                      )}
                      <div className="text-center font-bold">{settingsForm.watch("shop_name") || "SuperCaisse"}</div>
                      <div className="text-center text-xs">{settingsForm.watch("shop_address") || "Adresse du magasin"}</div>
                      <div className="text-center text-xs">{settingsForm.watch("shop_phone") || "Téléphone"}</div>
                      {settingsForm.watch("receipt_header") && (
                        <div className="text-center text-xs mt-1 italic">{settingsForm.watch("receipt_header")}</div>
                      )}
                      <div className="my-2 border-t border-dashed border-gray-400" />
                      <div className="text-xs">Article exemple × 2 ........ 20.00 MAD</div>
                      <div className="my-2 border-t border-dashed border-gray-400" />
                      <div className="text-center font-bold">Total: 20.00 MAD</div>
                      <div className="my-2 border-t border-dashed border-gray-400" />
                      <div className="text-center text-xs mt-2">{settingsForm.watch("receipt_footer") || "Merci de votre visite"}</div>
                    </div>
                  </div>
                </div>
                <div>
                  <p className="text-sm font-medium mb-2">Document PDF (A4)</p>
                  <div className="p-4 bg-white dark:bg-zinc-950 rounded-lg border border-dashed text-sm text-black dark:text-zinc-200">
                    <div className="flex justify-between items-start mb-3">
                      <div className="flex items-center gap-2">
                        {settingsForm.watch("logo_base64") && (
                          <img src={`data:image/png;base64,${settingsForm.watch("logo_base64")}`} alt="" className="h-8 object-contain" />
                        )}
                        <span className="font-bold">{settingsForm.watch("shop_name") || "SuperCaisse"}</span>
                      </div>
                      <div className="text-right">
                        <div className="font-bold" style={{ color: settingsForm.watch("doc_primary_color") || "#2563eb" }}>FACTURE</div>
                        <div className="text-xs text-muted-foreground">N° FA-2024-001</div>
                      </div>
                    </div>
                    <div className="h-px mb-2" style={{ backgroundColor: settingsForm.watch("doc_primary_color") || "#2563eb" }} />
                    <div className="text-xs text-muted-foreground">Aperçu simplifié du document A4</div>
                  </div>
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

        <TabsContent value="system" className="space-y-6">
          <MisesAJour />
          <form onSubmit={settingsForm.handleSubmit(handleSettingsSubmit, signalerChampsInvalides)} className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Shield className="h-5 w-5" />
                Sécurité & Sessions
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="idle_timeout">Délai de verrouillage automatique (secondes)</Label>
                <p className="text-sm text-muted-foreground">L'application se verrouille après cette période d'inactivité. 0 = désactivé.</p>
                <Input
                  type="number"
                  min="0"
                  step="30"
                  {...settingsForm.register("idle_timeout")}
                  id="idle_timeout"
                  className="max-w-[200px]"
                  placeholder="300"
                />
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

          <div className="flex justify-end">
            <Button type="submit" disabled={updateSettingsMutation.isPending}>
              {updateSettingsMutation.isPending ? (
                <><Loader2 className="h-4 w-4 animate-spin mr-2" />Enregistrement...</>
              ) : (
                "Enregistrer"
              )}
            </Button>
          </div>
          </form>
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
                  <p className="font-medium">Créer une sauvegarde</p>
                  <p className="text-sm text-muted-foreground">Copie immédiate dans le dossier des sauvegardes (une sauvegarde automatique est faite chaque jour au démarrage, 30 conservées)</p>
                </div>
                <Button variant="outline" onClick={() => backupMutation.mutate()} disabled={backupMutation.isPending}>
                  {backupMutation.isPending ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Database className="h-4 w-4 mr-2" />}
                  Sauvegarder
                </Button>
              </div>
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Exporter la base de données</p>
                  <p className="text-sm text-muted-foreground">Copie complète de la base SQLite dans Documents/SuperCaisse/exports</p>
                </div>
                <Button onClick={() => exportMutation.mutate()} disabled={exportMutation.isPending}>
                  {exportMutation.isPending ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Download className="h-4 w-4 mr-2" />}
                  Exporter
                </Button>
              </div>
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">Importer une sauvegarde</p>
                  <p className="text-sm text-muted-foreground text-destructive">⚠️ Remplace toutes les données actuelles</p>
                </div>
                <Button variant="destructive" onClick={() => setShowImportConfirm(true)}>
                  Importer
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="cloud" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Cloud className="h-5 w-5" />
                Synchronisation cloud
              </CardTitle>
              <CardDescription>
                Connectez cette caisse au compte cloud de votre entreprise pour synchroniser ventes, stock et clients.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {etatSynchroQuery.isLoading ? (
                <p className="text-sm text-muted-foreground">Chargement…</p>
              ) : etatSynchroQuery.data?.connecte ? (
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <p className="font-medium flex items-center gap-2">
                      <Cloud className="h-4 w-4 text-green-600" />
                      Connectée
                    </p>
                    <p className="text-sm text-muted-foreground">
                      Boutique cloud : {etatSynchroQuery.data.cloud_magasin_id}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {etatSynchroQuery.data.en_attente} modification(s) en attente d'envoi
                    </p>
                  </div>
                  <Button variant="destructive" onClick={() => setShowDesappairageConfirm(true)}>
                    <CloudOff className="h-4 w-4 mr-2" />
                    Déconnecter
                  </Button>
                </div>
              ) : appairageChoix ? (
                <div className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    Associez chaque boutique locale à sa boutique cloud correspondante.
                  </p>
                  {appairageChoix.magasins_locaux.length === 0 ? (
                    <p className="text-sm text-muted-foreground">Toutes les boutiques locales sont déjà associées.</p>
                  ) : (
                    appairageChoix.magasins_locaux.map((magasin) => (
                      <div key={magasin.id} className="flex items-center justify-between gap-4">
                        <Label className="min-w-0 flex-1 truncate">{magasin.nom}</Label>
                        <Select
                          value={associationsChoisies[magasin.id] ?? ""}
                          onValueChange={(v) => setAssociationsChoisies((a) => ({ ...a, [magasin.id]: v }))}
                        >
                          <SelectTrigger className="w-64">
                            <SelectValue placeholder="Choisir la boutique cloud" />
                          </SelectTrigger>
                          <SelectContent>
                            {appairageChoix.magasins_cloud.map((mc) => (
                              <SelectItem key={mc.cloud_magasin_id} value={mc.cloud_magasin_id}>
                                {mc.nom}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                    ))
                  )}
                  <div className="flex justify-end gap-2">
                    <Button
                      variant="outline"
                      onClick={() => annulerAppairageMutation.mutate()}
                      disabled={annulerAppairageMutation.isPending}
                    >
                      Annuler
                    </Button>
                    <Button
                      onClick={() => finaliserAppairageMutation.mutate()}
                      disabled={
                        finaliserAppairageMutation.isPending ||
                        appairageChoix.magasins_locaux.some((m) => !associationsChoisies[m.id])
                      }
                    >
                      {finaliserAppairageMutation.isPending ? (
                        <Loader2 className="h-4 w-4 animate-spin mr-2" />
                      ) : (
                        <Cloud className="h-4 w-4 mr-2" />
                      )}
                      Confirmer la connexion
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="space-y-4 max-w-sm">
                  <div className="space-y-2">
                    <Label htmlFor="cloud_email">Email administrateur</Label>
                    <Input
                      id="cloud_email"
                      type="email"
                      value={cloudEmail}
                      onChange={(e) => setCloudEmail(e.target.value)}
                      placeholder="admin@entreprise.com"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="cloud_password">Mot de passe</Label>
                    <Input
                      id="cloud_password"
                      type="password"
                      value={cloudMotDePasse}
                      onChange={(e) => setCloudMotDePasse(e.target.value)}
                    />
                  </div>
                  <Button
                    onClick={() => appairageMutation.mutate()}
                    disabled={!cloudEmail.trim() || !cloudMotDePasse || appairageMutation.isPending}
                  >
                    {appairageMutation.isPending ? (
                      <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    ) : (
                      <Cloud className="h-4 w-4 mr-2" />
                    )}
                    Se connecter
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <Dialog open={showDesappairageConfirm} onOpenChange={setShowDesappairageConfirm}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="h-5 w-5" />
              Déconnecter la synchronisation cloud
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Cette caisse arrêtera d'envoyer ses données au cloud. Les modifications déjà envoyées ne sont pas affectées.
          </p>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={() => setShowDesappairageConfirm(false)}>Annuler</Button>
            <Button
              variant="destructive"
              onClick={() => {
                desappairerMutation.mutate()
                setShowDesappairageConfirm(false)
              }}
              disabled={desappairerMutation.isPending}
            >
              {desappairerMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <CloudOff className="h-4 w-4 mr-2" />
              )}
              Déconnecter
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={showImportConfirm} onOpenChange={(open) => { if (!importMutation.isPending) setShowImportConfirm(open) }}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="h-5 w-5" />
              Restaurer une sauvegarde
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Toutes les données actuelles seront remplacées par le contenu du fichier. Une copie de la base actuelle est faite
            automatiquement avant la restauration, et tous les utilisateurs devront se reconnecter.
          </p>
          <div className="min-w-0 space-y-2">
            <Label>Sauvegardes disponibles</Label>
            <div className="max-h-48 overflow-y-auto rounded-md border divide-y">
              {sauvegardesLoading ? (
                <p className="p-3 text-sm text-muted-foreground">Chargement…</p>
              ) : sauvegardes.length === 0 ? (
                <p className="p-3 text-sm text-muted-foreground">Aucune sauvegarde trouvée</p>
              ) : (
                sauvegardes.map((s) => (
                  <button
                    key={s.chemin}
                    type="button"
                    title={s.chemin}
                    onClick={() => setImportPath(s.chemin)}
                    className={`w-full px-3 py-2 text-left text-sm hover:bg-muted ${importPath === s.chemin ? "bg-muted font-medium" : ""}`}
                  >
                    <span className="block truncate">{s.nom}</span>
                    <span className="text-xs text-muted-foreground">{s.date ?? "—"} · {(s.taille / 1024).toFixed(0)} Ko</span>
                  </button>
                ))
              )}
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="import_path">Ou chemin d'un autre fichier (.db)</Label>
            <Input
              id="import_path"
              value={importPath}
              onChange={(e) => setImportPath(e.target.value)}
              placeholder="Documents/SuperCaisse/exports/supercaisse_export_20260101_120000.db"
            />
          </div>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={() => setShowImportConfirm(false)} disabled={importMutation.isPending}>Annuler</Button>
            <Button
              variant="destructive"
              onClick={() => importMutation.mutate(importPath.trim())}
              disabled={!importPath.trim() || importMutation.isPending}
            >
              {importMutation.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Upload className="h-4 w-4 mr-2" />}
              Restaurer
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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