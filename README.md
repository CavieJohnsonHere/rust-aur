# raur
A simple AUR helper written in rust.

## Usage
```bash
raur [OPTIONS] <COMMAND>
```

As of now, 7 commands are available:

| Command     | Description                            |
|-------------|----------------------------------------|
| `search`    | Searches for AUR package(s             |
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

The binaries are available inside `./target/release`. You may have to add `~/.cargo/bin` to your PATH to run this program from everywhere.
