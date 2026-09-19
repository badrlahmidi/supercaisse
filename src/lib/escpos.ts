export class EscPosBuilder {
  private buffer: number[] = []

  constructor() {
    this.init()
  }

  // Initialiser l'imprimante (ESC @)
  init() {
    this.buffer.push(0x1B, 0x40)
    return this
  }

  // Alignement (ESC a n) - 0: Gauche, 1: Centre, 2: Droite
  align(align: "left" | "center" | "right") {
    const n = align === "left" ? 0 : align === "center" ? 1 : 2
    this.buffer.push(0x1B, 0x61, n)
    return this
  }

  // Mettre en gras (ESC E n)
  bold(on: boolean) {
    this.buffer.push(0x1B, 0x45, on ? 1 : 0)
    return this
  }

  // Taille du texte (GS ! n)
  size(width: number, height: number) {
    // width et height de 1 à 8 (0-7 pour n)
    const n = ((width - 1) << 4) | (height - 1)
    this.buffer.push(0x1D, 0x21, n)
    return this
  }

  // Ajouter du texte (encodage basique ASCII/CP858)
  text(str: string) {
    // Remplacement basique des accents pour l'encodage CP858 / CP852
    const cleanStr = str
      .replace(/é/g, "e")
      .replace(/è/g, "e")
      .replace(/ê/g, "e")
      .replace(/à/g, "a")
      .replace(/â/g, "a")
      .replace(/ô/g, "o")
      .replace(/ç/g, "c")
    
    for (let i = 0; i < cleanStr.length; i++) {
      this.buffer.push(cleanStr.charCodeAt(i) & 0xFF)
    }
    return this
  }

  // Ajouter une ligne avec un saut de ligne
  line(str: string = "") {
    this.text(str)
    this.buffer.push(0x0A) // LF
    return this
  }

  // Couper le papier (GS V m)
  cut() {
    this.buffer.push(0x0A, 0x0A, 0x0A) // 3 sauts de ligne avant la coupe
    this.buffer.push(0x1D, 0x56, 0x41, 0x00) // Partial cut
    return this
  }

  // Ouvrir le tiroir caisse
  openDrawer() {
    this.buffer.push(0x1B, 0x70, 0x00, 0x19, 0xFA)
    return this
  }

  // Générer le Base64
  toBase64(): string {
    const bytes = new Uint8Array(this.buffer)
    let binary = ""
    for (let i = 0; i < bytes.byteLength; i++) {
      binary += String.fromCharCode(bytes[i])
    }
    return btoa(binary)
  }
}
