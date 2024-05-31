# MCSDK - mc-server-dev-kit

A Minecraft Server wrapper that makes it easier testing Spigot/Paper Server Plugins on your local Machine.

![Rust Badge](https://img.shields.io/badge/Rust-000?logo=rust&logoColor=fff&style=for-the-badge)

![screenshot of mcsdk](https://raw.githubusercontent.com/Noriskky/mc-server-dev-kit/main/screenshots/screenshot.png)

## Installation

You can install MCSDK via Cargo:

```bash
cargo install mcsdk
```

Make Sure to add the ``.cargo/bin`` folder in your path with you can put the following in your ``.bashrc`` or ``.zshrc``: <br>
```bash
export PATH="$PATH:$HOME/.cargo/bin"
```

## Usage

When Installed, it can be executed with the following:
```bash
mcsdk
```

### Options

- `-h, --help`:     Print help
- `-V, --version`:  Print version

### Subcommands

- ``start``: Start a Local Test Server
- ``help``: Print Help message

### `Start` Options:

**Arguments**: <br>
- `<SOFTWARE>`:    Define what Server Software should be used `[possible values: paper]` <br>
- `<VERSION>  `:   Which Minecraft Version should be started <br>
- `[PLUGINS]...`:  Path to Plugin jars to put into the plugins Folder <br>

**Options**: <br>
- `-w, --working-directory <WORKING_DIRECTORY>`: Where the server should be stored `[default: none]`  <br>
- `-a, --args <ARGS>`: Arguments to give the server  <br>
- `-m, --mem <MEM>`: How much Ram is the server allowed to use `[default: 2048]`  <br>
- `-h, --help`: Print help  <br>
- `-g, --gui`: If used the server Gui will start too <br>
- `-p, --port`: Which Port to bind for the Server `[default: 25565]` <br>

## Support

It should be usable under Windows and MacOS but it only got tested under Linux.
It would be appreciated that if you have any Issues to open a Issue. Thx u

## How to contribute

Hey if you want to contribute your are welcome to do so.
Testing can be done with the following:
```bash
# For make users
make release
./target/debug/mcsdk

# For just users (recommended)
make release
./target/debug/mcsdk
```

or you could also install it but with the following:
```bash
# For make users
make install
mcsdk

# For just users
just install
mcsdk
```

> [!NOTE]
> This is not recommended!

## Build from Source

To do build from source use the following command:
```bash
# For make users
make release

# For just users
just release
```

The binary will be found under `target/debug`

Or if you want to directly install it you can use the following command:
```bash
# For make users
make install

# For just users
just install
```
