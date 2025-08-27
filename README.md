# raur
A simple AUR helper written in rust.

## Usage
```bash
raur [OPTIONS] <COMMAND>
```

As of now, 7 commands are available:

| Command     | Description                            |
|-------------|----------------------------------------|
| `search`    | Searches for AUR package(s)            |
| `install`   | Installs an AUR package                |
| `update`    | Updates an installed AUR package       |
| `info`      | Shows information about an AUR package |
| `clean`     | Cleans build directories               |
| `uninstall` | Uninstalls an installed AUR package    |
| `help`      | Provides help on how to use this tool  |

The only available flag is `--github` which uses the github mirror for package installation.

## Building

To build this project, run:

```bash
cargo install path ./the/path/to/project's/root
```

Or, using the github aur mirror (recommended, fetches all dependencies automatically):
```bash
 git clone --single-branch --branch raur-helper-git https://github.com/archlinux/aur.git raur-helper-git
 cd raur-helper-git
 makepkg -si
```
