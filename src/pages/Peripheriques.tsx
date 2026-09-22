import { useState } from "react"
import { Card, CardContent } from "@/ui/Card"
import { Button } from "@/ui/Button"
import { Badge } from "@/ui/Badge"
import { Input } from "@/ui/Input"
import { Label } from "@/ui/Label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/Select"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/ui/Tabs"
import { Monitor, Scale, CreditCard, Wifi, WifiOff, RefreshCw, Settings } from "lucide-react"
import PageHeader from "@/components/PageHeader"

interface Peripherique {
  type: string
  nom: string
  port: string
  statut: "connecte" | "deconnecte" | "erreur"
}

export default function Peripheriques() {
  const [afficheurPort, setAfficheurPort] = useState("COM3")
  const [afficheurBaudRate, setAfficheurBaudRate] = useState("9600")
  const [balancePort, setBalancePort] = useState("COM4")
  const [balanceProtocol, setBalanceProtocol] = useState("toledo")
  const [tpePort, setTpePort] = useState("COM5")
  const [tpeProtocol, setTpeProtocol] = useState("concert")

  const peripheriques: Peripherique[] = [
    { type: "afficheur", nom: "Afficheur client VFD", port: afficheurPort, statut: "deconnecte" },
    { type: "balance", nom: "Balance connectée", port: balancePort, statut: "deconnecte" },
    { type: "tpe", nom: "Terminal de paiement", port: tpePort, statut: "deconnecte" },
  ]

  const getStatutBadge = (statut: Peripherique["statut"]) => {
    switch (statut) {
      case "connecte": return <Badge className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"><Wifi className="h-3 w-3 mr-1" /> Connecté</Badge>
      case "deconnecte": return <Badge variant="secondary"><WifiOff className="h-3 w-3 mr-1" /> Déconnecté</Badge>
      case "erreur": return <Badge variant="destructive">Erreur</Badge>
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Périphériques" description="Écran client, balance, terminal de paiement" />

      <div className="grid gap-4 md:grid-cols-3">
        {peripheriques.map((p) => (
          <Card key={p.type}>
            <CardContent className="pt-6">
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-2">
                  {p.type === "afficheur" && <Monitor className="h-5 w-5 text-blue-500" />}
                  {p.type === "balance" && <Scale className="h-5 w-5 text-green-500" />}
                  {p.type === "tpe" && <CreditCard className="h-5 w-5 text-purple-500" />}
                  <span className="font-medium">{p.nom}</span>
                </div>
                {getStatutBadge(p.statut)}
              </div>
              <p className="text-sm text-muted-foreground">Port : {p.port}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <Tabs defaultValue="afficheur">
        <TabsList>
          <TabsTrigger value="afficheur">
            <Monitor className="h-4 w-4 mr-2" />
            Afficheur client
          </TabsTrigger>
          <TabsTrigger value="balance">
            <Scale className="h-4 w-4 mr-2" />
            Balance
          </TabsTrigger>
          <TabsTrigger value="tpe">
            <CreditCard className="h-4 w-4 mr-2" />
            TPE
          </TabsTrigger>
        </TabsList>

        <TabsContent value="afficheur" className="mt-4">
          <Card>
            <CardContent className="pt-6">
              <h3 className="font-semibold mb-4 flex items-center gap-2">
                <Monitor className="h-5 w-5" />
                Écran client secondaire (double afficheur)
              </h3>
              <p className="text-sm text-muted-foreground mb-6">
                Configurez un afficheur VFD ou un second écran pour afficher les articles scannés, le total et les informations promotionnelles au client.
              </p>
              <div className="grid gap-4 md:grid-cols-2 max-w-lg">
                <div className="space-y-2">
                  <Label>Port série</Label>
                  <Input value={afficheurPort} onChange={(e) => setAfficheurPort(e.target.value)} placeholder="COM3" />
                </div>
                <div className="space-y-2">
                  <Label>Vitesse (baud rate)</Label>
                  <Select value={afficheurBaudRate} onValueChange={setAfficheurBaudRate}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="2400">2400</SelectItem>
                      <SelectItem value="4800">4800</SelectItem>
                      <SelectItem value="9600">9600</SelectItem>
                      <SelectItem value="19200">19200</SelectItem>
                      <SelectItem value="38400">38400</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="flex gap-2 mt-6">
                <Button variant="outline">
                  <RefreshCw className="h-4 w-4 mr-2" />
                  Tester la connexion
                </Button>
                <Button>
                  <Settings className="h-4 w-4 mr-2" />
                  Enregistrer
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="balance" className="mt-4">
          <Card>
            <CardContent className="pt-6">
              <h3 className="font-semibold mb-4 flex items-center gap-2">
                <Scale className="h-5 w-5" />
                Balance connectée
              </h3>
              <p className="text-sm text-muted-foreground mb-6">
                Connectez une ou deux balances pour peser les produits au poids directement depuis le POS. Compatible protocoles Toledo, CAS, DIGI.
              </p>
              <div className="grid gap-4 md:grid-cols-2 max-w-lg">
                <div className="space-y-2">
                  <Label>Port série</Label>
                  <Input value={balancePort} onChange={(e) => setBalancePort(e.target.value)} placeholder="COM4" />
                </div>
                <div className="space-y-2">
                  <Label>Protocole</Label>
                  <Select value={balanceProtocol} onValueChange={setBalanceProtocol}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="toledo">Toledo</SelectItem>
                      <SelectItem value="cas">CAS</SelectItem>
                      <SelectItem value="digi">DIGI</SelectItem>
                      <SelectItem value="custom">Personnalisé</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="rounded-lg bg-muted p-4 mt-4 max-w-lg">
                <div className="text-sm font-medium mb-2">Dernière pesée</div>
                <div className="text-3xl font-bold text-muted-foreground">— kg</div>
                <p className="text-xs text-muted-foreground mt-1">Connectez une balance pour afficher le poids en temps réel</p>
              </div>
              <div className="flex gap-2 mt-6">
                <Button variant="outline">
                  <RefreshCw className="h-4 w-4 mr-2" />
                  Tester
                </Button>
                <Button>Enregistrer</Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="tpe" className="mt-4">
          <Card>
            <CardContent className="pt-6">
              <h3 className="font-semibold mb-4 flex items-center gap-2">
                <CreditCard className="h-5 w-5" />
                Terminal de paiement (TPE) intégré
              </h3>
              <p className="text-sm text-muted-foreground mb-6">
                Intégrez votre terminal de paiement électronique pour envoyer automatiquement le montant à encaisser. Compatible protocoles CONCERT V3, Ingenico, Verifone.
              </p>
              <div className="grid gap-4 md:grid-cols-2 max-w-lg">
                <div className="space-y-2">
                  <Label>Port série / IP</Label>
                  <Input value={tpePort} onChange={(e) => setTpePort(e.target.value)} placeholder="COM5 ou 192.168.1.100" />
                </div>
                <div className="space-y-2">
                  <Label>Protocole</Label>
                  <Select value={tpeProtocol} onValueChange={setTpeProtocol}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="concert">CONCERT V3</SelectItem>
                      <SelectItem value="ingenico">Ingenico TelePAY</SelectItem>
                      <SelectItem value="verifone">Verifone</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="rounded-lg bg-muted p-4 mt-4 max-w-lg">
                <div className="text-sm font-medium mb-2">Statut TPE</div>
                <div className="flex items-center gap-2 text-muted-foreground">
                  <WifiOff className="h-5 w-5" />
                  <span>Non connecté — saisie manuelle active</span>
                </div>
              </div>
              <div className="flex gap-2 mt-6">
                <Button variant="outline">
                  <RefreshCw className="h-4 w-4 mr-2" />
                  Tester
                </Button>
                <Button>Enregistrer</Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
