# Chords

<img width="2080" height="1352" alt="Screenshot From 2026-03-18 08-19-59" src="https://github.com/user-attachments/assets/b6169067-9b24-419b-ab82-1e2a97309aae" />

A native GNOME guitar chords viewer. Browse, search, and save guitar chord sheets with a clean, performant interface.

Chords fetches tabs from [freetar.de](https://freetar.de) and renders them as plain monospace text — no web views, no bloat.

## Features

- **Search** — Find songs online via Ctrl+F
- **Library** — Save chords for offline access, filter with Ctrl+K
- **Transpose** — Shift chords up/down with correct voicings from a 12,000+ chord database
- **Auto-scroll** — Hands-free scrolling with adjustable speed (toggle with Space)
- **Chord diagrams** — Visual fingering charts for every chord in the song
- **Columns** — Split long sheets into 1-4 columns
- **Capo indicator** — Prominent display when a song requires a capo
- **Customizable** — Pick any monospace font, adjust size, choose highlight colors
- **Responsive** — Sidebar collapses on narrow windows

## Installation

- **Ubuntu/Debian** — download the `.deb` from [Releases](https://github.com/bjesus/chords/releases)
- **Arch Linux** — install `chords` from the AUR
- **Nix** — `nix run github:bjesus/chords`
- **AppImage** — download from [Releases](https://github.com/bjesus/chords/releases)
- **Windows** — download the `.zip` from [Releases](https://github.com/bjesus/chords/releases), extract, and run `chords.exe`

### Building from source

Requires GTK4, libadwaita, and Rust.

```sh
# Ubuntu/Debian
sudo apt install libgtk-4-dev libadwaita-1-dev

# Fedora
sudo dnf install gtk4-devel libadwaita-devel

# Arch
sudo pacman -S gtk4 libadwaita

# Build & run
cargo run
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+F` | Search online |
| `Ctrl+K` | Filter library |
| `Space` | Toggle auto-scroll |
| `Ctrl+Plus/Minus` | Zoom in/out |
| `Alt+Left` | Back to search results |
| `Ctrl+Q` | Quit |

## Credits

Chord database from [Fretboard](https://github.com/bragefuglseth/fretboard) by Brage Fuglseth (GPL-3.0).
Tab data from [freetar.de](https://freetar.de) by kmille.
Artist images from [Deezer API](https://developers.deezer.com/).

## License

GPL-3.0
