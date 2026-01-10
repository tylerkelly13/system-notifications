# system-notifier

A Rust-based Debian package

## Building

Build the project:

```bash
cargo build --release
```

## Testing

Run tests:

```bash
cargo test
```

## Building Debian Package

Build the Debian package:

```bash
dpkg-buildpackage -us -uc -b
```

The generated `.deb` file will be in the parent directory.

## Installing

Install the package:

```bash
sudo dpkg -i ../system-notifier_*.deb
```

## License

MIT
