# Ether

WIP desktop streaming app built with `Tauri + Rust + React + TypeScript`.

## Current Status

- Work in progress.
- Currently supports anime streaming (movies/other media may be added later).

## Development

```bash
pnpm install
pnpm tauri dev
```

## Install

### Binary Release (Linux)

Download a prebuilt binary from:

https://github.com/TheZero0-ctrl/ether-stream/releases/latest

Manual install examples:

```bash
# AppImage
chmod +x Ether_*.AppImage
./Ether_*.AppImage

# Debian package
sudo dpkg -i Ether_*_amd64.deb
```

### Automated install/update (Linux x86_64)

Always inspect scripts before piping into shell.

```bash
curl -fsSL https://raw.githubusercontent.com/TheZero0-ctrl/ether-stream/main/scripts/install_update_linux.sh | bash
```

Default install location is `$HOME/.local/bin`.
Override it with `DIR`:

```bash
curl -fsSL https://raw.githubusercontent.com/TheZero0-ctrl/ether-stream/main/scripts/install_update_linux.sh | DIR=/usr/local/bin bash
```

Install a specific version by setting `VERSION` (example `v0.1.0`):

```bash
curl -fsSL https://raw.githubusercontent.com/TheZero0-ctrl/ether-stream/main/scripts/install_update_linux.sh | VERSION=v0.1.0 bash
```

## Disclaimer

This application is provided "as is" for educational and experimental use.

- The developer does not claim ownership of any third-party content.
- The developer does not host or distribute copyrighted media.
- The developer does not control or guarantee third-party providers.
- Users are responsible for complying with local laws and platform terms.
- Users are encouraged to support creators through legal channels.
