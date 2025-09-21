armake2 - Enhanced Fork
========================

This is an enhanced fork of [KoffeinFlummi's armake2](https://github.com/KoffeinFlummi/armake2), bringing the project up to date with modern Rust standards and completing previously unimplemented features.

## What's New in This Fork

### ✨ Major Improvements

- **Fully Implemented PAA Conversion** - `paa2img` and `img2paa` commands now work
  - Proper handling of DXT1/DXT5 compression
  - Correct dimension parsing
  - LZO compression support

- **Modernized to Rust Edition 2024**
  - Updated all deprecated patterns and syntax
  - Thread-safe implementation replacing unsafe static mutables
  - Zero compilation warnings

- **No External Dependencies Required**
  - OpenSSL is vendored (compiled into the binary)
  - Pre-compiled binaries work out-of-the-box on Windows and Linux
  - No need to install or configure OpenSSL separately

- **Fully Implemented CLI Flags**
  - `--force` flag now works for all commands that create files
  - Proper file overwrite protection

## Download

Check the [Releases](../../releases) page for pre-compiled binaries:
- `armake2` - Linux x86_64 (standalone, no dependencies)
- `armake2.exe` - Windows x86_64 (standalone, no dependencies)

## Building from Source

Requirements:
- Rust 1.87.0 or later
- Cargo (Rust's package manager)

### Linux
```bash
cargo build --release
```

### Windows Cross-Compilation from Linux
```bash
# Install MinGW toolchain
sudo apt-get install mingw-w64

# Add Windows target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

The binaries will be in `target/release/` or `target/x86_64-pc-windows-gnu/release/`.

## Usage

```
armake2

Usage:
    armake2 rapify [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 preprocess [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 derapify [-v] [-f] [-d <indentation>] [<source> [<target>]]
    armake2 binarize [-v] [-f] [-w <wname>]... <source> <target>
    armake2 build [-v] [-f] [-w <wname>]... [-i <includefolder>]... [-x <excludepattern>]... [-e <headerext>]... [-k <privatekey>] [-s <signature>] <sourcefolder> [<target>]
    armake2 pack [-v] [-f] [-x <excludepattern>]... [-e <headerext>]... [-k <privatekey>] [-s <signature>] <sourcefolder> [<target>]
    armake2 inspect [-v] [<source>]
    armake2 unpack [-v] [-f] <source> <targetfolder>
    armake2 cat [-v] <source> <filename> [<target>]
    armake2 keygen [-v] [-f] <keyname>
    armake2 sign [-v] [-f] [--v2] <privatekey> <pbo> [<signature>]
    armake2 verify [-v] <publickey> <pbo> [<signature>]
    armake2 paa2img [-v] [-f] <source> <target>
    armake2 img2paa [-v] [-f] [-z] [-t <paatype>] <source> <target>
    armake2 (-h | --help)
    armake2 --version

Commands:
    rapify      Preprocess and rapify a config file
    preprocess  Preprocess a file
    derapify    Derapify a config
    binarize    Binarize a file using BI's binarize.exe (Windows only)
    build       Build a PBO from a folder
    pack        Pack a folder into a PBO without binarization/rapification
    inspect     Inspect a PBO and list contained files
    unpack      Unpack a PBO into a folder
    cat         Read a file from a PBO to stdout
    keygen      Generate a signing keypair
    sign        Sign a PBO with a private key
    verify      Verify a PBO's signature
    paa2img     Convert PAA to PNG image
    img2paa     Convert image to PAA format

Options:
    -v --verbose    Enable verbose output
    -f --force      Overwrite existing files
    -w --warning    Disable specific warning
    -i --include    Add include folder for preprocessing
    -x --exclude    Exclude files matching pattern
    -e --headerext  Add PBO header extension
    -k --key        Private key for signing
    -s --signature  Custom signature path
    -z --compress   Enable LZO compression (img2paa)
    -t --type       PAA type: DXT1 or DXT5 (img2paa)
    --v2            Use v2 signatures (sign)
```

### PAA Conversion Examples

Convert PAA to PNG:
```bash
armake2 paa2img texture.paa texture.png
```

Convert PNG to PAA with DXT5 (default, with alpha):
```bash
armake2 img2paa image.png texture.paa
```

Convert PNG to PAA with DXT1 (no alpha, smaller file):
```bash
armake2 img2paa -t DXT1 image.png texture.paa
```

Convert with LZO compression for smaller file size:
```bash
armake2 img2paa -z image.png texture.paa
```

### PBO Operations Examples

Build a PBO:
```bash
armake2 build -f mymod.p3d mymod.pbo
```

Build and sign a PBO:
```bash
armake2 keygen mykey
armake2 build -k mykey.biprivatekey mission.sqm mission.pbo
```

Unpack a PBO:
```bash
armake2 unpack mission.pbo mission_folder/
```

## Technical Details

### PAA Format Support
- **DXT1**: RGB compression, no alpha channel, 4:1 compression ratio
- **DXT5**: RGBA compression, with alpha channel, 4:1 compression ratio
- **LZO**: Additional compression layer for smaller file sizes
- Automatic mipmap generation
- Proper handling of compression flags in PAA headers

### Changes from Original armake2

- PAA conversion fully implemented (was previously disabled)
- Updated from Rust 2018 to Rust 2024 edition
- Replaced unsafe static mutable warnings system with thread-safe implementation
- Fixed deprecated language patterns (`...` → `..=`, removed `ref` patterns)
- Vendored OpenSSL to eliminate external dependencies
- Implemented all documented but missing CLI functionality
- Fixed PAA dimension parsing (high bit is compression flag, not part of dimensions)

## Credits

- Original armake and armake2 by [KoffeinFlummi](https://github.com/KoffeinFlummi)
- PAA format implementation based on the original [armake](https://github.com/KoffeinFlummi/armake) C code
- This fork maintained to keep these essential Arma modding tools alive and modern

## License

GPL-2.0 or later (same as original armake2)

## Contributing

Issues and pull requests are welcome!
