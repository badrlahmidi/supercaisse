import { readFileSync, writeFileSync } from "node:fs"

const racine = new URL("../", import.meta.url)
const { version } = JSON.parse(readFileSync(new URL("package.json", racine), "utf8"))

function remplacer(chemin, motif, remplacement) {
  const fichier = new URL(chemin, racine)
  const contenu = readFileSync(fichier, "utf8")
  if (!motif.test(contenu)) throw new Error(`Version introuvable dans ${chemin}`)
  writeFileSync(fichier, contenu.replace(motif, remplacement))
}

remplacer("src-tauri/Cargo.toml", /^version = "[^"]+"/m, `version = "${version}"`)
remplacer("src-tauri/Cargo.lock", /(\[\[package\]\]\nname = "app"\nversion = )"[^"]+"/, `$1"${version}"`)
console.log(`Version ${version} reportée dans Cargo.toml et Cargo.lock`)
