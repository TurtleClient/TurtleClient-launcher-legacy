# Turtle Launcher — Native Alpha 0.1.0

Windows x64 launcher for the native TurtleClient, based on Horizon / Minecraft 26.3.
This ports the Svelte / Tauri architecture and branding of TurtleClient's previous launcher.

## Starten

`TurtleLauncher-0.1.0-setup.exe` installieren oder das portable ZIP vollständig entpacken
und `TurtleLauncher.exe` starten. Der Ordner `payload` muss neben dem portablen Programm bleiben.

1. Unter **Konto** mit dem Microsoft-Konto anmelden, das Minecraft: Java Edition besitzt.
2. Unter **Einrichtung** eine vorhandene 26.3-Installation nutzen. Optional ihren Minecraft-Ordner
   angeben; alternativ nach der Anmeldung **Von Mojang herunterladen** wählen.
3. Unten über **Spielen** starten. Ocean-Rest startet dabei automatisch auf `127.0.0.1:3000`.

Der erste Ressourcenaufbau kann einige Minuten benötigen. Microsoft-Anmeldung, Besitzprüfung,
Vorbereitung und Session-Übergabe erledigt das mitgelieferte Horizon-Backend.
Für den Launcher muss weder Rust noch IntelliJ noch ein separat installierter Horizon Launcher vorhanden sein.
Windows WebView2 wird benötigt; der Installer lädt es bei Bedarf von Microsoft.
Die Ressourcenaufbereitung benötigt gegebenenfalls eine von Horizon verwaltete Java-Laufzeit.
Das Spiel selbst ist nativ und lädt keine Fabric-JARs.

**B**: Emote-Rad halten und loslassen. **C**: Zoom halten, Mausrad für Zoomstufe.
Direktbefehl: `/emote wave`; weitere: `dance`, `point`, `clap`, `cheer`, `laugh`, `stop`, `list`.

Daten liegen unter `%LOCALAPPDATA%/TurtleClient/`: `Launcher/`, `Game/`, `Ocean-Rest/`, `logs/`.
`TURTLE_LAUNCHER_HOME` überschreibt den Hauptordner. Vorhandene Minecraft- und Horizon-Welten
werden nicht verschoben. Beende das laufende Spiel im Spielmenü, damit die Welt gespeichert wird.
Deutsch und Englisch sind unter **Einstellungen → Sprache** umschaltbar.

Die Oberfläche bietet eine Übersicht, Profilverwaltung, Module, Aussehen, Konten, Einrichtung und
Aktivität. Neue Profile erhalten getrennte Spielordner. Zoom, Tastenbelegung, fließender Zoom,
Emotes und Capes werden pro Profil in `config/turtleclient.json` gespeichert. Änderungen gelten
beim nächsten Spielstart; während des Spiels sind diese Einstellungen gesperrt.
Unter **Aussehen** lässt sich ein PNG-Cape (64 × 32 Pixel, maximal 2 MB) für das ausgewählte
Konto in den lokalen Cape-Dienst laden. Die 3D-Vorschau lädt den Skin und das offizielle Cape des ausgewählten Minecraft-Kontos über
Mojangs öffentlichen Profildienst; ein lokales Turtle-Cape hat Vorrang. Sie unterstützt Classic-/Slim-
Arme, Skin-Ebenen, Drehen per Maus oder Pfeiltasten sowie Blickbewegungen. **Pos1** setzt die Kamera
zurück. **Skin aktualisieren** lädt Änderungen erneut; bereits geladene Skins bleiben offline verfügbar.
Ohne Anmeldung oder erreichbaren Skin wird ein neutrales Modell gezeigt. Die Emotes in der
Launcher-Vorschau sind Vorschauanimationen. Reduzierte Bewegung deaktiviert automatische Animationen.
Der Renderer [skinview3d](https://github.com/bs-community/skinview3d) nutzt Three.js; die MIT-Lizenzen
sind unter `public/licenses/` enthalten.
Akzentfarbe, reduzierte Bewegung und das Fensterverhalten werden lokal im Launcher gespeichert.

Für die Entwicklung im Launcher-Verzeichnis:

```powershell
npm install
npm run tauri dev
```

Dafür muss `src-tauri/payload` zuvor mit dem unten beschriebenen Build-Skript vorbereitet sein.

## Stand

Dies ist eine erste Alpha für **26.3**. Ältere Serverversionen folgen später.
Emotes sind lokal; eine gemeinsame Emote-/Cape-Plattform und weitere Module stehen in der Roadmap.
Der lokale Cape-Dienst bleibt für weitere Spielstarts aktiv und kann mit
`payload/Stop-OceanRest.ps1` beendet werden (PowerShell 7).

Die Windows-Dateien sind nicht mit einem kommerziellen Codesignatur-Zertifikat signiert.
Die SHA-256-Datei gehört zum Download. Es werden keine Minecraft-JARs, Mojang-Assets oder Kontodaten mitgeliefert.

## Bauen

Repositories:

- [Nativer Client](https://github.com/TurtleClient/TurtleClient-Mod/tree/codex/rust-26.3), Branch `codex/rust-26.3`.
- [Nativer Launcher](https://github.com/TurtleClient/TurtleClient-launcher/tree/codex/native-launcher-26.3), Branch `codex/native-launcher-26.3`.

Aus dem separaten Launcher-Checkout dieses Skript mit dem Pfad zum nativen Client ausführen:

```powershell
./Build-Native.ps1 -ClientRoot 'C:\Projekte\TurtleClient-Mod'
```

Das Skript baut die Quellen dieses Launcher-Checkouts. Das Client-Repository liefert den Spiel-Build,
die lokal vorbereiteten Hilfsprogramme und die Paketierung. Die Ausgabe steht im `dist/`-Ordner
des Client-Checkouts. Binärdateien und Minecraft-Ressourcen bleiben außerhalb von Git.

Alternativ aus dem Client-Quellbaum mit der darin enthaltenen Launcher-Kopie:

```powershell
./tools/Build-TurtleLauncher.ps1
```

Voraussetzungen: Rust stable MSVC, Visual Studio C++ Build Tools, Node.js / npm und PowerShell 7.
Das Skript übernimmt native Programme und die redistributierbaren Hilfsdateien aus der angegebenen
Horizon-Version; alternative Pfade sind über `-HorizonLauncher` und `-HorizonVersion` möglich.
Quellstand und Herkunft: `UPSTREAM.md`. Lizenz: GPL-3.0-or-later für den Launcher;
Horizon und die übrigen Pakete behalten ihre eigenen Lizenzen.
